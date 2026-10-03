# Audit

Run systematic technical quality checks on a Cheers UI and report them without fixing anything; other commands address the findings. This is a code-level audit of what is measurable and verifiable in the implementation, not a design critique: hierarchy, taste, and persona walkthroughs belong to `critique`. Use the main `cheers` skill for exact technical diagnosis.

## Visitor mode

- **Persuade + Experience:** the committed world's expensive effects (large imagery, scroll moments, canvas or WebGL) are legitimate; audit their performance budget, reduced-motion alternative, and fallback, not their existence.
- **Operate + Read:** consistency is measurable: one component and state vocabulary, token use, keyboard paths, tabular numerals in data, a real reading measure on Read surfaces.

## Two isolated assessments

Run these yourself in this order, or as independent subagents when the harness can spawn them. Keep scan output out of the first assessment, then synthesize.

1. **Technical inspection.** Read the templates, stylesheets, tokens, and any static JS helpers, and inspect the running app at desktop and mobile in one batched round when a browser is available. Score the five dimensions below with file, selector, or computed-value evidence.
2. **Mechanical scan.** Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) over the rendered pages. Verify each finding in context; keep deterministic findings separate from judgment and call out false positives.

## Dimensions

Score each 0-4.

### 1. Accessibility and semantics

- contrast below 4.5:1 (or 7:1 for AAA) for body and placeholder text, 3:1 for large text; color-only meaning
- missing landmarks, broken heading hierarchy, `div`s instead of buttons or links, unlabeled inputs, missing required indicators, weak error messaging
- custom controls missing a label, role, or state (`aria-expanded`, `aria-pressed`, `aria-selected`)
- keyboard traps, illogical tab order, hover-only controls, missing focus-visible states
- patched or streamed regions that change without a status announcement
- missing or useless alt text; decorative images not hidden
- motion sensitivity: `prefers-reduced-motion` needs an intentional alternative that preserves state change and hierarchy; flag a global `0.01ms` kill that destroys useful feedback, flashing above threshold, and motion that blocks focus, reading, or task completion

Score: 0 fails WCAG A, 1 major gaps, 2 partial effort, 3 WCAG AA mostly met, 4 AA fully met and approaching AAA.

### 2. Performance

- heavy images without dimensions, lazy loading, or responsive sources; excess font files and weights; layout shift
- layout-property animation, unbounded blur, filter, or shadow effects, visibly dropped frames
- `will-change` applied broadly or left on at rest; it is a targeted hint for a known expensive animation, not a baseline
- layout thrashing in static JS helpers (reading and writing layout in loops); JS shipped where CSS or a native browser feature would do
- an oversized dynamic layer: many tiny patches where one region would do, huge patches that replace stable UI, streams for one-shot work

Score: 0 severe user-visible cost, 1 major problems, 2 partial optimization, 3 mostly lean, 4 fast and lean.

### 3. Responsive and touch

- fixed widths that break narrow viewports; horizontal scroll; layouts that break at 200% text size; no structural mobile variant
- touch targets under 44x44px
- broken touch interaction: custom sliders, drag surfaces, and scrollable control strips whose primary gesture fails under touch, that swallow page scroll or lose the drag to it, or that stay stuck after an interrupted gesture. Code tells: mouse-only handlers, no `touch-action` on a pointer-event drag surface, drag state that nothing clears on cancel, lost capture, or blur. Exercise the gesture when a browser tool can synthesize touch (a rendered viewport proves layout, not the gesture), then say what produced the evidence (emulated viewport, synthesized touch, which engine, physical device) and what stayed untested

Score: 0 desktop-only, 1 major failures, 2 works with rough edges, 3 minor target or overflow issues, 4 every viewport works and gestures work under touch.

### 4. Interaction trust and state coverage

- optimistic success, removal, completion, or reordering before backend confirmation
- absent loading, pending, error, success, disabled, permission, overflow, or long-data states
- validation that loses user input or hides the next step
- pending feedback that is invisible, ambiguous, or already celebratory
- signals mirroring broad backend state, or local affordances that read as durable state

Score: 0 untrustworthy or fragile, 4 complete, honest, and resilient.

### 5. Implementation integrity

- hard-coded colors and spacing that bypass tokens; wrong or mixed token types; dark or alternate themes with missing variants, poor contrast, or values that do not update
- repeated implementation shortcuts and design-system drift: the same pattern built several ways, shared components bypassed
- craft-floor Refuse-list elements in the shipped UI, and misleading or decorative content standing in for real content
- structure interchangeable with an unrelated product

Score: 0 systemic drift, 1 major repeated failures, 2 several verified issues, 3 minor isolated issues, 4 coherent and intentional.

## Report format

```markdown
## Audit Health Score

| # | Dimension | Score | Key finding |
|---|---|---|---|
| 1 | Accessibility and semantics | ?/4 | ... |
| 2 | Performance | ?/4 | ... |
| 3 | Responsive and touch | ?/4 | ... |
| 4 | Interaction trust and states | ?/4 | ... |
| 5 | Implementation integrity | ?/4 | ... |
| **Total** |  | **?/20** | **<band>** |

## Implementation integrity verdict

Start here. Pass or fail: does the implementation express a coherent, product-specific system? Cite verified evidence and scan findings, with false positives named.

## Trust verdict

Pass or fail for backend-confirmed outcomes, honest pending states, and no optimistic UI.

## Executive summary

Score and band, issue counts by severity, the top 3-5 issues, next steps.

## Findings by severity

- [P0-P3] Issue
  - Location: file, component, route
  - Category: one of the five dimensions
  - Impact: what it costs users
  - Standard: the WCAG criterion or standard, if any
  - Recommendation: the concrete fix, naming the Cheers implementation area when relevant
  - Suggested command: `cheers-design <command>`

## Systemic patterns

Recurring problems that point to a gap rather than a one-off, such as "hard-coded colors in 15 templates" or "touch targets under 44px throughout mobile".

## Positive findings

Practices to keep and replicate.

## Recommended actions

In priority order: P0 first, then P1, then P2.

1. **[P?] `cheers-design <command>`:** what to fix, with context from the findings.
```

Bands: 18-20 Excellent, 14-17 Good, 10-13 Acceptable, 6-9 Poor, 0-5 Critical.

Severity:

- **P0 blocking:** prevents task completion, breaks data trust, or blocks access.
- **P1 major:** significant difficulty, a WCAG AA violation, or a backend-confirmed trust violation.
- **P2 minor:** an annoyance with a workaround.
- **P3 polish:** no real user impact.

Explain every issue's impact, keep recommendations specific, verify before reporting, and do not bury the user in P3 noise. Map findings to the commands that fix them (`harden`, `optimize`, `adapt`, `clarify`, `layout`, `typeset`, `colorize`, `animate`, and so on), and end with `cheers-design polish` when any fixes were recommended. Then tell the user they can run these one at a time, all at once, or in any order, and re-run `cheers-design audit` after fixes to see the score move.
