import debug from "./debug";

// Datastar prevents a form's native submission only from `data-on:submit` on the form, or while a
// `contentType: 'form'` action serializing it runs. Without this guard, pressing Enter in a form
// whose action sits on a `type="button"` button submits it natively and reloads the page.

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

const selects = (selector: string, form: HTMLFormElement): boolean => {
  try {
    return document.querySelector(unescapeString(selector)) === form;
  } catch {
    return false;
  }
};

// Resolves each action's form the way Datastar's fetch action does.
const submitsForm = (el: Element, form: HTMLFormElement): boolean =>
  Array.from(el.attributes).some(
    (attr) =>
      attr.name.startsWith("data-on:") &&
      Array.from(attr.value.matchAll(FORM_ACTION)).some(([, selector]) =>
        selector === undefined
          ? el.closest("form") === form
          : selects(selector, form),
      ),
  );

const hasFormActions = (form: HTMLFormElement): boolean =>
  Array.from(document.querySelectorAll("*")).some((el) =>
    submitsForm(el, form),
  );

const submitsElsewhere = (
  form: HTMLFormElement,
  submitter: HTMLElement | null,
): boolean => {
  const attribute = (submitterName: string, formName: string) =>
    (
      submitter?.getAttribute(submitterName) ??
      form.getAttribute(formName) ??
      ""
    ).toLowerCase();
  const target = attribute("formtarget", "target");

  return (
    form.hasAttribute("action") ||
    submitter?.hasAttribute("formaction") === true ||
    attribute("formmethod", "method") === "dialog" ||
    (target !== "" && target !== "_self")
  );
};

window.addEventListener("submit", (evt) => {
  const form = evt.target;
  if (
    evt.defaultPrevented ||
    !(form instanceof HTMLFormElement) ||
    submitsElsewhere(form, evt.submitter) ||
    !hasFormActions(form)
  ) {
    return;
  }

  evt.preventDefault();

  if (debug && !(evt.submitter && submitsForm(evt.submitter, form))) {
    console.warn(
      "cheers: blocked the native submission of a form that Datastar actions submit, which " +
        "would reload the page. Put the form's action on the form with `!on:submit(...)` so " +
        "submitting it runs the action, or give the form an `action` attribute to submit it " +
        "natively.",
      form,
    );
  }
});
