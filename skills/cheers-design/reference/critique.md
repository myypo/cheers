# Critique

Run a design-director review of a Cheers UI and produce prioritized feedback. Critique may inspect code and rendered output, but does not fix issues.

## Inputs

Clarify the surface goal if it is not obvious. A critique without knowing the intended primary action is mostly taste.

## Assessment passes

When practical, run two independent passes before synthesizing:

1. **Design pass**: hierarchy, information architecture, visual quality, copy, state coverage, accessibility, responsive behavior, emotional fit.
2. **Cheers/Datastar pass**: state ownership, actions, patches, generated ids/forms, signal use, no optimistic UI, loading/error handling, Scripts inclusion.

If browser automation is available, inspect the live UI at mobile and desktop. If not, inspect source and state the limitation.

## What to evaluate

### AI-slop and visual quality

Look for gradient text, decorative glass, side-stripe card accents, generic hero metrics, equal icon-card grids, nested cards, centered-stack defaults, and vague filler copy.

### Heuristics

Score 0-4:

1. Visibility of system status.
2. Match with user language and mental model.
3. User control and recovery.
4. Consistency and standards.
5. Error prevention.
6. Recognition over recall.
7. Efficiency for frequent users.
8. Aesthetic and focused design.
9. Error recovery.
10. Help and onboarding.

### Cheers/Datastar model

Flag:

- optimistic UI
- broad backend state mirrored into signals
- missing generated ids for patch targets
- hardcoded generated URLs or ids
- overuse of selectors where default morph would work
- missing `Scripts`
- client-only validation for server-owned actions
- unnecessary JS bundles

### Persona red flags

Pick 2-3 relevant personas from context, such as:

- First-timer: needs orientation and clear next step.
- Power user: needs efficient navigation and predictable controls.
- Keyboard/screen-reader user: needs semantics, labels, focus, status announcements.
- Mobile user: needs touch targets and structural adaptation.
- Low-trust/high-stakes user: needs confirmation, reversibility, precise errors.

Do not write generic persona descriptions. Walk each through the primary action and name what breaks.

## Report format

```markdown
## Design Health Score

| # | Heuristic | Score | Key issue |
|---|---:|---:|---|
| 1 | Visibility of system status | ?/4 | ... |
| ... | ... | ... | ... |
| **Total** |  | **?/40** | **Excellent/Good/Acceptable/Poor/Critical** |

## Anti-pattern verdict

Does this look generated or generic? What specific tells prove it?

## Datastar verdict

Pass/fail for backend-owned truth, sparse signals, generated actions/ids/forms, honest loading, and no optimistic UI.

## Overall impression

One paragraph.

## What's working

2-3 concrete strengths.

## Priority issues

- [P0/P1/P2/P3] Issue
  - Location:
  - Why it matters:
  - Fix:
  - Suggested command: `cheers-design ...`

## Persona red flags

...

## Questions to decide

2-4 targeted questions only if needed.

## Recommended actions

Prioritized command list.
```

Severity:

- **P0**: blocks core task, data trust, or accessibility.
- **P1**: major user harm or Datastar model violation.
- **P2**: meaningful quality issue.
- **P3**: polish.
