# NOTICE

This skill adapts material from Impeccable.

- Original work: <https://github.com/pbakaus/impeccable>
- Copyright: 2025-2026 Paul Bakaus
- License: Apache License 2.0
- License text: [`LICENSES/impeccable-Apache-2.0.txt`](LICENSES/impeccable-Apache-2.0.txt)
- Upstream sync/base: `508d7e8955de3b3caf2d8676e85206723d41a887`
- Changes: adapted the design-skill guidance for Cheers/Rust/Datastar, added a clear boundary with the main `cheers` implementation skill, kept only design-level interaction guardrails such as backend-confirmed state and no optimistic UI, migrated setup language to `init`, added critique snapshot persistence under `.cheers-design/critique/`, and adopted Google Stitch-style `DESIGN.md` frontmatter guidance. The new-work flow, visitor modes, craft floor, and finish review are reworked to run without the upstream `impeccable` binary: direction rounds are run by hand instead of the concept-seed roll and decision page, surface briefs live under `.cheers-design/surfaces/`, the finish reviewer is a generic subagent prompt, and the `impeccable detect` scan is optional. Live mode, variant generation, image comps, hooks, doctor, and native iOS/Android guidance are not included.

Upstream Impeccable `NOTICE.md` attribution, as published at upstream commit `f636bd065a11234bfc7f1b0e0cd9d1ba7a0eb209` (the previous sync base), is reproduced below. The current upstream `NOTICE.md` only attributes the iOS and Android platform references, which this skill does not include.

```text
# Notice

Impeccable
Copyright 2025-2026 Paul Bakaus

## Anthropic frontend-design Skill

The `impeccable` skill in this project builds on Anthropic's original frontend-design skill.

**Original work:** https://github.com/anthropics/skills/tree/main/skills/frontend-design
**Original license:** Apache License 2.0
**Copyright:** 2025 Anthropic, PBC

This project extends the original with:
- 7 domain-specific reference files (typography, color-and-contrast, spatial-design, motion-design, interaction-design, responsive-design, ux-writing)
- 23 commands
- Expanded patterns and anti-patterns

## Typecraft Guide Skill

The `typography.md` reference in this project incorporates a set of tactical additions merged in from ehmo's `typecraft-guide-skill` at the author's request: dark-mode weight/tracking compensation, `font-display: optional` vs `swap`, preload-critical-weight-only guidance, variable fonts for 3+ weights, `clamp()` max-to-min ratio bound, responsive measure/container coupling, `text-wrap: balance` / `pretty`, `font-optical-sizing: auto`, ALL-CAPS tracking quantification, and the paragraph-rhythm rule (space OR indent, never both).

**Original work:** https://github.com/ehmo/typecraft-guide-skill
**Original license:** see upstream repo
**Author:** ehmo
```
