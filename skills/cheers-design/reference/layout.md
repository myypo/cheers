# Layout

Layout turns product priority into reading order, grouping, rhythm, and usable space. Diagnose the structural problem before moving boxes.

## Visitor mode

- **Persuade + Experience:** composition may be asymmetric, fluid, or intentionally disruptive when the selected world earns it. Change composition and pacing per breakpoint, not just size.
- **Operate + Read:** predictable structure, stable density, and navigable linearity are affordances. See [operate.md](operate.md#layout).

Preserve the established visual world. A layout command changes structure inside it; identity replacement belongs to [new-work.md](new-work.md) and DESIGN.md.

## Two isolated assessments

When a sub-agent tool is available and permitted, run these independently; otherwise run them yourself in this order. Do not let scan findings anchor the design assessment.

1. **Layout assessment.** Inspect representative states and viewports. Answer every question with rendered or source evidence (a file, selector, or computed value):
   - **Reading order:** apply the squint test. With detail blurred, can you still find the primary element, the secondary element, and the major groups, in order? Is the primary action obvious within two seconds?
   - **Grouping:** are related items close and distinct groups separated, or are containers compensating for weak proximity?
   - **Rhythm:** do tight and generous intervals create a deliberate cadence, or is one spacing value repeated until everything has equal weight?
   - **Structure:** does the topology match the content and task? Are repeated cards, columns, or sections genuinely equivalent, or a default?
   - **Density:** does the information per region fit use frequency, decision complexity, and visitor mode?
   - **Adaptation:** at narrow, intermediate, wide, zoomed, and localized states, what reorders, collapses, wraps, scrolls, or stays fixed? Do DOM and focus order still agree with the visual order?
   - **Update regions:** do the regions the backend patches line up with meaningful visual sections? Does a patch, a validation error, or a pending state shift the surrounding structure?
   - **Extremes:** do long content, empty states, overlays, sticky elements, safe areas, and small touch targets expose structural failures?
2. **Mechanical scan.** Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) with the layout scope, for example `impeccable detect --json --scope layout <url>`, adding `--viewport 390x844` for mobile. Also inspect arbitrary spacing, overflow, stacking, and container behavior it cannot resolve, including values set in `html!` templates.

Synthesize both before editing. A clean scan cannot prove hierarchy or rhythm.

## Set the spatial thesis

Before editing, name:

- the primary reading or task path;
- what belongs together and what must separate;
- which element leads and which supports;
- the intended density and spacing rhythm;
- how the structure changes across containers, viewports, input modes, and content extremes.

Choose the simplest structural model that expresses those relationships.

## Apply

Read [craft-floor.md](craft-floor.md) before the first edit; its spacing checks and its refusals (card scaffolds, nested cards, hero-metric template, side stripes) apply here and are not repeated.

- Group by meaning. Use proximity before adding containers, borders, or decoration; use headings, spacing, dividers, and background layers for hierarchy.
- Create rhythm through deliberate contrast between tight and generous intervals.
- Use the project's documented spacing scale. When none exists, introduce a small semantic one (`--space-xs` through `--space-xl`) on a 4-unit base, which gives the middle steps an 8-only scale misses. Avoid one-off values unless the optical correction is deliberate.
- Let hierarchy follow product priority, not framework or template defaults. Do not center everything by reflex.
- Pick primitives by the relationship they control: flexbox for rows, toolbars, button groups, and component internals; grid for page regions, dashboards, and coordinated columns; container queries when a component must adapt to its parent rather than the viewport. Put `min-width: 0` on flex and grid children that hold text.
- Use `gap` for sibling rhythm; margins for relationships between independent blocks.
- Make responsive behavior structural: reorder, collapse sidebars, stack form columns, give tables a compact view, and keep primary actions reachable. Never hide core functionality on mobile.
- Give touch targets a usable hit area (about 44px) even when the visible mark is smaller, and never make an action hover-only.
- Use depth only when it clarifies state or hierarchy, and stack with a semantic z-index scale, not arbitrary values.
- Make optical corrections only after inspecting the rendered result.

Variation is not a goal by itself. Repetition supports recognition; break it only when content or priority changes.

### Cheers fit

- Keep backend update regions stable and visually coherent: a patched region keeps its footprint, and pending or error states reserve their space instead of pushing the page around.
- Extract a repeated structural chunk into a component only when it improves clarity and reuse.
- Follow the project's CSS conventions; use `cheers` for component and patch mechanics. Format changed templates with `cargo cheers fmt --rustfmt <files>`.

## Verify

Inspect in one batched round (desktop and mobile together), fix everything it shows in one batch, confirm with at most one more round, then rerun the scan once. Answer each item with rendered or source evidence; a bare "yes" is not verification.

- The squint test still reveals the primary, secondary, and major groups in order.
- The reading and task path stays clear at every supported size.
- Related content groups naturally; unrelated content does not blur together.
- Tight and generous spacing make an intentional rhythm, not monotony.
- Density matches use frequency and content complexity.
- Long text, empty states, localization, zoom, and backend-patched content do not break the structure.
- Keyboard, touch, and assistive-technology order agree with the visual order.
- The final scan has no unexplained findings.

When the structure holds, hand off to `cheers-design polish`.
