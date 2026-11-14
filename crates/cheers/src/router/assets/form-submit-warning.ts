// Datastar prevents a form's native submission only from `data-on:submit` on the form, or while a
// `contentType: 'form'` action serializing it runs. A form whose action sits on a
// `type="button"` button therefore reloads the page when Enter is pressed in it.

// Input types in which Enter submits a form that has no submit button, when it is the only one.
const IMPLICIT_SUBMISSION_TYPES = new Set([
  "date",
  "datetime-local",
  "email",
  "month",
  "number",
  "password",
  "search",
  "tel",
  "text",
  "time",
  "url",
  "week",
]);

// The shape `__render_action_call` renders for actions that take a form.
const FORM_ACTION =
  /@(?:get|post|put|patch|delete)\('(?:\\.|[^'\\])*',\{contentType:'form'(?:,selector:'((?:\\.|[^'\\])*)')?/g;
const STRING_ESCAPE = /\\(u[0-9a-fA-F]{4}|x[0-9a-fA-F]{2}|.)/g;
const SINGLE_CHARACTER_ESCAPES: Record<string, string> = {
  n: "\n",
  r: "\r",
  t: "\t",
};

const unescapeString = (source: string): string =>
  source.replace(STRING_ESCAPE, (_, sequence: string) =>
    sequence.length > 1
      ? String.fromCharCode(Number.parseInt(sequence.slice(1), 16))
      : (SINGLE_CHARACTER_ESCAPES[sequence] ?? sequence),
  );

const querySelector = (selector: string): Element | null => {
  try {
    return document.querySelector(unescapeString(selector));
  } catch {
    return null;
  }
};

// Resolves each action's form the way Datastar's fetch action does.
const submittedForms = (el: Element): Element[] =>
  Array.from(el.attributes).flatMap((attr) =>
    attr.name.startsWith("data-on:")
      ? Array.from(attr.value.matchAll(FORM_ACTION), ([, selector]) =>
          selector === undefined ? el.closest("form") : querySelector(selector),
        ).filter((form) => form !== null)
      : [],
  );

const isSubmitButton = (el: Element): boolean =>
  (el instanceof HTMLButtonElement || el instanceof HTMLInputElement) &&
  (el.type === "submit" || el.type === "image");

// A submit button running a form action prevents the submission it starts.
const reloads = (form: HTMLFormElement): boolean => {
  if (
    Array.from(form.attributes).some(
      (attr) =>
        attr.name === "data-on:submit" ||
        attr.name.startsWith("data-on:submit__"),
    )
  ) {
    return false;
  }
  const elements = Array.from(form.elements);
  const buttons = elements.filter(isSubmitButton);
  if (buttons.length > 0) {
    return buttons.some((button) => !submittedForms(button).includes(form));
  }
  return (
    elements.filter(
      (el) =>
        el instanceof HTMLInputElement &&
        IMPLICIT_SUBMISSION_TYPES.has(el.type),
    ).length === 1
  );
};

const submitsNatively = (form: HTMLFormElement): boolean => {
  const target = form.getAttribute("target")?.toLowerCase() ?? "";
  return (
    form.hasAttribute("action") ||
    form.getAttribute("method")?.toLowerCase() === "dialog" ||
    (target !== "" && target !== "_self")
  );
};

const warned = new WeakSet<HTMLFormElement>();

const warn = () => {
  const forms = new Set(
    Array.from(document.querySelectorAll("*")).flatMap(submittedForms),
  );
  for (const form of forms) {
    if (
      !(form instanceof HTMLFormElement) ||
      warned.has(form) ||
      submitsNatively(form) ||
      !reloads(form)
    ) {
      continue;
    }
    warned.add(form);
    console.warn(
      "cheers: this form is submitted by a Datastar action outside its `!on:submit`, so pressing " +
        "Enter or a submit button in it submits it natively and reloads the page. Put the action " +
        "on the form with `!on:submit(...)`, or give the form an `action` attribute if it should " +
        "submit natively.",
      form,
    );
  }
};

let scheduled = false;
const schedule = () => {
  if (!scheduled) {
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      warn();
    });
  }
};

schedule();
new MutationObserver((records) => {
  if (
    records.some((record) =>
      Array.from(record.addedNodes).some((node) => node instanceof Element),
    )
  ) {
    schedule();
  }
}).observe(document.documentElement, { childList: true, subtree: true });
