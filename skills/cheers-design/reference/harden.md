# Harden

Make a Cheers UI resilient to real users, real data, network failures, and assistive technology.

## State coverage

Every interactive surface should intentionally handle:

- default
- loading / pending
- success from backend
- validation error
- network/server error
- empty / first-run
- permission denied / read-only
- disabled
- large data
- long text / short text / emoji / CJK / RTL
- mobile and 200% zoom

No optimistic UI. If the backend has not confirmed it, the UI may say "Saving..." or "Deleting...", but not "Saved" or remove the item as if deletion succeeded.

## Forms

Use Cheers form generation where possible:

- Keep `#[form(...)]`, `form_names!(...)`, input `name=...`, and handler `Form<GeneratedForm>` aligned.
- Validate on the backend.
- Patch the form/component back with field-level errors and preserved values.
- Use visible `<label>` elements. Place errors near fields and connect them with `aria-describedby` where practical.
- Use `role="alert"` or a live region for form-level errors when the patch appears after submit.
- Prevent duplicate submission with `!indicator` and disabled state; add backend idempotency for high-risk actions.

Pattern:

```rust
form
    !on:submit((SaveSettingsAction {}))
    !indicator(signal_saving)
    !attr("aria-busy": { (signal_saving) " ? 'true' : null" })
{
    label for=id_email { "Email" }
    input id=id_email name=form_email value=(@&self.email) aria:describedby=id_email_error;
    @if let Some(error) = &self.email_error {
        p id=id_email_error role="alert" { (@&error) }
    }
    button type="submit" !attr("disabled": signal_saving) { "Save settings" }
}
```

## Error handling

Map backend outcomes to rendered states:

- 400/validation: patch field errors and preserve values.
- 401: redirect to login or patch an auth-expired state.
- 403: explain permission and show the allowed next step.
- 404: not-found state with navigation out.
- 409: conflict state with current backend value and retry/refresh.
- 429: rate-limit copy with when to retry if known.
- 500/network: apology, retry, support/debug reference if appropriate.

Keep errors specific and recoverable. Do not show raw internal errors.

## Deletion and undo

Use backend-modeled undo, not optimistic removal.

Good options:

- Soft-delete in the backend, patch the row into a "pending deletion" or "deleted" state with an Undo action.
- Require confirmation for truly irreversible, high-cost, or batch operations.
- For low-cost removal, action returns the updated list only after the backend records the change.

Avoid: removing from DOM immediately and rolling back if the request fails.

## Dynamic updates

Use `EventReceiver` when updates continue after the handler returns or multiple users/processes can change the same surface. Keep command/write actions short-lived and stream reads/updates separately when that simplifies coordination.

For streams:

- send `PatchElements` for structural changes
- send `PatchSignals` only for small reactive values
- compress SSE when available
- stop streams cleanly when the client disconnects

## Internationalization and layout resilience

- Use logical CSS properties where direction may vary: `margin-inline`, `padding-inline`, `border-inline-start`.
- Avoid fixed-width text containers and fixed-width buttons.
- Let controls grow for translated labels.
- Test 30-40% longer copy.
- Use `min-width: 0` on flex/grid children that contain text.
- Use `overflow-wrap: break-word` where user-generated text can appear.
- Format dates/numbers server-side or through a small justified helper using locale-aware APIs.

## Testing

Read `../cheers/testing.md` before adding tests.

Prefer:

1. Render tests for static semantics and edge content.
2. Handler/router tests for backend state and validation outcomes.
3. Browser tests with `cheers::test::App` for actions, patches, signals, streams, and keyboard/client behavior.

Test at least one failure path for each critical action.
