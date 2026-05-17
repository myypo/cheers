# Product register

Use this when design serves a task: app shells, authenticated UI, dashboards, settings, admin tools, forms, tables, workflows, and data surfaces.

## Product slop test

The test is not whether the UI is surprising. The test is whether a user fluent in strong tools would trust it immediately or pause at subtly odd controls, density, spacing, or terminology.

Product UI fails when it is strange without purpose: over-decorated buttons, mismatched controls, gratuitous motion, display fonts in labels, invented affordances for standard tasks, or colors used as decoration. The bar is earned familiarity. The tool should disappear into the task.

## Typography

- System fonts and familiar sans families are legitimate.
- One family is often enough. Product UIs need clear roles more than display/body pairing.
- Use a tight fixed rem scale. Fluid display sizing usually belongs to brand surfaces, not dense tools.
- Use 1.125-1.2 ratios between type steps for compact UI. Larger contrast is fine for page titles and onboarding.
- Keep prose around 65-75ch. Dense data can run wider when scanning and comparison matter.
- Use tabular numbers for aligned numeric data.

## Color

Product defaults to **Restrained**: tinted neutrals plus one accent used with discipline.

- Accent color means primary action, current selection, focus, or important semantic state, not decoration.
- Maintain a state vocabulary: hover, focus, active, disabled, selected, loading, error, warning, success, info.
- Use a second neutral layer for sidebars, toolbars, panels, and raised surfaces.
- Charts and status systems need distinguishable shape, labels, and contrast, not color alone.

## Layout

- Predictable grids are an affordance. Users move faster when structure stays stable.
- Familiar navigation, breadcrumbs, tabs, tables, filters, and form layouts are features, not failures of imagination.
- Use density deliberately. Empty space should clarify relationships, not pad the surface.
- Responsive behavior is structural: collapse sidebars, reflow columns, adapt tables, keep primary actions reachable.

## Components

Every interactive component needs visible default, hover, focus-visible, active, disabled, loading, error, and success states where relevant.

- Empty states should explain what will appear, why it matters, and the first useful action.
- Loading states should preserve layout and set expectations.
- The same control should look and behave the same across the product.
- Icons must share a style and have accessible names or surrounding text.

## Motion

- 150-250ms for most transitions.
- Motion communicates state: feedback, loading, reveal, selection, navigation context.
- Avoid choreographed page-load sequences in task UI.
- Reduced-motion mode must still feel finished.

## Product bans

- Decorative motion that does not convey state.
- Inconsistent component vocabulary across screens.
- Display fonts in labels, buttons, or dense data.
- Reinventing standard controls for flavor.
- Heavy full-saturation accents on inactive or decorative elements.
- Modal-first flows where inline, page, or progressive disclosure would fit better.

## Product permissions

- Familiar patterns, native-feeling controls, and system fonts.
- Density for expert workflows.
- Consistency over surprise.
- Delight at earned moments, not everywhere.
