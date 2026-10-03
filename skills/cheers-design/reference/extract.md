# Extract

Pull repeated UI patterns into reusable Cheers components and CSS custom properties. The goal is a more coherent design system, not abstraction for its own sake. Extraction is a refinement: the incumbent identity, behavior, copy, and every call site's appearance stay the same unless the user asks otherwise.

## 1. Discover the system

Find the existing conventions first:

- shared component modules (Rust types implementing `Render`), the layout or page shell, and how components are named and exported;
- CSS custom properties, token files, and the stylesheets that own them;
- DESIGN.md frontmatter tokens and component entries, when present;
- forms, buttons, navigation, badges, and empty, error, loading, and pending states;
- icons, imagery, and motion conventions;
- tests or examples that define UI contracts.

If no design system or shared component area exists, do not create one yet. Ask where it should live and how it should be structured.

## 2. Identify candidates

Extract only what is used three or more times with the same intent:

- repeated markup: form rows, error blocks, empty states, status badges, toolbars, nav items, containers;
- hard-coded colors, spacing, type, radii, and shadows that should become semantic tokens;
- inconsistent variations of one concept (three slightly different primary buttons);
- repeated type styles: the same size, weight, and line-height combination;
- repeated easing, duration, or keyframe combinations;
- repeated pending, success, and error interaction patterns, and update regions with the same conceptual boundary.

Duplication beats a bad abstraction. Two elements that look alike but serve different purposes stay separate.

## 3. State the plan before editing

For each candidate, name:

- the component or token name, matching existing conventions;
- its semantic purpose and non-goals;
- content slots and variants in design terms;
- the accessibility contract (landmark or role, label, focus, keyboard path, live status);
- the states it owns, including which appear only after backend confirmation;
- the CSS classes and custom properties it owns or consumes;
- the migration path for existing call sites, and the tests or examples to update.

Grow the system incrementally: extract what is clearly reusable now, not what might be someday. Read [craft-floor.md](craft-floor.md) before editing; an extracted component must not canonize a pattern the floor refuses.

## 4. Extract

- **Components:** a small content API with sensible defaults and only the variants call sites need. Semantic HTML, focus, and keyboard behavior are built in, not left to callers. Signals inside a component stay local affordances; durable state arrives as rendered input. Use the main `cheers` skill for `Render`, generated helpers, and test mechanics.
- **Tokens:** a small semantic vocabulary layered over primitives, declared as CSS custom properties in the project's stylesheet conventions:

```css
:root {
  --color-surface: oklch(98% 0.01 80);
  --color-text-muted: oklch(45% 0.02 80);
  --color-danger: oklch(55% 0.18 25);
  --space-inline: 0.5rem;
  --space-section: 4rem;
  --radius-control: 6px;
}
```

Cover surface, text, muted text, border, accent, and status colors; inline, component, and section spacing; body, label, title, heading, and data type; radii, shadows, and easing only where actually used. A token needs semantic meaning; do not create one for every value.

## 5. Migrate

Replace call sites systematically, format changed templates with `cargo cheers fmt --rustfmt <files>`, run the targeted tests, and delete the dead CSS and components. Preserve user-facing behavior and state semantics.

## 6. Document

When tokens or shared components changed, update DESIGN.md to match per [document.md](document.md): frontmatter values, the Components section, and any named rule the extraction made true. Do not rewrite unrelated sections, and report pre-existing drift rather than repairing it unasked.

## Verify

Check with evidence in one bounded pass, and fix what it shows in one batch:

- each migrated call site renders as before at desktop and mobile widths, including its pending, error, empty, and success states;
- the extracted pattern is easier to use correctly than copying markup;
- accessibility, backend-confirmed state, and keyboard and focus order are preserved;
- no hard-coded value remains where a token now exists, and no token exists without a consumer;
- the [mechanical scan](../SKILL.md#mechanical-scan-optional), when available, shows no new findings.

Never extract one-off context-specific UI, components so generic they say nothing, or abstractions that ignore the existing conventions.
