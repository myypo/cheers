# Document

Create or refresh `DESIGN.md` for a Cheers app so future UI work stays visually consistent. This is the visual companion to `PRODUCT.md`.

## Before writing

If `DESIGN.md` exists, do not overwrite it silently. Read it and ask whether to merge, refresh, or replace.

Scan the project first:

- global CSS and `include_css!` files
- CSS custom properties and token files
- shared `Render` components
- buttons, forms, cards, nav, empty/error states
- `include_svg_sprite!` and image assets
- layout/base components and page shells
- rendered output if practical

## Modes

### Scan mode

Use when code exists. Extract actual visual system values and component behavior from the repo.

### Seed mode

Use when the app has little or no UI yet. Ask 4-5 questions, wait for answers, then write a minimal starter DESIGN.md marked `<!-- SEED -->`. Re-run document later after implementation.

Questions for seed mode:

- product or brand register?
- color strategy: Restrained, Committed, Full palette, or Drenched?
- theme scene: who uses this, where, under what light and mood?
- typography tone and density?
- 2-3 references or anti-references?

## Recommended DESIGN.md structure

Use this structure unless the project already has a preferred format:

```markdown
# Design System: [Project]

## Overview

Creative north star, register, mood, density, and what the system rejects.

## Colors

Palette tokens, semantic roles, contrast rules, status colors, dark/light behavior if any.

## Typography

Font stacks, scale, weights, line-height, prose measure, data-number rules.

## Layout and Spacing

Spacing scale, grid/flex patterns, responsive behavior, container rules.

## Components

Buttons, links, inputs, forms, cards/containers, nav, badges, empty/error/loading states, dialogs/popovers.

## Motion

Durations, easing, reduced-motion behavior, allowed motion purposes.

## Do's and Don'ts

Concrete guardrails, including no gradient text, no decorative glass, no side-stripe card accents, no optimistic UI, generated ids/actions/forms for Cheers interactions.
```

If another tool in the project expects Google Stitch-style six sections, preserve that format and fold Layout/Motion into the nearest allowed sections.

## Sidecar data

If helpful, also write `.cheers-design/design.json` with extracted tokens and component metadata. Do not duplicate values unless it serves tooling. Prefer `DESIGN.md` as the human-readable source.

## Cheers-specific content to capture

- CSS token names used by Cheers templates.
- Which components require `Scripts` because they use Datastar behavior.
- Common generated id/form/signal patterns.
- Patchable component boundaries.
- Loading/error/success state vocabulary.

## Verify

The doc should be concrete enough that another agent can build a new screen without inventing colors, spacing, type, or state behavior.
