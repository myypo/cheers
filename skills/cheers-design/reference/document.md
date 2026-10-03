# Document

Create or refresh `DESIGN.md` at the project root (or the app root in a workspace with several apps) so future UI work stays visually consistent. It is the visual companion to `PRODUCT.md`: strictly visual, never a copy of product truth or surface strategy.

DESIGN.md follows the [DESIGN.md format spec](https://raw.githubusercontent.com/google-labs-code/design.md/main/docs/spec.md) used by Google Stitch: YAML frontmatter with machine-readable tokens, then up to eight markdown sections in a fixed order. Tokens are normative; prose explains where and why to apply them.

## When to run

- New-work found a coherent incumbent system but no DESIGN.md.
- A new world or an approved system change has been built and reviewed (new-work section 7 records it here).
- An existing DESIGN.md has drifted from the code.
- Before a large redesign, to capture the current state as reference.

On a standalone `document` run, never overwrite an existing DESIGN.md silently. Show it and ask: refresh, overwrite, or merge. The record-from-build pass below already has the user's world approval and writes without asking. If another tool in the project expects a specific format, preserve that format and fold these requirements into it.

## Choose the path

- **Scan mode (default):** the project has tokens, components, or rendered output. Extract, then confirm the descriptive language.
- **Seed mode:** the project is pre-implementation. Route through new-work's world workshop and write a directional seed.

Decide by scanning first. If the scan finds no tokens, no components, and no rendered pages, offer seed mode; do not switch silently. `cheers-design document --seed` requests the workshop but never authorizes replacing coherent code: when an incumbent system exists, offer scan mode, or route an explicit identity replacement through [new-work.md](new-work.md).

## Record from the build

At the finish of a new world or approved system change, DESIGN.md is written from what shipped, preferably by a fresh subagent given the project root, the changed files, the direction contract from the surface brief, PRODUCT.md, this file, and the write boundary. Without subagents, step out of the build context and follow these rules yourself.

- **The build is ground truth.** Every token and rule must be evidenced by the built code, never by the plan. The contract's OWN-WORLD block names the world; the build shows how it landed. Where they diverge, the build wins and the prose may note the divergence.
- **Extension vs new world.** A new world or approved system change writes DESIGN.md from durable, reused rules in the build. On an approved system change, update an existing DESIGN.md rather than replacing it: preserve confirmed incumbent decisions and reconcile them with the build. A redesign replaces it. An ordinary extension preserves the incumbent DESIGN.md and its files; compare the build against it and report pre-existing drift without repairing it unasked. When the recorded system still matches, leave it untouched and report the evidence checked. Never write merely to prove the pass ran.
- **Do not legitimize defects.** A rule goes wrong in two ways: a prohibition that bans a device the world itself uses natively, and a value recorded to make a finding disappear. Check every prohibition against the world's own materials; a value earns its place by the build and by legibility.
- **Never canonize a craft-floor refusal.** An element [craft-floor.md](craft-floor.md) refuses (kickers and eyebrows, hard offset shadows outside a neobrutalist world, glyph icons, system display faces, gradient text) is recorded as a defect the build carries, never as a rule for future surfaces to inherit. One shipped violation written into DESIGN.md becomes the house style.
- **Keep surface strategy out.** The first surface's composition, narrative, and contract stay in its surface brief.
- **Do not re-interview.** Take the qualitative language from the direction contract and the confirmed answers instead of asking again.
- **Budget the reading.** Batch reads, take this file and the stylesheets first, sample components rather than walking the tree, and start writing by the midpoint.

Return: paths written, or "No changes" with the files checked; a five-line system summary (palette, type ramp, named rules); and one line naming the defects or drift not canonized or repaired, and why.

## Scan mode

### 1. Find the design assets

In priority order:

1. CSS custom properties: `--color-`, `--font-`, `--space-` and `--spacing-`, `--radius-`, `--shadow-`, `--ease-`, `--duration-` declarations, with name, value, and file.
2. Token files, theme stylesheets, and the global stylesheet's base type and color assignments.
3. Shared components (Rust types implementing `Render`) and page shells: buttons, forms and inputs, containers, navigation, badges, dialogs and popovers, empty, error, loading, and pending states. Note variants and default styles.
4. Icon and image assets, and motion conventions.
5. The running app, when available: sample computed styles from `body`, headings, links, buttons, and key containers. This catches values tokens miss.

### 2. Extract and stage the frontmatter

- **Colors:** group by role into Primary, Secondary, Tertiary, Neutral. With one accent, write Primary plus Neutral; never invent Secondary or Tertiary.
- **Typography:** map observed sizes and weights to display, headline, title, body, label. Note stacks and the scale ratio.
- **Layout:** grid, containers, breakpoints, spacing rhythm, density.
- **Elevation:** the shadow vocabulary, or an explicit statement that the system is flat or tonally layered.
- **Shapes:** radius, corner, border, clipping, recurring silhouettes.
- **Components:** per common component, shape, color assignment, hover and focus treatment, padding, and states.

Skip anything the project does not have. Stop at what is reused; one-offs and empty scales pollute the system.

### 3. Ask for the qualitative language

Ask in at most two structured rounds of up to three questions, waiting between rounds:

- **Creative North Star:** one named metaphor for the system. Offer two or three options grounded in the incumbent world and PRODUCT.md's brand personality.
- **Overview voice:** mood words, the aesthetic philosophy in two or three sentences, and any confirmed visual anti-reference.
- **Color character:** descriptive names for key colors ("Deep Muted Teal-Navy", not "blue-800"), two or three options each.
- **Elevation and component philosophy:** flat, layered, or lifted; the feel of buttons, containers, and inputs in one phrase.

Carry a PRODUCT.md line only when it is a durable constraint on the visual system: a binding logo, identity asset, accessibility need, or brand commitment.

### 4. Write, then confirm

Write the frontmatter and body below. Show the result, call out the non-obvious choices (color names, named rules, atmosphere language), and offer to refine any section.

## Frontmatter

Include only tokens the project uses. Token groups are limited to `colors`, `typography`, `rounded`, `spacing`, and `components`; no `motion:`, `shadows:`, or `breakpoints:` groups. The only other valid top-level keys are `name`, `description`, `version`, and `omitted` (a list of deliberately omitted sections).

```yaml
---
name: <project title>
description: <one-line design-system summary>
colors:
  primary: "#b8422e"
  primary-deep: "#8f3222"
  surface: "#faf7f2"
  ink: "#201916"
typography:
  display:
    fontFamily: "Example Display, Georgia, serif"
    fontSize: "clamp(2.5rem, 7vw, 4.5rem)"
    fontWeight: 700
    lineHeight: 1
  body:
    fontFamily: "system-ui, sans-serif"
    fontSize: "1rem"
    fontWeight: 400
    lineHeight: 1.55
rounded:
  sm: "4px"
  md: "8px"
spacing:
  sm: "8px"
  md: "16px"
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.surface}"
    rounded: "{rounded.md}"
    padding: "12px 16px"
  button-primary-hover:
    backgroundColor: "{colors.primary-deep}"
---
```

- Token refs use `{path.to.token}`. Components may reference primitives; primitives never reference each other.
- `colors` must include a `primary` key; when the project's own names differ, alias `primary` to its main accent.
- Colors accept any valid CSS color string. Hex is the portable default; keep an incumbent `oklch()`, `hsl()`, or wide-gamut value when it is the project's normative source. One source of truth: prose names a token and its role but never restates a different value.
- Typography props: `fontFamily`, `fontSize`, `fontWeight`, `lineHeight`, `letterSpacing`, `fontFeature`, `fontVariation`. Include only the real ones.
- Component tokens use at most `backgroundColor`, `textColor`, `typography`, `rounded`, `padding`, `size`, `height`, `width`. Variants are sibling keys (`button-primary-hover`). Shadows, focus rings, motion, and backdrop effects go in the body prose.
- Keep the project's own token names (`--color-surface-raised` becomes `surface-raised`). Never rename a mature system to Material defaults.
- Values must match the real CSS custom properties, stylesheets, or component defaults.
- Fluid `clamp()` sizes and multi-value padding may draw linter warnings, since the spec's Dimension is a single px, em, or rem value. Keep them only when they are the normative source.

Do not write a `.cheers-design/design.json` sidecar. Upstream Impeccable writes a JSON sidecar for its live design panel; `cheers-design` has no consumer for it, so the frontmatter carries the tokens and the body carries everything else.

## Body

Use these headings verbatim, in this order. Omit a section that does not apply rather than inventing rules; never rename one ("Colors", not "Color Palette & Roles").

```markdown
# Design System: [Project]

## Overview
**Creative North Star: "[Named metaphor]"**
[Two or three paragraphs: personality, density, philosophy, confirmed visual rejections. End with **Key Characteristics:** bullets.]

## Colors
[Palette character in one sentence, then Primary / Secondary / Tertiary / Neutral, each color as **Descriptive Name** (value): where and why. Contrast rules, status colors, light/dark behavior.]

## Typography
[Display, body, and label families with fallbacks and character. Hierarchy: display, headline, title, body (with measure), label. Tabular numerals, font loading.]

## Layout
[Grid or spatial model, containers, density, spacing rhythm, responsive changes. Exact values only when observed.]

## Elevation & Depth
[Shadows, tonal layering, or both. If flat, say so and say how depth is conveyed. Shadow vocabulary with exact values.]

## Shapes
[Corner and radius strategy, borders, clipping, recurring geometry.]

## Components
[Per component: character line, shape, colors, states (hover, focus-visible, active, disabled, pending, error, success), distinctive behavior. Buttons, links, inputs, containers, navigation, badges, dialogs, empty/error/loading states, signature components.]

## Do's and Don'ts
### Do:
- **Do** [prescription with exact values or a named rule].
### Don't:
- **Don't** [prohibition confirmed by the incumbent system or the user].
```

- **Named Rules:** `**The [Name] Rule.** [short doctrine]`, one to three per section where the system has real invariants. They are citable and sticky for later agents.
- **Descriptive first, value in parens:** "gently curved edges (8px)", not a class name.
- **Functional over decorative:** say where and why each token is used.
- **Decisive where evidence is decisive:** hard language for invariants, softer for provisional guidance. Audit tests only when grounded in the observed system or a confirmed decision.
- **Motion has no token group.** The world's motion grammar goes in Overview; per-component motion goes in Components.
- **Guardrails are system-wide.** Do not turn a task-specific concept into a global prohibition. Interaction trust always applies: no optimistic success, no pending state that reads as done.

## Cheers-specific content

Capture design-relevant implementation facts without turning DESIGN.md into an API manual:

- CSS custom property names used by Cheers templates and stylesheets;
- components whose appearance depends on client behavior or local affordance signals;
- the state vocabulary for pending, error, success, empty, and permissions, and which states appear only after backend confirmation;
- conceptual update regions that affect layout or motion;
- reusable UI patterns and their accessibility contracts.

These belong in Components and Do's and Don'ts. For exact syntax and implementation rules, defer to the main `cheers` skill.

## Seed mode

For projects with no visual system to extract. It produces a user-chosen world scaffold, not a fabricated token spec.

1. **Route through the workshop.** PRODUCT.md is the prerequisite; complete [init.md](init.md) first when it is missing. Then load [new-work.md](new-work.md), resolve visual authority, and name a concrete first surface (the user's target, or ask). Run **Create or replace the visual world** and **Commit the world**, then stop after the seed and the surface brief; do not implement. If new-work already ran the workshop this session, use its chosen direction without asking again.
2. **Write the seed.** Open with `<!-- SEED: established with the user before implementation; re-run cheers-design document once there's code to capture the actual tokens and components. -->`. Frontmatter carries only `name` and `description`. Use the canonical sections: Overview (thesis, material, imagery stance, motion grammar, reusable signature; not the first surface's composition), Colors and Typography (strategy, roles, and character, with values or faces only when established, otherwise `[to be resolved during implementation]`), Layout, Elevation & Depth, and Shapes as invariants, no Components section, and Do's and Don'ts holding only durable guardrails from the world choice.
3. **Confirm.** Show the seed, say plainly that it is a seed, and tell the user to re-run `cheers-design document` once code exists. When the world is built through new-work, its finish replaces the seed with a scan of the build.

## Verify

Check with evidence, not a bare yes:

- every frontmatter value traces to a file and a real CSS value or component default;
- headings match the canonical names and order, and no token is restated with a different value in prose;
- no rule canonizes a craft-floor refusal, no value is recorded to make a reviewer finding disappear, and no prohibition bans a device the world uses natively;
- another agent could build a new screen from it without inventing colors, spacing, type, state behavior, or Cheers interaction trust rules.
