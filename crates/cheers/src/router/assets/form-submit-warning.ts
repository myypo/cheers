// Debug builds only. Datastar blocks a form's native submission only from `data-on:submit` on
// the form or while an action started by a click inside it runs, so pressing Enter in a form whose
// button is `type="button"` reloads the page instead of running the action.

const ACTION_CALL = /@(?:get|post|put|patch|delete)\(/;

const triggersAction = (el: Element): boolean => {
  for (const attr of el.attributes) {
    if (attr.name.startsWith("data-on") && ACTION_CALL.test(attr.value)) {
      return true;
    }
  }
  return false;
};

document.addEventListener("submit", (evt) => {
  const form = evt.target;
  if (!(form instanceof HTMLFormElement) || evt.defaultPrevented) return;

  const submitter = evt.submitter;
  if (
    form.hasAttribute("action") ||
    submitter?.hasAttribute("formaction") ||
    (submitter?.getAttribute("formmethod") ?? form.method) === "dialog"
  ) {
    return;
  }
  if (
    !triggersAction(form) &&
    !Array.from(form.querySelectorAll("*")).some(triggersAction)
  ) {
    return;
  }

  console.warn(
    "cheers: a form with Datastar actions inside was submitted natively to the current URL, " +
      "reloading the page. Put its action on the form with `!on:submit(...)`, or " +
      "give the form an `action` attribute if a native submission is intended.",
    form,
  );
});
