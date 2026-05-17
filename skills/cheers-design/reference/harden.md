# Harden

Make a Cheers UI resilient to real users, real data, network failures, assistive technology, and localization. This is UX hardening first; exact Cheers mechanics live in the main `cheers` skill.

## State coverage

Every interactive surface should intentionally handle:

- default
- loading and pending
- success from backend-confirmed state
- validation error
- network/server error
- empty and first-run
- permission denied and read-only
- disabled
- large data
- long text, short text, emoji, CJK, RTL
- mobile and 200% zoom

No optimistic UI. If the backend has not confirmed it, the UI may say "Saving..." or "Deleting...", but not "Saved" or remove the item as if deletion succeeded.

## Forms and decisions

- Use visible labels, not placeholder-only labels.
- Put hints where they answer why, format, or consequences.
- Preserve user input after validation errors.
- Place field errors near fields and connect them semantically where practical.
- Use form-level errors for system or permission failures.
- Prevent risky duplicate submission with visible pending treatment and disabled or guarded controls.
- Keep destructive actions specific: name the object and consequence.

Use the `cheers` skill for exact form generation, handlers, action wiring, and tests.

## Error handling UX

Map outcomes to understandable states:

- validation: field errors and preserved values
- auth expired: sign-in path or refreshed session state
- permission: explain who can act and what to do next
- not found: not-found state with navigation out
- conflict: show current backend value and recovery choice
- rate limit: say when to retry if known
- server/network: apology, retry path, support/debug reference if appropriate

Keep errors specific, recoverable, and blame-free. Do not show raw internal errors.

## Deletion and undo

Use backend-modeled undo or confirmation, not optimistic removal.

Good options:

- soft-delete, then render a deleted or pending-deletion state with undo
- confirmation for irreversible, high-cost, or batch operations
- updated list rendered only after the backend records the change

Avoid removing from the UI immediately and hoping rollback covers failure.

## Internationalization and layout resilience

- Use logical CSS properties where direction may vary.
- Avoid fixed-width text containers and fixed-width buttons.
- Let controls grow for translated labels.
- Test 30-40% longer copy.
- Use `min-width: 0` on flex/grid children that contain text.
- Use `overflow-wrap: break-word` where user-generated text can appear.
- Keep dates, numbers, units, and relative time readable for the target locale.

## Accessibility resilience

- Keyboard path reaches every control in logical order.
- Focus stays visible and moves intentionally after major changes.
- Important dynamic status is announced.
- Touch targets are large enough.
- Color is never the only meaning.
- Reduced-motion mode preserves clarity.

## Verify

Walk success, validation failure, permission failure, server failure, slow network, mobile, keyboard-only, screen-reader-relevant semantics, 200% zoom, and long localized copy.
