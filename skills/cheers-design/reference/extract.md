# Extract

Extract repeated UI patterns into reusable Cheers components and design tokens. The goal is a more coherent design system, not abstraction for its own sake.

## Discover

Find existing conventions first:

- shared components/modules
- design tokens and CSS variables
- layout/base component and page shell
- forms, buttons, nav, badges, empty/error/loading states
- icons, imagery, and motion conventions
- tests or examples that define UI contracts

If no design system or shared component area exists, ask before creating one.

## Identify extraction candidates

Good candidates:

- repeated markup used 3+ times with the same intent
- repeated form rows, error blocks, empty states, status badges, toolbars, nav items
- repeated CSS values that should become semantic tokens
- repeated pending/success/error interaction patterns
- repeated update regions with the same conceptual boundary

Do not extract one-off context-specific UI. Duplication is better than a bad abstraction.

## Plan the extraction

For each candidate, define:

- component or token name
- semantic purpose and non-goals
- props/content slots in design terms
- accessibility contract
- state variants it owns
- CSS classes/tokens it owns or consumes
- migration path for existing call sites
- tests or examples to update

Use the main `cheers` skill for exact `Render`, generated helpers, and test mechanics.

## Token extraction

Create or extend a small semantic vocabulary:

- colors: surface, text, muted text, border, accent, success, warning, error, info
- spacing: component gaps, section gaps, inline gaps
- type: body, label, title, heading, data
- radii, shadows, easing only if actually used

Use tokens where they clarify intent. Do not create a token for every value.

## Migrate

Replace call sites systematically, run formatter/tests, and delete dead CSS or components. Preserve user-facing behavior and state semantics.

## Verify

The extracted pattern should be easier to use correctly than copying markup. It must preserve accessibility, backend-confirmed state, and visual consistency.
