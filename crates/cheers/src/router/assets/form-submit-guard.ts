import debug from "./debug";

// Datastar cancels a form's native submission only from `data-on:submit` on the form or while a
// `contentType: 'form'` action serializing it runs. Without this guard, pressing Enter in a form
// whose action sits on a `type="button"` button submits it natively and reloads the page.

const ACTION_CALL = /@(?:get|post|put|patch|delete)\(/;
const SELECTOR_OPTION = /selector:'((?:\\.|[^'\\])*)'/g;
const STRING_ESCAPE = /\\(u[0-9a-fA-F]{4}|x[0-9a-fA-F]{2}|.)/g;
const SINGLE_CHARACTER_ESCAPES: Record<string, string> = {
  n: "\n",
  r: "\r",
  t: "\t",
};

const actionExpressions = (el: Element): string[] =>
  Array.from(el.attributes)
    .filter(
      (attr) =>
        attr.name.startsWith("data-on:") && ACTION_CALL.test(attr.value),
    )
    .map((attr) => attr.value);

const triggersAction = (el: Element): boolean =>
  actionExpressions(el).length > 0;

const unescapeString = (source: string): string =>
  source.replace(STRING_ESCAPE, (_, sequence: string) =>
    sequence.length > 1
      ? String.fromCharCode(Number.parseInt(sequence.slice(1), 16))
      : (SINGLE_CHARACTER_ESCAPES[sequence] ?? sequence),
  );

const selectsForm = (form: HTMLFormElement, expression: string): boolean =>
  Array.from(expression.matchAll(SELECTOR_OPTION)).some((match) => {
    try {
      return form.matches(unescapeString(match[1]));
    } catch {
      return false;
    }
  });

const hasActions = (form: HTMLFormElement): boolean =>
  triggersAction(form) ||
  Array.from(form.querySelectorAll("*")).some(triggersAction) ||
  Array.from(form.elements).some(triggersAction) ||
  Array.from(document.querySelectorAll("*")).some((el) =>
    actionExpressions(el).some((expression) => selectsForm(form, expression)),
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
    !(form instanceof HTMLFormElement) ||
    submitsElsewhere(form, evt.submitter) ||
    !hasActions(form)
  ) {
    return;
  }

  const unhandled =
    !evt.defaultPrevented &&
    !Array.from(form.attributes).some((attr) =>
      attr.name.startsWith("data-on:submit"),
    ) &&
    !(evt.submitter && triggersAction(evt.submitter));
  evt.preventDefault();

  if (debug && unhandled) {
    console.warn(
      "cheers: blocked the native submission of a form with Datastar actions, which would " +
        "reload the page. Put the form's action on the form with `!on:submit(...)` so submitting " +
        "it runs the action, or give the form an `action` attribute to submit it natively.",
      form,
    );
  }
});
