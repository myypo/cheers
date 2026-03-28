use std::fmt::{Debug, Display};

use macros::Refs;

use crate::{
    context::Context,
    render::{Buffer, Render},
    router::css_url,
};

/// Renders `<!DOCTYPE html>`.
///
/// This is the first item in a full HTML document response.
///
/// # Example
///
/// ```
/// use cheers::{components::Doctype, prelude::*};
///
/// let page = html! {
///     (Doctype)
///     html {
///         body { "Hello" }
///     }
/// };
///
/// assert_eq!(
///     page.render().into_inner(),
///     "<!DOCTYPE html><html><body>Hello</body></html>"
/// );
/// ```
#[derive(Refs)]
pub struct Doctype;

impl Render for Doctype {
    fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
        // XSS SAFETY: static doctype string
        buffer.dangerously_get_string().push_str("<!DOCTYPE html>");
    }
}

/// Renders the core Cheers client-side runtime scripts.
///
/// This includes the streaming helper script and the `datastar.js` runtime. In debug builds it
/// also includes the live-reload script.
///
/// Include this in pages that use Cheers client-side behaviors such as actions, signals and patching
///
/// # Example
///
/// ```
/// use cheers::{components::Scripts, prelude::*};
///
/// let rendered = Scripts.render().into_inner();
///
/// assert!(rendered.contains("/cheers/assets/datastar.js"));
/// ```
#[derive(Refs)]
pub struct Scripts;

impl Render for Scripts {
    fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
        // XSS SAFETY: static HTML streaming script
        buffer.dangerously_get_string().push_str("<script>function __ssrStream(key){const t=document.querySelector(`[data-ssr='${key}-t']`),s=document.querySelector(`[data-ssr='${key}']`);s.replaceWith(t.content);t.remove();document.querySelector(`[data-ssr='${key}-s']`).remove()}</script>");
        // XSS SAFETY: static inclusion of datastar
        buffer
            .dangerously_get_string()
            .push_str(r#"<script src="/cheers/assets/datastar.js"></script>"#);
        if cfg!(debug_assertions) {
            // XSS SAFETY: static reload script
            buffer.dangerously_get_string().push_str(
                r#"
<script>
(function() {
  let attempts = 0;

  function connect() {
    const source = new EventSource("/cheers/live-reload");

    source.onopen = function() {
      if (attempts !== 0) {
          window.location.reload();
      }
      console.log("Cheers live-reload connected");
      attempts = 0;
    };

    source.onmessage = function(event) {
      if (event.data === "reload") {
        console.log("Cheers reload event received, reloading page...");
        window.location.reload();
      }
    };

    source.onerror = function(err) {
      console.error("Cheers live-reload connection error:", err);

      const delay = Math.min(50 * Math.pow(1.25, attempts), 1000);
      source.close();
      attempts++;
      setTimeout(() => {
        connect();
      }, delay);
    };
  }

  connect();
})();
</script>
                "#,
            );
        }
    }
}

/// Renders the `<link rel="stylesheet">` tag for the Cheers CSS bundle.
///
/// This links to the framework-managed stylesheet path produced by the router.
#[derive(Refs)]
pub struct Css;

impl Render for Css {
    fn render_to(&self, buffer: &mut Buffer<crate::context::Element>) {
        let link = format!(r#"<link rel="stylesheet" href="/cheers{}">"#, css_url());
        // XSS SAFETY: CSS URL is computed by us
        buffer.dangerously_get_string().push_str(&link);
    }
}

/// A value rendered via its [`Display`] implementation.
///
/// This will handle escaping special characters for you.
///
/// # Example
///
/// ```
/// use cheers::{components::Displayed, prelude::*};
///
/// let rendered = html! { p { (Displayed("<hello>")) } }.render().into_inner();
///
/// assert_eq!(rendered, "<p>&lt;hello&gt;</p>");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Displayed<T: Display>(pub T);

impl<C: Context, T: Display> Render<C> for Displayed<T>
where
    for<'a> std::fmt::Arguments<'a>: Render<C>,
{
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<C>) {
        format_args!("{}", self.0).render_to(buffer);
    }
}

/// A value rendered via its [`Debug`] implementation.
///
/// This will handle escaping special characters for you.
///
/// # Example
///
/// ```
/// use cheers::{components::Debugged, prelude::*};
///
/// let rendered = html! { pre { (Debugged(vec![1, 2, 3])) } }
///     .render()
///     .into_inner();
///
/// assert_eq!(rendered, "<pre>[1, 2, 3]</pre>");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Debugged<T: Debug>(pub T);

impl<C: Context, T: Debug> Render<C> for Debugged<T>
where
    for<'a> std::fmt::Arguments<'a>: Render<C>,
{
    #[inline]
    fn render_to(&self, buffer: &mut Buffer<C>) {
        format_args!("{:?}", self.0).render_to(buffer);
    }
}
