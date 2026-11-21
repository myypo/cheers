//! Problems the debug runtime finds in a page, which it posts to the server.

use std::fmt;

use serde::Deserialize;

/// A problem the debug runtime found in a page.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[non_exhaustive]
pub struct Diagnostic {
    /// The path and query of the page that reported it.
    pub page: String,
    /// What went wrong.
    #[serde(flatten)]
    pub kind: DiagnosticKind,
}

/// What went wrong in a page, as [`Diagnostic::kind`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum DiagnosticKind {
    /// A Datastar action outside the form's `!on:submit` submits it, so pressing Enter or a
    /// submit button in it submits it natively and reloads the page.
    FormSubmittedOutsideOnSubmit {
        /// The form's opening tag.
        form: String,
    },
    /// An async stream arrived for a placeholder the page does not have.
    AsyncStreamTargetMissing {
        /// The stream's key.
        key: String,
    },
    /// A patch matched no element, and was not marked with `PatchElements::optional`.
    PatchTargetMissing {
        /// What the patch looked for.
        target: PatchTarget,
    },
    /// A patch without a selector carries an element without an id, so it has no target. Marking
    /// the patch with `PatchElements::optional` does not silence this.
    PatchElementWithoutId {
        /// The element's opening tag.
        element: String,
    },
    /// An error no script caught.
    UncaughtError {
        /// The error's message.
        message: String,
    },
    /// A promise rejected with no handler.
    UnhandledRejection {
        /// The rejection reason's message.
        message: String,
    },
    /// A report Cheers could not read, which is a bug in Cheers.
    #[cfg(feature = "test")]
    #[serde(skip)]
    UnreadableReport {
        /// The report as the server received it.
        report: String,
        /// Why it could not be read.
        error: String,
    },
}

/// What a patch that matched no element looked for, in [`DiagnosticKind::PatchTargetMissing`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PatchTarget {
    /// The id of an element the patch carries.
    Id(String),
    /// The patch's selector.
    Selector(String),
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.page, self.kind)
    }
}

impl fmt::Display for DiagnosticKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FormSubmittedOutsideOnSubmit { form } => write!(
                f,
                "this form is submitted by a Datastar action outside its `!on:submit`, so \
                 pressing Enter or a submit button in it submits it natively and reloads the \
                 page. Put the action on the form with `!on:submit(...)`, or give the form an \
                 `action` attribute if it should submit natively.\n  {form}"
            ),
            Self::AsyncStreamTargetMissing { key } => {
                write!(f, "async stream target missing for {key}")
            }
            Self::PatchTargetMissing { target } => write!(
                f,
                "a patch found no element {target} on the page. Mark the patch with \
                 `PatchElements::optional` if the element may be absent."
            ),
            Self::PatchElementWithoutId { element } => write!(
                f,
                "a patch without a selector carries an element without an id, so it has no \
                 target. Give the element an id or the patch a selector.\n  {element}"
            ),
            Self::UncaughtError { message } => write!(f, "uncaught error: {message}"),
            Self::UnhandledRejection { message } => {
                write!(f, "unhandled promise rejection: {message}")
            }
            #[cfg(feature = "test")]
            Self::UnreadableReport { report, error } => {
                write!(
                    f,
                    "a diagnostic report could not be read ({error}): {report}"
                )
            }
        }
    }
}

impl fmt::Display for PatchTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(id) => write!(f, "with id `{id}`"),
            Self::Selector(selector) => write!(f, "matching `{selector}`"),
        }
    }
}

/// Where the debug runtime posts each diagnostic.
pub(crate) const PATH: &str = "/cheers/diagnostics";

#[cfg(debug_assertions)]
pub(crate) fn router<S>() -> axum::Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    axum::Router::new().route(PATH, axum::routing::post(report))
}

#[cfg(debug_assertions)]
async fn report(axum::Json(diagnostic): axum::Json<Diagnostic>) -> axum::http::StatusCode {
    tracing::warn!("cheers diagnostic on {diagnostic}");
    axum::http::StatusCode::NO_CONTENT
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticKind, PatchTarget};

    #[test]
    fn parses_the_reports_the_runtime_posts() {
        for (report, kind) in [
            (
                r#"{"kind":"form_submitted_outside_on_submit","form":"<form>"}"#,
                DiagnosticKind::FormSubmittedOutsideOnSubmit {
                    form: "<form>".into(),
                },
            ),
            (
                r#"{"kind":"async_stream_target_missing","key":"s1"}"#,
                DiagnosticKind::AsyncStreamTargetMissing { key: "s1".into() },
            ),
            (
                r#"{"kind":"patch_target_missing","target":{"id":"note"}}"#,
                DiagnosticKind::PatchTargetMissing {
                    target: PatchTarget::Id("note".into()),
                },
            ),
            (
                r##"{"kind":"patch_target_missing","target":{"selector":"#a .b"}}"##,
                DiagnosticKind::PatchTargetMissing {
                    target: PatchTarget::Selector("#a .b".into()),
                },
            ),
            (
                r#"{"kind":"patch_element_without_id","element":"<p>"}"#,
                DiagnosticKind::PatchElementWithoutId {
                    element: "<p>".into(),
                },
            ),
            (
                r#"{"kind":"uncaught_error","message":"boom"}"#,
                DiagnosticKind::UncaughtError {
                    message: "boom".into(),
                },
            ),
            (
                r#"{"kind":"unhandled_rejection","message":"boom"}"#,
                DiagnosticKind::UnhandledRejection {
                    message: "boom".into(),
                },
            ),
        ] {
            let report = format!(r#"{{"page":"/p",{}"#, &report[1..]);
            let diagnostic: Diagnostic =
                serde_json::from_str(&report).unwrap_or_else(|err| panic!("{report}: {err}"));
            assert_eq!(
                diagnostic,
                Diagnostic {
                    page: "/p".into(),
                    kind
                }
            );
        }
    }

    #[test]
    fn rejects_a_report_of_an_unreadable_report() {
        serde_json::from_str::<Diagnostic>(
            r#"{"page":"/p","kind":"unreadable_report","report":"","error":""}"#,
        )
        .expect_err("only the server records unreadable reports");
    }
}
