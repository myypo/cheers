# Adapt

Adapt an existing Cheers design to another viewport, device, input method, or usage context. Adaptation is not scaling pixels; it is rethinking structure for the new context.

## Discover

Identify:

- source context and assumptions: screen size, input, density, connection, user posture
- target context: phone, tablet, desktop, kiosk, print, embedded panel, slow network
- what must remain available
- what can move behind progressive disclosure
- what interaction patterns break: hover-only controls, tiny targets, wide tables, dense sidebars

Ask for target devices and usage context if missing.

## Adaptation strategy

### Mobile

- Stack content into one clear flow.
- Keep primary actions reachable and large enough.
- Avoid hover-dependent interactions.
- Replace wide tables with summary rows, detail pages, or horizontally scrollable tables only when acceptable.
- Use normal navigation and links for page movement.
- Keep forms short; split only when it reduces cognitive load.

### Tablet

- Use master-detail or two-column patterns when useful.
- Support touch and pointer.
- Let side panels collapse based on orientation or container size.

### Desktop and wide screens

- Use horizontal space for persistent navigation, side panels, filters, and data comparison.
- Avoid stretching prose or forms across the whole viewport.
- Add keyboard affordances for power workflows where appropriate.

### Print or static export

- Hide interactive controls and navigation.
- Expand hidden details that matter.
- Preserve semantic heading order.
- Use print-specific CSS rather than a separate data model when possible.

## Cheers fit

- Prefer CSS media/container queries and semantic markup over duplicate component trees.
- If structure truly differs, compose smaller reusable UI pieces.
- Keep conceptual update regions stable across breakpoints.
- Use local affordances for disclosure, not to remove core functionality from a device class.

Use `cheers` for exact component and patch mechanics.

## Verify

Test narrow mobile, mobile landscape, tablet/small laptop, desktop wide, keyboard-only, touch, and 200% zoom. For action-driven UI, verify the interaction that changes state.
