// Posts the page's diagnostics and uncaught errors to the server. Debug builds only: release
// builds bundle a `diagnose` that does nothing in place of this module.
//
// Each diagnostic deserializes into `cheers::test::Diagnostic`, and the test hooks read
// `ID_HEADER`, so keep them in step.

export type Diagnostic =
  | { kind: "form_submitted_outside_on_submit"; form: Element }
  | { kind: "async_stream_target_missing"; key: string }
  | {
      kind: "patch_target_missing";
      target: { id: string } | { selector: string };
    }
  | { kind: "patch_element_without_id"; element: Element }
  | { kind: "uncaught_error"; message: string }
  | { kind: "unhandled_rejection"; message: string };

const ENDPOINT = "/cheers/diagnostics";
const MAX_MESSAGE_LENGTH = 4000;
const MAX_ELEMENT_LENGTH = 500;

// The ids of reports sent from this tab whose response no page has seen. A report sent just before
// a navigation outlives its page, so the tab, not the page, remembers it.
const UNCONFIRMED_KEY = "cheers:unconfirmed-diagnostics";
const ID_HEADER = "x-cheers-diagnostic-id";

const unconfirmed = (): string[] =>
  JSON.parse(sessionStorage.getItem(UNCONFIRMED_KEY) ?? "[]");

const setUnconfirmed = (ids: string[]) =>
  sessionStorage.setItem(UNCONFIRMED_KEY, JSON.stringify(ids));

const reported = new Set<string>();
const checks: (() => void)[] = [];

// Runs `check` when a test settles the page's diagnostics, in case the page is hidden and skips
// the animation frame `check` waits for.
export const beforeSettle = (check: () => void): void => {
  checks.push(check);
};

// The element's own tag and attributes, without its children.
const describe = (element: Element): string =>
  (element.cloneNode(false) as Element).outerHTML.slice(0, MAX_ELEMENT_LENGTH);

const serialize = (diagnostic: Diagnostic) => {
  switch (diagnostic.kind) {
    case "form_submitted_outside_on_submit":
      return { ...diagnostic, form: describe(diagnostic.form) };
    case "patch_element_without_id":
      return { ...diagnostic, element: describe(diagnostic.element) };
    case "uncaught_error":
    case "unhandled_rejection":
      return {
        ...diagnostic,
        message: diagnostic.message.slice(0, MAX_MESSAGE_LENGTH),
      };
    default:
      return diagnostic;
  }
};

export const diagnose = (diagnostic: Diagnostic): void => {
  const body = JSON.stringify({
    page: location.pathname + location.search,
    ...serialize(diagnostic),
  });
  if (reported.has(body)) {
    return;
  }
  reported.add(body);
  const id = `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  setUnconfirmed([...unconfirmed(), id]);
  fetch(ENDPOINT, {
    method: "POST",
    keepalive: true,
    headers: { "Content-Type": "application/json", [ID_HEADER]: id },
    body,
  }).then(
    (response) => {
      setUnconfirmed(
        unconfirmed().filter((unconfirmedId) => unconfirmedId !== id),
      );
      if (!response.ok) {
        console.warn(
          `cheers: the server answered ${response.status} to a diagnostic report. ` +
            `Let POST ${ENDPOINT} through the app's middleware to have it logged.`,
        );
      }
    },
    (error) =>
      console.warn(
        "cheers: a diagnostic report did not reach the server",
        error,
      ),
  );
};

// Datastar's errors link to a page with their context URL-encoded, which repeats the context they
// also carry, including that of a nested error.
const DATASTAR_ERROR_LINK = /(\n|\\n)More info: [^\s"\\]+/g;

const errorMessage = (error: unknown): string =>
  (error instanceof Error ? error.message : String(error)).replace(
    DATASTAR_ERROR_LINK,
    "",
  );

// Chrome raises this when a ResizeObserver callback changes layout; the observer gets the rest
// in the next frame, so nothing has failed.
const BENIGN_ERROR = /^ResizeObserver loop/;

window.addEventListener("error", (event) => {
  const message = event.error ? errorMessage(event.error) : event.message;
  if (!BENIGN_ERROR.test(message)) {
    diagnose({ kind: "uncaught_error", message });
  }
});

window.addEventListener("unhandledrejection", (event) => {
  diagnose({
    kind: "unhandled_rejection",
    message: errorMessage(event.reason),
  });
});

declare global {
  interface Window {
    __cheersSettleDiagnostics: () => string[];
  }
}

// Runs the page's pending checks and returns the ids of the tab's reports that may not have
// reached the server yet.
window.__cheersSettleDiagnostics = () => {
  for (const check of checks) {
    check();
  }
  return unconfirmed();
};
