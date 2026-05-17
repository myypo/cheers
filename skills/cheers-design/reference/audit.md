# Audit

Audit a Cheers UI without fixing it. Report actionable issues across UX, visual quality, state trust, accessibility, and implementation fit. Use the main `cheers` skill for exact technical diagnosis when needed.

## Scan dimensions

Score each dimension 0-4.

### 1. Purpose, flow, and information architecture

Check for:

- unclear primary action or success condition
- confusing entry path, reading order, or navigation
- too many competing actions or modal-first flows
- hidden dependencies, poor recovery, or unclear next step
- empty/first-run states that fail to orient the user

Score: 0 = confusing or blocks completion, 4 = obvious, efficient, and confidence-building.

### 2. Visual craft and design-system fit

Check for:

- weak hierarchy, spacing rhythm, alignment, density, or responsive composition
- generic AI tells: gradient text, decorative glass, nested cards, side stripes, equal icon-card grids, hero metrics
- inconsistent component vocabulary or token use
- typography that is flat, illegible, overflowing, or off-register
- brand work without a point of view or product UI that feels unfamiliar without reason

Score: 0 = slop or incoherent, 4 = intentional, polished, and system-aligned.

### 3. Interaction trust and state coverage

Check for:

- optimistic success, removal, completion, or reordering before backend confirmation
- absent loading, pending, error, success, disabled, permission, overflow, or long-data states
- validation/recovery that loses user input or hides next steps
- pending feedback that is invisible, ambiguous, or too celebratory
- local affordances that feel like durable state

Score: 0 = untrustworthy or fragile, 4 = complete, honest, and resilient.

### 4. Accessibility and semantics

Check for:

- missing landmarks, heading hierarchy, labels, button/link semantics
- keyboard traps, hover-only controls, missing focus-visible states
- missing status announcements for important dynamic changes
- poor contrast, color-only meaning, touch targets below 44px
- images without useful alt text or decorative images not hidden

Score: 0 = inaccessible, 4 = WCAG AA-quality with strong semantics.

### 5. Performance and maintainability as experienced by users

Check for:

- slow initial load, heavy assets, excessive fonts, or layout shift
- overcomplicated dynamic behavior for a simple task
- janky motion, unbounded blur/filter/shadow, or layout-property animation
- too many tiny updates or huge updates that make state hard to reason about
- duplicated UI patterns that create design-system drift

Score: 0 = severe user-visible cost, 4 = lean, measured, and maintainable.

## Report format

```markdown
## Audit Health Score

| # | Dimension | Score | Key finding |
|---|---:|---:|---|
| 1 | Purpose, flow, and IA | ?/4 | ... |
| 2 | Visual craft and system fit | ?/4 | ... |
| 3 | Interaction trust and states | ?/4 | ... |
| 4 | Accessibility and semantics | ?/4 | ... |
| 5 | Performance and maintainability | ?/4 | ... |
| **Total** |  | **?/20** | **Excellent/Good/Acceptable/Poor/Critical** |

## Trust verdict

Pass/fail for backend-confirmed success, honest pending states, and no optimistic UI.

## Top issues

- [P0/P1/P2/P3] Issue title
  - Location: file/component/route if known
  - Impact: why it matters
  - Recommendation: concrete design fix, plus Cheers implementation area if relevant
  - Suggested next command: `cheers-design harden|polish|optimize|craft|animate`

## Positive findings

What should be preserved.

## Recommended order

1. ...
```

Severity:

- **P0**: blocks task completion, data trust, or accessibility basics.
- **P1**: major user harm, WCAG AA violation, or backend-confirmed trust violation.
- **P2**: meaningful quality issue with workaround.
- **P3**: polish.

Do not bury the user in P3 noise. Prioritize user harm, accessibility, trust, state coverage, and the biggest visual problems.
