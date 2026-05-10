# Bolder

Make a safe or bland Cheers UI more confident without falling into generic AI effects.

## Register

- **Product**: bolder means clearer hierarchy, stronger affordances, better density, and one decisive accent. It rarely means theatrics.
- **Brand**: bolder means stronger point of view: scale, type, image, color, pacing, and composition can all push harder.

## Assess

Find why the design feels timid:

- flat hierarchy
- medium-sized everything
- weak primary action
- generic centered stack
- low contrast between sections
- timid or absent imagery
- safe neutral palette with no strategy
- no memorable brand moment

Ask about risk tolerance only if the register and product context do not make it clear.

## Amplify safely

### Hierarchy

Pick one focal point and make it unmistakable. Increase contrast between primary, secondary, and tertiary content through size, weight, spacing, and placement before adding effects.

### Typography

- Product: stronger title/body/label roles, clearer weights, more confident button and tab treatments.
- Brand: larger display type, more distinctive pairings, stronger line breaks, more committed voice.

### Color

- Product: accent only where it carries meaning: primary action, current selection, status.
- Brand: choose Committed, Full palette, or Drenched when the surface earns it. Do not hedge.
- Use OKLCH where the project allows. Never use pure black/white as the main neutral.

### Composition

Break monotony with asymmetry, varied section rhythm, stronger alignment, or one signature visual motif. Avoid identical card grids.

### Motion

Use purposeful motion only: reveal, feedback, state transition. Brand can have larger choreographed moments; product should stay quick and task-focused.

## Cheers constraints

- Do not add client-side state just to make the UI feel lively.
- Do not show success or completed state before backend confirmation.
- Use `!indicator` for pending states and patch the bolder confirmed state from the backend.
- Keep motion in CSS/Datastar/native APIs unless a JS bundle is justified.

## Never

- Gradient text.
- Decorative glassmorphism.
- Cyan/purple glow as a default bold move.
- Side-stripe card accents.
- Making every element loud.
- Sacrificing readability, contrast, keyboard use, or performance.

## Verify

Would a user trust the bolder product UI more, not less? Would the brand surface be remembered? Does it still pass accessibility, reduced motion, and Datastar state correctness?
