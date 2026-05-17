---
name: cheers-design
description: "Use when a Cheers task requires UI/UX design judgment: shaping a new visible surface, materially changing layout, hierarchy, copy, interaction states, motion, onboarding, or requested design audit/polish."
argument-hint: "[teach|document|shape|craft|extract|critique|audit|polish|harden|optimize|clarify|adapt|onboard|layout|typeset|colorize|bolder|quieter|distill|delight|animate|overdrive] [target]"
user-invocable: true
---

# Cheers Design

Design and iterate production-grade interfaces for Cheers apps. This skill owns UX, visual direction, interaction design, content, accessibility, state coverage, and design-system judgment. The main `cheers` skill owns the technical Cheers/Rust/Datastar implementation rules.

## Required setup

Before design work or file edits:

1. If the task involves code edits and `cheers` is not already loaded, read it for implementation mechanics. Do not duplicate those mechanics here.
2. Read existing `PRODUCT.md` and `DESIGN.md` when present. If they are missing and the task depends on strategic context, use `teach` or `document` rather than inventing a brand or system.
3. Classify the surface as **product** or **brand**, then read the matching register reference:
   - [reference/product.md](reference/product.md) for authenticated app UI, dashboards, settings, tools, forms, and data surfaces. Design serves a task.
   - [reference/brand.md](reference/brand.md) for landing pages, marketing, long-form, portfolio, campaign, public storytelling. Design is the product.
4. If a sub-command is used, read its reference file from `reference/`.
5. Inspect the target app's visible UI, source, components, styles, content, state coverage, tests, and conventions before changing code.
6. State the interaction contract before implementation at design level: normal navigation, backend-confirmed action, local affordance signal, stream, server-pushed script, or static JS helper.

If the task is visually open-ended, use `shape` first. Do not implement an unconfirmed major direction unless the user explicitly asks you to proceed without a separate brief.

## Skill boundary

Use this skill to decide:

- user purpose, primary action, information architecture, and flow
- register, theme, visual direction, hierarchy, composition, typography, color, motion, and imagery
- empty, loading, pending, error, success, permission, overflow, mobile, and first-run states
- labels, microcopy, status messages, help text, and recovery copy
- design-system alignment, reusable UI patterns, and visual quality bar

Use the main `cheers` skill to decide exact implementation details such as generated ids/actions/forms, `Render` mechanics, Datastar attribute syntax, patch APIs, streams, tests, and formatting. In this skill, keep Cheers guidance at the level of design constraints and interaction contracts.

## Cheers interaction guardrails

These are design constraints, not a duplicate implementation manual.

1. **Backend-confirmed trust.** Do not design optimistic success, irreversible removal, completed steps, or reordered data before the backend confirms them.
2. **Honest pending states.** Pending UI may say what is happening, disable risky repeated actions, and show progress. It must not pretend the work is done.
3. **Smallest dynamic layer wins.** Prefer normal navigation and forms when enough; use backend-confirmed actions for structural updates; use signals for local affordances; reserve streams and JS for interactions that truly need them.
4. **Signals are affordances, not app models.** Local open/closed, focus, selection, pending, and lightweight input affordances are fine. Broad backend state belongs in rendered state.
5. **Semantic HTML remains the design substrate.** Headings, landmarks, labels, focus, live status, keyboard paths, and native browser behavior are part of the interface, not implementation afterthoughts.
6. **JS is exceptional.** Reach for CSS, native browser features, and Cheers hypermedia first. Static JS helpers need a clear experiential reason.

## Shared design laws

Apply these without fighting the Cheers interaction guardrails.

- Start with user purpose, content, states, constraints, and primary action before layout or styling.
- Product UI should feel trustworthy, consistent, task-focused, and familiar where familiarity helps. See [product.md](reference/product.md).
- Brand UI needs a point of view. Typography, imagery, composition, and color should feel specific, not generated. See [brand.md](reference/brand.md).
- Pick a color strategy before values: Restrained, Committed, Full palette, or Drenched. Use OKLCH when possible; tint neutrals rather than defaulting to pure black, pure white, or flat gray.
- Choose light or dark from a scene sentence: who uses this, where, under what ambient light, in what mood. Do not default by category.
- Use typography for hierarchy and voice. Keep prose around 65-75ch; avoid flat type scales.
- Vary spacing for rhythm. Same padding everywhere is monotony. Cards are not a default container, and nested cards are a design smell.
- Motion must clarify hierarchy, feedback, loading, reveal, or transition. Respect reduced motion and do not animate layout casually.
- Use real content over placeholders. Handle empty, loading, pending, error, success, overflow, long text, permissions, and first-run states.
- Every word earns its place. Avoid repeated headings, filler claims, vague button labels, em dashes in product copy, and status text that overpromises.
- Avoid generic AI tells: gradient text, decorative glassmorphism, nested cards, endless equal card grids, hero metric blocks, side-stripe card accents, modal-first flows, redundant copy, and category-reflex palettes.
- Run the category-reflex check: if the theme, palette, typography, or layout could be guessed from the product category alone, rework the visual direction.
- Keep examples and implementation advice in Cheers/Rust/Datastar terms when code is needed, but prefer referencing `cheers` over repeating syntax here.

## Commands

| Command | Use | Reference |
|---|---|---|
| `teach` | Capture strategic product/brand context in PRODUCT.md | [reference/teach.md](reference/teach.md) |
| `document` | Create or refresh DESIGN.md from Cheers UI code | [reference/document.md](reference/document.md) |
| `shape [target]` | Produce a task-specific UX/UI brief before code | [reference/shape.md](reference/shape.md) |
| `craft [target]` | Build a confirmed brief into Cheers code and iterate | [reference/craft.md](reference/craft.md) |
| `extract [target]` | Extract reusable UI components, patterns, and design tokens | [reference/extract.md](reference/extract.md) |
| `critique [target]` | Design-director review with prioritized feedback | [reference/critique.md](reference/critique.md) |
| `audit [target]` | Design and UI-quality implementation report without fixing | [reference/audit.md](reference/audit.md) |
| `polish [target]` | Refine an existing UI to shipping quality | [reference/polish.md](reference/polish.md) |
| `harden [target]` | Make states, errors, a11y, i18n, and edge cases production-ready | [reference/harden.md](reference/harden.md) |
| `optimize [target]` | Improve perceived and measured UI performance | [reference/optimize.md](reference/optimize.md) |
| `clarify [target]` | Improve labels, microcopy, loading, success, and error text | [reference/clarify.md](reference/clarify.md) |
| `adapt [target]` | Adapt UI to another viewport, device, or input context | [reference/adapt.md](reference/adapt.md) |
| `onboard [target]` | Design first-run, empty, and activation flows | [reference/onboard.md](reference/onboard.md) |
| `layout [target]` | Fix spacing, rhythm, hierarchy, and responsive structure | [reference/layout.md](reference/layout.md) |
| `typeset [target]` | Improve typography hierarchy, readability, and font strategy | [reference/typeset.md](reference/typeset.md) |
| `colorize [target]` | Introduce strategic semantic or brand color | [reference/colorize.md](reference/colorize.md) |
| `bolder [target]` | Make a safe design more confident without AI effects | [reference/bolder.md](reference/bolder.md) |
| `quieter [target]` | Reduce visual noise while preserving intent | [reference/quieter.md](reference/quieter.md) |
| `distill [target]` | Remove clutter and reduce interaction/state complexity | [reference/distill.md](reference/distill.md) |
| `delight [target]` | Add appropriate, backend-confirmed moments of personality | [reference/delight.md](reference/delight.md) |
| `animate [target]` | Add purposeful motion and state feedback | [reference/animate.md](reference/animate.md) |
| `overdrive [target]` | Propose ambitious Cheers-safe polish; build after confirmation | [reference/overdrive.md](reference/overdrive.md) |

### Routing rules

1. If the first word of the skill arguments matches a command above, read that reference and follow it.
2. If there is no command, treat the request as general UI design/craft: perform setup, apply the shared design laws, and choose the nearest reference when useful.
3. If implementation changes Cheers templates, use the main `cheers` validation guidance. At minimum, format changed templates with `cargo cheers fmt --rustfmt <files>` and run targeted checks appropriate to the change.
