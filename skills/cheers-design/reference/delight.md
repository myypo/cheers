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

## Principles

- Delight must never delay or block the task.
- Delight must not imply success before backend confirmation.
- Delight should be brief, optional where possible, and respectful after repetition.
- The more serious the user state, the quieter the delight.
- Reduced-motion users still get a polished static version.

## Patterns

### Backend-confirmed celebration

Celebrate only after the confirmed success state is rendered. Motion can emphasize that new state, but it should not fire on click before success.

### Helpful empty states

Use the empty state to explain what will appear, why it matters, and the first useful action. Make the tone specific to the product.

### Local micro-interactions

Use hover, focus, active, reveal, and small local toggles to make controls feel considered. Keep them independent of durable state.

### Waiting states

Use reassuring, specific copy. "Importing 24 rows..." is better than generic jokes. For long jobs, show truthful progress when available.

## Avoid

- Confetti for routine saves.
- Humor in severe errors.
- Repeated animations that become annoying.
- Large JS bundles for tiny effects.
- Sound without explicit opt-in.
- Delight that hides poor UX.

## Verify

Does the moment feel earned? Is it still pleasant after repeated use? Does it work with keyboard, screen readers, reduced motion, slow network, and failed backend actions?
