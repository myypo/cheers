# Layout

Fix spacing, rhythm, hierarchy, and responsive structure in a Cheers UI. Layout problems are usually structural, not cosmetic.

## Register

- **Product**: predictable grids, consistent density, familiar navigation, structural responsive changes.
- **Brand**: stronger composition, asymmetric or strict editorial grids, fluid spacing, deliberate pacing.

## Assess

Check:

- Is the primary action obvious in two seconds?
- Are related elements close and unrelated groups separated?
- Is every section/card using the same padding and rhythm?
- Are there nested cards or wrappers without purpose?
- Does the layout adapt on mobile, or only shrink?
- Are tables, forms, and toolbars using familiar patterns?
- Are patch targets stable and local enough for the layout sections being updated?

## Improve

### Establish spacing

Use existing CSS variables or introduce a small local scale:

```css
:root {
  --space-1: .25rem;
  --space-2: .5rem;
  --space-3: .75rem;
  --space-4: 1rem;
  --space-6: 1.5rem;
  --space-8: 2rem;
}
```

Use `gap` for sibling spacing. Use margins for relationships between independent blocks. Avoid arbitrary one-off values unless the optical adjustment is intentional.

### Choose the right structure

- Flexbox for rows, toolbars, button groups, and component internals.
- Grid for page regions, dashboards, responsive card/list arrangements, and coordinated columns.
- Container queries when a component must adapt to its parent, not the viewport.
- `min-width: 0` on flex/grid children containing text.

### Reduce container noise

Do not wrap everything in cards. Use cards only for distinct objects or actionable groups. Never nest cards inside cards. Use headings, spacing, dividers, and background layers for hierarchy.

### Responsive behavior

- Product: collapse sidebars, stack form columns, transform tables into appropriate compact views, keep primary actions reachable.
- Brand: change composition and pacing per breakpoint, not just font size.
- All touch targets should be large enough and not hover-only.

## Cheers considerations

- Extract repeated structural chunks into `Render` components.
- Give patchable sections generated ids.
- Prefer patching a whole coherent layout region over many tiny fragments when morphing can handle it.
- Keep CSS in project conventions, usually `include_css!` for global CSS or component-scoped classes where established.

## Never

- Use side-stripe card accents.
- Use identical icon-card grids as the default answer.
- Center everything by reflex.
- Use arbitrary z-index values instead of a semantic scale.
- Hide core functionality on mobile.

## Verify

Run the squint test, keyboard through the page, check mobile/tablet/desktop, and inspect long text. If the structure lands but details still feel unfinished, run `polish`.
