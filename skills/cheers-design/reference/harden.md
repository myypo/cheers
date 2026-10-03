# Harden

Make a Cheers UI resilient to real users, real data, failing networks, assistive technology, touch, and localization. Designs that only work with perfect data are not production-ready. This is UX hardening first; exact Cheers mechanics live in the main `cheers` skill.

## Visitor mode

- **Persuade + Experience:** the committed world must survive real content: long names, translated headlines, missing images, slow media, and reduced motion keep the composition intact rather than collapsing it to a template.
- **Operate + Read:** every state is part of the product. Errors, empty results, permissions, and large data use the same component vocabulary as the happy path, near the decision point.

Refinement preserves: harden states and edges inside the established world, copy voice, and behavior. Do not restyle the surface while hardening it.

## Two isolated assessments

When a subagent tool is available and permitted, run these independently; otherwise run them yourself in this order.

1. **Resilience assessment.** Walk the real flows and answer each question with rendered or source evidence:
   - **States:** Which of default, loading, pending, backend-confirmed success, validation error, server or network error, empty and first-run, no results, permission denied, read-only, and disabled exist? Which are missing or improvised?
   - **Content extremes:** What happens with 100+ character names, a single character, empty values, emoji, CJK, RTL, accents, millions and billions, 1000+ rows, 50+ options?
   - **Failure:** What does the user see when the action fails, times out, is rate limited, conflicts with another edit, or the session expires? Is input preserved, and is there a way forward?
   - **Concurrency:** What happens on ten rapid clicks, two tabs, or a stream that disconnects mid-update?
   - **Trust:** Does anything show success, removal, completion, or a new order before the backend confirms it?
   - **Access:** Can keyboard, screen reader, 200% zoom, forced colors, and touch users complete every path?
   - **Gestures:** Do custom sliders, drag surfaces, and swipe rows recover from interruption and offer a non-gesture path?
2. **Mechanical scan.** Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) over the changed pages, or check by hand when it is unavailable. Also look for fixed-width text containers, missing `min-width: 0`, placeholder-only labels, and `user-scalable=no`.

Keep scan results out of the first assessment, then synthesize both before editing.

## State the resilience contract

Before editing, list per surface: the states to cover, the backend outcome each one renders from, where each error appears, what the pending state disables, and how retry and recovery work. Name the interaction contract for each action.

## Apply

Read [craft-floor.md](craft-floor.md) before editing. Implement through `cheers`.

### Trust and pending

- No optimistic UI. Pending may say "Saving..." or "Deleting...", disable risky repeats, and show truthful progress. It never says "Saved", removes the item, completes the step, or reorders the list before the backend confirms.
- Guard against duplicate submission with a visible pending treatment and disabled or guarded controls, so ten rapid clicks produce one action.
- Retry repeats the same backend action and shows pending again; it never reports success the server did not send.
- A stream that drops says so, reconnects or offers a way to, and never presents stale data as live.

### Forms and validation

- Visible labels, never placeholder-only. Hints answer why, format, or consequence, before submission.
- The backend validates and is the source of truth; native constraints (`required`, `maxlength`, `type`) are a courtesy that never replaces it.
- Preserve user input after errors. Field errors sit at the field and are connected semantically; system and permission failures get a form-level error.
- Don't block submission unnecessarily.
- Destructive actions name the object and consequence.

### Error states

Map each outcome to a state the user understands:

- **Validation:** field errors with preserved values and a suggested correction.
- **Session expired:** a sign-in path that returns the user where they were, input intact where possible.
- **Permission:** who can act and how to get access; read-only views say why.
- **Not found:** a not-found state with navigation out.
- **Conflict:** the current backend value and a recovery choice.
- **Rate limit:** when to retry, if known.
- **Server or network:** a plain apology, a retry path, and a support reference when useful.

Keep errors specific, recoverable, and blame-free; never show raw internal errors or codes as the message. One failing region shows its own error; never block the whole interface.

### Deletion and undo

Use backend-modeled undo or confirmation, never optimistic removal with rollback:

- soft-delete, then render a deleted or pending-deletion state with undo;
- confirmation for irreversible, high-cost, or batch operations;
- the updated list rendered only after the backend records the change.

### Empty, loading, and large data

- Empty states distinguish first use, no results, active filters, permissions, and failure, each with the next useful action.
- Loading names what is loading, preserves layout, and gives time expectations for long operations.
- Large data uses backend pagination, filtering, search, or chunking; never send 10,000 rows at once.

### Text, layout, and localization

- No fixed-width text containers or buttons; budget 30 to 40% expansion for translations.
- `min-width: 0` on flex and grid children holding text; `overflow-wrap: break-word` where user-generated text appears. Truncate or line-clamp only where the full value is reachable another way.
- Logical properties (`margin-inline-start`, `padding-inline`) and mirrored directional icons where direction may vary.
- UTF-8 everywhere. Dates, numbers, currencies, units, and relative time formatted for the locale on the server. Plurals and messages come from whole translatable strings, never concatenated fragments.
- Body and input text at least 16px on mobile (14px only for genuinely secondary text); containers grow at 200% zoom.

### Accessibility

- Every control is keyboard reachable in logical order, with visible focus that moves intentionally after major changes, into and back out of dialogs. Skip links on long pages.
- Important dynamic status is announced through semantic live status, not only shown.
- Color is never the only meaning; forced-colors mode keeps borders, focus, and state visible.
- Reduced motion keeps every state legible.
- Images and controls have alt text or accessible labels.
- Content and navigation are server-rendered HTML that still reads when a script fails or loads slowly; prefer normal links and forms where they suffice.
- Use feature detection, not browser detection; give unsupported CSS a fallback with `@supports`.

### Touch and gestures

- Touch targets at least 44x44 CSS px; nothing essential depends on hover.
- Every swipe, drag, pinch, or long-press has a visible single-tap alternative. Never disable pinch zoom.
- Interrupted gestures in custom controls, which are static JS helpers under the JS-is-exceptional guardrail:
  - A second finger or pointer mid-drag: the first drag keeps its pointer or ends cleanly, never jumps.
  - `pointercancel`, `lostpointercapture`, release outside the control, or window `blur` mid-drag: clear the dragging state and release capture.
  - After each, the next tap or drag works without a reload.
- Static JS helpers remove their listeners, timers, and pending requests when their element is patched away or removed.
- A drag that changes data drops into a pending state; the confirmed result comes from the backend, and a failure returns the item to its confirmed place with the error.

## Verify

Walk every state in one batched round, desktop and mobile together: success, validation failure, permission failure, server failure, offline and throttled network, a dropped stream, rapid repeated clicks, 1000+ items, empty data, 100+ character names, emoji, RTL, CJK, 30 to 40% longer copy, keyboard only, screen-reader semantics, automated accessibility checks (such as axe) where the project runs them, forced colors, 200% zoom, and touch. For gestures, add a second finger mid-drag, scroll across the control, release outside it, switch windows mid-drag, then drag again. Fix everything in one batch and confirm with at most one more round.

Say what produced the evidence for each check: rendered page, forced backend error, throttled network, emulated viewport, synthesized touch and which engine, or a physical device. Name what stayed untested. Add a behavioral regression test for each confirmed state or gesture fix the project's test runner can drive; use `cheers` guidance for tests and format changed templates with `cargo cheers fmt --rustfmt <files>`.

When edge cases are covered, hand off to `cheers-design polish`.
