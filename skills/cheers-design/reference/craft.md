# Craft

Build a confirmed design brief into production-quality Cheers UI, then inspect and refine it. Craft is where design decisions become code, but the exact Cheers mechanics come from the main `cheers` skill.

## Gate

Do not start code edits until one is true:

- The user confirmed a `shape` brief for this task.
- The user supplied an already-confirmed brief.
- The user explicitly asked to skip shaping and proceed.

Before editing, load `cheers` for implementation rules, the matching register reference, and any relevant design references such as `layout`, `typeset`, `colorize`, `harden`, `animate`, or `polish`.

Briefly state: confirmed brief status, register, primary action, interaction contract, and the fact that success remains backend-confirmed.

## Visual direction and assets capability gate

If the harness provides native image generation, screenshot comparison, or sub-agent asset production, use those capabilities before implementation when the brief is visually open-ended, image-led, brand-heavy, or production-ready:

1. Ask 2-3 targeted visual direction questions if palette, atmosphere, or references are still unresolved.
2. Produce or request 1-3 distinct visual direction probes or mocks.
3. Stop for approval or explicit delegation before coding.
4. Inventory the approved direction's major ingredients: hero silhouette, imagery/media, typography, palette, section structure, signature motifs, motion cues, and what should remain semantic HTML/CSS/SVG rather than raster.
5. Use a scoped sub-agent for asset production only when the harness supports sub-agents and the user allows it.

This flow is **not Codex-exclusive**; upstream names one file `codex.md` because Codex exposes native image generation there. In Pi, only run the generated-mock or asset-subagent steps when those capabilities are actually available. If they are unavailable, state that the visual-generation step is skipped and implement directly from the confirmed brief.

## Build passes

### 1. Re-anchor the design

Extract from the brief:

- primary user action and success condition
- visual lane, theme scene, color strategy, typography tone, density, imagery needs
- required states and edge cases
- responsive expectations
- existing design-system components or tokens to preserve
- anti-goals and generic AI tells to avoid

If these are materially unclear, stop and ask.

### 2. Design the structure before details

Sketch the code plan in design terms:

- page landmarks and heading hierarchy
- content groups and reading order
- primary, secondary, and tertiary actions
- form, table, list, detail, empty, and error patterns
- patchable or refreshable regions as conceptual boundaries
- mobile structure, not just squeezed desktop

Do not begin by styling a pile of divs. Build semantic shape first.

### 3. Implement through the Cheers skill

Use `cheers` for exact choices around `Render`, generated helpers, Datastar attributes, actions, patches, streams, tests, and formatting.

From this skill, keep the implementation aligned with design intent:

- interaction remains honest and backend-confirmed
- pending feedback is visible and does not imply success
- local client affordances stay local
- state and errors render near the user decision point
- components are extracted when they clarify repeated UI, not to abstract prematurely
- CSS follows project conventions and token vocabulary

### 4. Finish visual, content, and responsive quality

- Replace placeholder copy with real, task-specific content.
- Ensure every relevant state has layout, copy, and visual treatment.
- Tune spacing, alignment, density, type hierarchy, and color roles.
- Check hover, focus-visible, active, disabled, pending, error, and success states.
- Add purposeful motion only when it clarifies feedback, hierarchy, reveal, or transition.
- Preserve accessibility: labels, landmarks, status announcements, keyboard path, touch targets, contrast.

### 5. Inspect and iterate

When practical, inspect the rendered UI at mobile, tablet/small laptop, and desktop. Compare against:

- confirmed brief
- register reference
- design system and neighboring surfaces
- state coverage
- anti-pattern list
- performance and motion constraints

Fix material defects. Do not invent fake defects just to show iteration.

## Present

Summarize:

- files changed
- design decisions that connect to the brief
- interaction contract used
- states covered
- viewports or rendered checks performed
- formatting/tests/checks run through `cheers` guidance
- remaining limitations or follow-up risks
