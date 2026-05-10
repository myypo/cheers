# Animate

Add motion that clarifies state, feedback, hierarchy, or navigation in a Cheers UI. Motion must not hide latency, imply success before confirmation, or require unnecessary JS.

## Decide the role of motion

Product surfaces:

- 150-250ms for most feedback and state transitions.
- Motion conveys state: pending, reveal, selection, validation, navigation context.
- No choreographed page-load delays that block task flow.

Brand surfaces:

- Motion can carry voice: staged reveals, scroll moments, typographic rhythm.
- Still preserve accessibility, performance, and readable content.

Always support `prefers-reduced-motion`.

## Preferred implementation order

1. CSS transitions/keyframes for hover, focus, reveal, and state classes.
2. Native elements/APIs: `dialog`, `popover`, CSS scroll snapping, View Transitions when progressive.
3. Datastar attributes and tiny expressions for toggles or class/state changes.
4. `EventReceiver` when motion represents long-running backend progress or coordinated updates.
5. `JsScript` for server-pushed imperative behavior when declarative patterns are not enough.
6. `include_js_bundle!` for reusable static helpers only when the interaction justifies a bundle.

Do not add client-framework animation dependencies to a Cheers app unless the project already uses them and the effect has a strong reason.

## Datastar-friendly patterns

### Toggle reveal

```rust
scoped_signal!(signal_open: bool);
html! {
    button
        !on:click({ (signal_open) " = !" (signal_open) })
        !attr("aria-expanded": { (signal_open) " ? 'true' : 'false'" })
    { "Details" }
    div class="reveal" !show(signal_open) {
        "More information"
    }
}
```

Pair with CSS:

```css
.reveal {
  transition: opacity 180ms cubic-bezier(.22, 1, .36, 1), transform 180ms cubic-bezier(.22, 1, .36, 1);
}
@media (prefers-reduced-motion: reduce) {
  .reveal { transition: none; }
}
```

### Pending feedback

Use request indicators, not fake success:

```rust
button
    type="submit"
    !indicator(signal_saving)
    !attr("disabled": signal_saving)
{
    span !show({ "!" (signal_saving) }) { "Save" }
    span !show(signal_saving) { "Saving..." }
}
```

The backend response patches the final success/error state.

### Validation feedback

Render error state from the backend and let CSS transition color/opacity if desired. Do not shake inputs on every keystroke or validate only in the browser.

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

- 100-150ms: press/toggle acknowledgement.
- 150-250ms: hover, selection, small reveal.
- 250-400ms: larger reveal, drawer/dialog entrance.
- Exits usually shorter than entrances.

Avoid bounce/elastic easing, long feedback animations, and animation that blocks task completion.

## Performance and accessibility

- Prefer `transform` and `opacity` for movement.
- Use blur/filter/mask/shadow only when bounded and measured smooth.
- Avoid layout-property animation unless using a deliberate technique and verified smooth.
- Do not rely on motion alone to communicate state.
- Preserve keyboard and screen-reader behavior.
- Use focus management for dialogs/menus.
- Reduced-motion mode should still look intentional, not broken.

## Verify

Check:

- pending state does not imply success
- reduced motion
- keyboard use
- mobile/touch
- no jank on target device class
- no console errors
- state after backend patch remains correct
