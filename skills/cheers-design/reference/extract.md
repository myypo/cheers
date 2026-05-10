# Extract

Extract repeated Cheers UI patterns into reusable `Render` components, forms, ids, and design tokens.

## Discover

Find existing conventions first:

- shared components/modules
- `#[derive(Cheers)]` usage
- generated ids and forms
- CSS variables or style modules
- layout/base component, `include_css!`, `include_svg_sprite!`, `include_js_bundle!`
- tests for component rendering or browser behavior

If no design system or shared component area exists, ask before creating one.

## Identify extraction candidates

Good candidates:

- repeated markup used 3+ times with the same intent
- repeated form rows, error blocks, empty states, status badges, toolbars, nav items
- repeated CSS values that should become tokens
- repeated Datastar interaction patterns such as save buttons with indicators
- repeated patchable regions needing generated ids

Do not extract one-off context-specific UI. Duplication is better than a bad abstraction.

## Plan the extraction

For each candidate, define:

- component name and module location
- props and defaults
- generated ids, signals, and form declarations
- semantic HTML and ARIA contract
- CSS classes/tokens it owns or consumes
- migration path for existing call sites
- tests to update or add

## Cheers component guidance

- A reusable component is a Rust value implementing `Render`.
- `#[derive(Cheers)]` generates helpers; it does not render by itself.
- Bind generated names inside `render_to`.
- Use associated helpers outside the component for patch targets and selectors.
- Keep backend state in props/view models, not in signals.
- Expose child content only when it stays semantically meaningful.

## Token extraction

Create or extend a small semantic vocabulary:

- colors: surface, text, muted text, border, accent, success, warning, error, info
- spacing: component gaps, section gaps, inline gaps
- type: body, label, title, heading
- radii/shadows/easing only if actually used

Use tokens where they clarify intent. Do not create a token for every value.

## Migrate

Replace call sites systematically, run formatter/tests, and delete dead CSS or components. Preserve generated ids where external tests or actions rely on them, or update tests intentionally.

## Verify

The extracted component should be easier to use correctly than copying markup. It must preserve accessibility, backend-confirmed state, and Datastar behavior.
