# Document

Create or refresh `DESIGN.md` for a Cheers app so future UI work stays visually consistent. This is the visual companion to `PRODUCT.md`.

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

## Interaction and State

Default, hover, focus, active, disabled, pending, error, success, empty, permissions, and backend-confirmed trust rules.

## Do's and Don'ts

Concrete guardrails, including no gradient text, no decorative glass, no side-stripe card accents, no optimistic success.
```

If another tool in the project expects a specific format, preserve that format and fold these sections into it.

## Sidecar data

If helpful, also write `.cheers-design/design.json` with extracted tokens and component metadata. Do not duplicate values unless it serves tooling. Prefer `DESIGN.md` as the human-readable source.

## Cheers-specific content to capture

Capture design-relevant implementation facts without turning DESIGN.md into an API manual:

- CSS token names used by Cheers templates
- components that depend on client behavior
- common state vocabulary for pending, error, success, empty, and permissions
- conceptual update boundaries that affect layout or motion
- reusable UI patterns and their accessibility contracts

For exact syntax and implementation rules, link or defer to the main `cheers` skill.

## Verify

The doc should be concrete enough that another agent can build a new screen without inventing colors, spacing, type, or state behavior.
