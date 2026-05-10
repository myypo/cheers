# Clarify

Improve UX copy, labels, help text, errors, and status messages in a Cheers UI. Clear copy reduces mistakes and makes backend-confirmed state understandable.

## Assess

Find copy that is:

- vague: "Submit", "Error", "Invalid input"
- repetitive: heading and intro saying the same thing
- jargon-heavy or based on internal model names
- passive or evasive during errors
- too clever for high-stress moments
- missing a next step
- inconsistent with terms used elsewhere

Ask for audience technical level and user mental state only when the code/product docs do not answer it.

## Cheers copy rules

- Button text should name the action: "Save settings", "Invite member", "Retry upload".
- Loading text is honest: "Saving...", "Checking availability...". It must not imply success.
- Success text appears only from backend-confirmed state.
- Error text explains what happened and what to do next.
- Validation messages come from backend validation when the action is submitted, then patch near the field.
- Empty states explain what will appear, why it matters, and the first useful action.
- Avoid em dashes, filler adjectives, and generic AI loading jokes.

## Rewrite common areas

### Forms

Use visible labels, not placeholder-only labels. Hints should answer why or format:

- Bad: "DOB"
- Good: "Date of birth" plus hint "Use YYYY-MM-DD."

When changing fields, keep `#[form]`, `form_names!(...)`, `name=...`, and handler `Form<T>` in sync.

### Errors

- Bad: "Forbidden"
- Good: "You do not have permission to edit this project. Ask an owner for access."

Render field-level errors near inputs and connect them with `aria-describedby` where practical. Use `role="alert"` for newly patched important errors.

### Loading and pending

- Bad: "Done" before the response arrives.
- Good: button pending text "Saving..." with `!indicator` and disabled state, then backend patches "Saved" or the error.

### Destructive actions

Name the object and consequence:

- "Delete deployment `green-42`? This removes its logs from the dashboard."

For undo, use a backend-modeled soft-delete or pending state, not optimistic removal.

## Verify

- A first-time user knows what to do next.
- Error copy is specific, recoverable, and blame-free.
- Terms match across navigation, headings, forms, and actions.
- Copy survives longer translations and small screens.
- All success messages are backend-confirmed.

Finish with `polish` if visual hierarchy or spacing still needs refinement.
