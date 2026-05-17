# Animate

Add motion that clarifies state, feedback, hierarchy, or navigation in a Cheers UI. Motion must not hide latency, imply success before confirmation, or require unnecessary JS.

## Decide the role of motion

Product surfaces:

- 150-250ms for most feedback and state transitions.
- Motion conveys pending, reveal, selection, validation, navigation context, or changed hierarchy.
- No choreographed page-load delays that block task flow.

Brand surfaces:

- Motion can carry voice: staged reveals, scroll moments, typographic rhythm, image treatment.
- Still preserve accessibility, performance, and readable content.

Always support `prefers-reduced-motion` with an intentional static or reduced alternative.

## Motion hierarchy

Use the smallest tool that achieves the effect:

1. CSS transitions/keyframes for hover, focus, reveal, and state classes.
2. Native elements/APIs such as `dialog`, `popover`, CSS scroll snapping, and View Transitions when progressive.
3. Cheers/Datastar affordances for local toggles, pending state, and backend-confirmed updates.
4. A static JS helper only when the behavior cannot stay declarative and the experience justifies it.

Use the main `cheers` skill for exact Datastar syntax and server-pushed behavior.

## Patterns to favor

### Reveal

- Use opacity, transform, clip, or grid-row techniques with short durations.
- Keep the element reachable by keyboard and screen readers.
- Do not hide core content behind hover-only reveals.

### Pending feedback

- Animate the pending affordance, not fake success.
- Success or failure appears only from backend-confirmed state.
- For long operations, prefer real progress or staged messages over an endless spinner.

### Validation feedback

- Transition color, icon, or helper text when a backend-rendered error appears.
- Avoid aggressive shake animations, especially for repeated validation.

## Easing and timing

Use natural deceleration:

```css
:root {
  --ease-out-quart: cubic-bezier(.25, 1, .5, 1);
  --ease-out-quint: cubic-bezier(.22, 1, .36, 1);
  --ease-out-expo: cubic-bezier(.16, 1, .3, 1);
}
```

Guidelines:

- 100-150ms: press or toggle acknowledgement.
- 150-250ms: hover, selection, small reveal.
- 250-400ms: larger reveal, drawer/dialog entrance.
- Exits usually shorter than entrances.

Avoid bounce/elastic easing, long feedback animations, and animation that blocks task completion.

## Performance and accessibility

- Prefer transform and opacity for movement.
- Use blur/filter/mask/shadow only when bounded and verified smooth.
- Avoid layout-property animation unless deliberately measured.
- Do not rely on motion alone to communicate state.
- Preserve keyboard and screen-reader behavior.
- Reduced-motion mode should still look finished.

## Verify

Check pending state, success/failure timing, reduced motion, keyboard use, mobile/touch, target device smoothness, console errors, and state after backend confirmation.
