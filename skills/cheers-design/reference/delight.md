# Delight

Add memorable, appropriate moments to a Cheers UI. Delight should amplify successful use, not distract from it.

## Register

- **Product**: delight belongs at earned moments: first success, completion, recovery, milestone, useful shortcut discovery.
- **Brand**: delight can be broader: copy voice, section transitions, imagery, interaction details, seasonal or narrative touches.

## Find earned moments

Look for:

- empty states that could welcome and guide
- backend-confirmed success states
- long waits that need reassuring progress
- error recovery moments
- milestones or first-time actions
- hover/focus micro-interactions that make controls feel finished

Ask what tone is appropriate if unclear: playful, professional, quirky, elegant, calm.

## Design principles

- Delight must never delay or block the task.
- Delight must not imply success before backend confirmation.
- Delight should be brief, optional where possible, and respectful after repetition.
- The more serious the user state, the quieter the delight.
- Reduced-motion users still get a polished static version.

## Cheers patterns

### Backend-confirmed celebration

The action handler records success, then patches a success component. CSS may animate that newly patched state.

```rust
PatchElements::new().element(SaveResult::success())
```

Pair with CSS animation that respects reduced motion. Do not fire a celebration on click before the action succeeds.

### Helpful empty states

Render empty states from backend data. Include a specific CTA using generated actions or normal links.

### Local micro-interactions

Use CSS hover/focus/active and small `scoped_signal!` toggles. Keep them independent of durable state.

### Waiting states

Use `!indicator` and product-specific copy: "Importing 24 rows..." is better than generic jokes. For long backend jobs, stream progress with `EventReceiver` when useful.

## Avoid

- Confetti for routine saves.
- Humor in severe errors.
- Repeated animations that become annoying.
- Large JS bundles for tiny effects.
- Sound without explicit opt-in.
- Delight that hides poor UX.

## Verify

Does the moment feel earned? Is it still pleasant after repeated use? Does it work with keyboard, screen readers, reduced motion, slow network, and failed backend actions?
