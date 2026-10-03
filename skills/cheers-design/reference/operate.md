# Operate mode depth (and Read notes)

Use this when design serves a task: app shells, authenticated UI, dashboards, settings, admin tools, forms, tables, workflows, and data surfaces. The essentials live in SKILL.md's visitor modes, [modes.md](modes.md), and [craft-floor.md](craft-floor.md); this file is extended depth. Read surfaces (docs, guides, long-form) take this file's typography and consistency rules; their prose measure and navigation matter more than component density.

## The product slop test

Familiarity is often a feature here. The test is whether a category-fluent user can trust the interface immediately or must pause at every subtly-off control, density, spacing, or term.

Product UI fails when it is strange without purpose: over-decorated buttons, mismatched form controls, gratuitous motion, display fonts where labels should be, invented affordances for standard tasks, color used as decoration. The bar is earned familiarity. The tool should disappear into the task.

## Typography

- **One family is often right.** A well-tuned sans carries headings, buttons, labels, body, and data. System fonts and familiar sans families are legitimate.
- **Fixed rem scale, not fluid.** Clamp-sized headings don't serve product UI; a fluid h1 that shrinks in a sidebar looks worse, not better.
- **Tighter scale ratio.** 1.125–1.2 between steps is typical. Larger contrast is fine for page titles and onboarding.
- **Line length still applies to prose** (65–75ch). Data and compact UI can run denser; tables at 120ch+ are fine.
- Use tabular numerals for aligned numeric data.

## Color

Restrained is the floor: tinted neutrals plus one accent used with discipline. A single surface can earn Committed (a dashboard where one category color carries a report, a drenched welcome screen in onboarding), and [modes.md](modes.md) asks color to do real jobs in the shell.

- Standardize a state vocabulary: hover, focus, active, disabled, selected, loading, error, warning, success, info.
- Accent color marks primary actions, current selection, focus, and state, not decoration.
- Use a second neutral layer for sidebars, toolbars, and panels, slightly cooler or warmer than the content surface.
- Charts and status systems need distinguishable shape, labels, and contrast, not color alone.

## Layout

- Predictable grids are an affordance; users move faster when structure stays stable.
- Familiar navigation, breadcrumbs, tabs, tables, filters, and form layouts are features, not failures of imagination.
- Use density deliberately. Empty space should clarify relationships, not pad the surface.
- Responsive behavior is structural: collapse sidebars, reflow columns, adapt tables, keep primary actions reachable. Not fluid typography.

## Components

Every interactive component has default, hover, focus-visible, active, disabled, pending, error, and success states where relevant. Don't ship half of them.

- Loading states preserve layout (skeletons over spinners in the middle of content) and set expectations without implying success.
- Empty states teach the interface: what will appear, why it matters, the first useful action. Never just "nothing here".
- Consistent affordances across the surface: same button shape, same form-control vocabulary, same icon style. Icons have accessible names or surrounding text.
- Overlays escape their container. An absolutely positioned dropdown inside an `overflow: hidden` or `overflow: auto` ancestor gets clipped; reach for `<dialog>`, the popover API, or `position: fixed`.

## Motion

- 150–250ms on most transitions. Users are in flow; don't make them wait for choreography.
- Motion conveys state: change, feedback, loading, reveal, navigation context. Nothing else.
- No orchestrated page-load sequences. Product loads into a task.
- Reduced-motion mode must still feel finished.

## Product constraints

- Decorative motion that doesn't convey state.
- Inconsistent component vocabulary across screens. If the "save" button looks different in two places, one is wrong.
- Display fonts in labels, buttons, or data.
- Reinventing standard affordances for flavor (custom scrollbars that hurt usability, weird form controls, non-standard modals).
- Heavy color or full-saturation accents on inactive states.
- Modal as first thought. Exhaust inline, page, and progressive alternatives first.

## Product permissions

- System fonts and familiar sans defaults.
- Standard navigation patterns: top bar plus side nav, breadcrumbs, tabs, command palettes.
- Density: tables with many rows, panels with many labels, dense information when users need it.
- Consistency over surprise. Delight is saved for earned moments, not pages.
