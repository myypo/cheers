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

## Copy rules

- Button text should name the action: "Save settings", "Invite member", "Retry upload".
- Loading text is honest: "Saving...", "Checking availability...". It must not imply success.
- Success text appears only from backend-confirmed state.
- Error text explains what happened and what to do next.
- Validation messages come from backend validation or the source of truth for the action, then appear near the field.
- Empty states explain what will appear, why it matters, and the first useful action.
- Avoid em dashes, filler adjectives, and generic AI loading jokes.

## Rewrite common areas

### Forms

Use visible labels, not placeholder-only labels. Hints should answer why or format:

- Bad: "DOB"
- Good: "Date of birth" plus hint "Use YYYY-MM-DD."

When changing fields, use the main `cheers` skill to keep form wiring aligned.

### Errors

- Bad: "Forbidden"
- Good: "You do not have permission to edit this project. Ask an owner for access."

Place field-level errors near inputs and connect them semantically where practical. Use alert/live treatment for newly appearing important errors.

### Loading and pending

- Bad: "Done" before the response arrives.
- Good: "Saving..." while pending, then "Saved" or the error only after backend confirmation.

### Destructive actions

Name the object and consequence:

- "Delete deployment `green-42`? This removes its logs from the dashboard."

Undo should be backend-modeled or otherwise truthful, not optimistic removal with a fragile rollback.

## Verify

- A first-time user knows what to do next.
- Error copy is specific, recoverable, and blame-free.
- Terms match across navigation, headings, forms, and actions.
- Copy survives longer translations and small screens.
- All success messages are backend-confirmed.

Finish with `polish` if visual hierarchy or spacing still needs refinement.
