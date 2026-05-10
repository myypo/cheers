# Audit

Audit a Cheers UI without fixing it. Report actionable issues and map each issue to a Cheers/Datastar-safe remedy.

## Scan dimensions

Score each dimension 0-4.

### 1. Cheers/Datastar correctness

Check for:

- hardcoded generated action URLs, signal paths, or patch ids
- missing generated ids on structural patch targets
- `form_names!(...)`, `#[form]`, input `name=...`, and handler `Form<T>` out of sync
- broad backend state mirrored into signals
- unnecessary `#[signal(global)]`
- optimistic updates or client-side success before backend confirmation
- `PatchElements` over-targeting with selectors where default morph would work
- missing `Scripts` on pages using actions, signals, streams, or Datastar attrs
- JS used where Datastar attrs, CSS, or server patches would be enough

Score: 0 = fights the model, 4 = idiomatic Cheers/Datastar.

### 2. Accessibility and semantics

Check for:

- missing landmarks, heading hierarchy, labels, button/link semantics
- keyboard traps, hover-only controls, missing `:focus-visible`
- missing `aria-busy`, `role="alert"`, or live announcements for dynamic status where needed
- poor contrast, color-only meaning, touch targets below 44px
- images without useful alt text or decorative images not hidden

Score: 0 = inaccessible, 4 = WCAG AA-quality with strong semantics.

### 3. State and resilience

Check for:

- absent empty/loading/error/success/disabled/permission states
- validation only on the client
- user input lost after server validation errors
- double-submit/race hazards not handled with indicators/disabled state/backend idempotency
- long text, CJK, RTL, emoji, large numbers, many rows, or no data breaking layout

Score: 0 = only works on perfect data, 4 = production-resilient.

### 4. Visual/product quality

Check for:

- unclear primary action or hierarchy
- generic AI UI: gradient text, glass cards, nested cards, equal icon-card grids, hero metrics, side-stripe accents
- inconsistent component vocabulary
- poor spacing rhythm, alignment, typography scale, density, or responsive composition
- brand work without point of view or imagery when the content calls for it

Score: 0 = slop or confusing, 4 = intentional and trustworthy/distinctive.

### 5. Performance and maintainability

Check for:

- needless JS bundles or dependencies
- oversized images/fonts/assets
- uncompressed or excessively chatty streams when SSE is used
- patch payloads that are too fine-grained and complex, or too broad without reason
- expensive unbounded blur/filter/shadow/motion
- layout-thrashing JS, casual layout-property animations
- duplicated markup that should be a `Render` component

Score: 0 = severe issues, 4 = lean, maintainable, and measured.

## Report format

```markdown
## Audit Health Score

| # | Dimension | Score | Key finding |
|---|---:|---:|---|
| 1 | Cheers/Datastar correctness | ?/4 | ... |
| 2 | Accessibility and semantics | ?/4 | ... |
| 3 | State and resilience | ?/4 | ... |
| 4 | Visual/product quality | ?/4 | ... |
| 5 | Performance and maintainability | ?/4 | ... |
| **Total** |  | **?/20** | **Excellent/Good/Acceptable/Poor/Critical** |

## Optimism verdict

Pass/fail. Identify any optimistic UI or client-confirmed success.

## Top issues

- [P0/P1/P2/P3] Issue title
  - Location: file/component/line if known
  - Impact: why it matters
  - Recommendation: concrete Cheers/Datastar-safe fix
  - Suggested next command: `cheers-design harden|polish|optimize|craft|animate`

## Positive findings

What should be preserved.

## Recommended order

1. ...
```

Severity:

- **P0**: blocks task completion, data trust, or accessibility basics.
- **P1**: major user harm, WCAG AA violation, or Datastar model violation.
- **P2**: meaningful quality issue with workaround.
- **P3**: polish.

Do not bury the user in P3 noise. Prioritize model violations, accessibility, state correctness, and the biggest visual problems.
