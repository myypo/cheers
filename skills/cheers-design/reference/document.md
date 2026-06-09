# Document

Create or refresh `DESIGN.md` for a Cheers app so future UI work stays visually consistent. This is the visual companion to `PRODUCT.md`.

DESIGN.md should follow the Google Stitch DESIGN.md shape: YAML frontmatter with machine-readable tokens, followed by a markdown body with six fixed top-level sections. Tokens are normative; prose explains how to apply them.

## Before writing

If `DESIGN.md` exists, do not overwrite it silently. Read it and ask whether to merge, refresh, or replace.

Scan the project first:

- global CSS and included CSS files
- CSS custom properties and token files
- shared UI components and page shells
- buttons, forms, cards/containers, nav, badges, empty/error/loading states
- icon and image assets
- motion conventions
- rendered output if practical

## Modes

### Scan mode

Use when code exists. Extract actual visual system values and component behavior from the repo.

### Seed mode

Use when the app has little or no UI yet. Ask 4-5 questions, wait for answers, then write a minimal starter DESIGN.md marked `<!-- SEED -->`. Re-run `document` later after implementation.

Questions for seed mode:

- product or brand register?
- color strategy: Restrained, Committed, Full palette, or Drenched?
- theme scene: who uses this, where, under what light and mood?
- typography tone and density?
- 2-3 references or anti-references?

## DESIGN.md frontmatter

Open DESIGN.md with YAML frontmatter. Include only tokens the project actually uses; do not invent empty scales.

```yaml
---
name: <project title>
description: <one-line design-system summary>
colors:
  primary: "#b8422e"
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
---
```

Rules:

- Token refs use `{path.to.token}`, such as `{colors.primary}` or `{rounded.md}`.
- Keep frontmatter values aligned with real CSS tokens, theme files, or component defaults.
- Color values should be hex for Stitch compatibility. If the project treats OKLCH as canonical, put the nearest hex in frontmatter and record the canonical OKLCH value in the markdown body.
- Component frontmatter should stay compact: background/text color, typography, rounded, padding, width/height/size. Put shadows, focus rings, motion, complex states, and Cheers-specific contracts in the markdown body.
- Use semantic token names from the project when they exist. Do not rename a mature system to generic Material-style keys.

## Markdown body

Use exactly these six top-level sections, in this order. Optional numbering or subtitles are fine, but each header must contain the literal section name.

```markdown
# Design System: [Project]

## 1. Overview

Creative north star, register, mood, density, and what the system rejects.

## 2. Colors

Palette tokens, semantic roles, contrast rules, status colors, dark/light behavior if any.

## 3. Typography

Font stacks, scale, weights, line-height, prose measure, data-number rules, and font-loading behavior.

## 4. Elevation

Shadow, border, tonal-layer, and depth rules. If the system is flat, say so explicitly.

## 5. Components

Buttons, links, inputs, forms, cards/containers, nav, badges, empty/error/loading states, dialogs/popovers.

## 6. Do's and Don'ts

Concrete guardrails, including no gradient text, no decorative glass, no side-stripe card accents, and no optimistic success.
```

Fold layout, spacing, motion, interaction, and state guidance into these six sections rather than adding extra top-level sections:

- layout and spacing usually belong in **Overview**, **Components**, or **Do's and Don'ts**
- motion belongs in **Components** or **Do's and Don'ts**
- backend-confirmed state and pending/error/success behavior belong in **Components** and **Do's and Don'ts**

If another tool in the project expects a specific format, preserve that format and fold these requirements into it.

Do not write a `.cheers-design/design.json` sidecar unless a Cheers tool that consumes it exists. Upstream Impeccable uses `.impeccable/design.json` for its live design-system panel; `cheers-design` does not currently have an equivalent consumer.

## Cheers-specific content to capture

Capture design-relevant implementation facts without turning DESIGN.md into an API manual:

- CSS token names used by Cheers templates
- components that depend on client behavior
- common state vocabulary for pending, error, success, empty, and permissions
- conceptual update boundaries that affect layout or motion
- reusable UI patterns and their accessibility contracts

For exact syntax and implementation rules, link or defer to the main `cheers` skill.

## Verify

The doc should be concrete enough that another agent can build a new screen without inventing colors, spacing, type, state behavior, or Cheers interaction trust rules.
