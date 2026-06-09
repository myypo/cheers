# Critique

Run a design-director review of a Cheers UI and produce prioritized feedback. Critique may inspect code and rendered output, but does not fix issues.

Critique is also the source of a reusable backlog: persist the final report under `.cheers-design/critique/` so later `polish` runs can consume prior P0/P1 findings without copy-paste.

## Inputs

Clarify the surface goal if it is not obvious. A critique without knowing the intended primary action is mostly taste.

Resolve the target to a stable file path or URL before reviewing. Prefer a source path over a dev-server URL when both identify the same surface; ports drift, paths do not.

Examples:

- "the homepage" -> `src/pages/home.rs`, `templates/home.html`, or equivalent source
- "the settings modal" -> the primary component file
- "this page" -> current URL only if no source path is identifiable

## Persistence setup

Use the helper script from this skill directory: `scripts/critique-storage.mjs`. Resolve it against the skill directory before running commands.

1. Compute the slug:

   ```bash
   node <skill-dir>/scripts/critique-storage.mjs slug "<resolved-path-or-url>"
   ```

   Keep the slug. If it exits non-zero because the target is vague or root-level, continue the critique and skip persistence.

2. Read `.cheers-design/critique/ignore.md` if it exists. Drop matching findings silently; it is the user's explicit ignore list for repeated critique noise.

## Assessment passes

When practical, run two independent passes before synthesizing:

1. **Design pass**: hierarchy, information architecture, visual quality, copy, state coverage, accessibility, responsive behavior, emotional fit.
2. **Interaction fit pass**: whether dynamic behavior stays backend-confirmed, pending states are honest, and local affordances do not pretend to be durable app state.

If browser automation is available, inspect the live UI at mobile and desktop. If not, inspect source and state the limitation.

## What to evaluate

### AI-slop and visual quality

Look for gradient text, decorative glass, side-stripe card accents, generic hero metrics, equal icon-card grids, nested cards, centered-stack defaults, category-reflex palettes, repeated tiny section kickers, default numbered section markers, oversized display type, text overflow, and vague filler copy.

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

### Register fit

For product surfaces, ask whether the interface feels trustworthy, familiar, dense enough, and efficient. For brand surfaces, ask whether it has a memorable point of view, specific imagery/type/color, and a non-template composition.

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

## Interaction trust verdict

Pass/fail for backend-confirmed success, honest pending, and local-only affordances.

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
- **P1**: major user harm or backend-confirmed trust violation.
- **P2**: meaningful quality issue.
- **P3**: polish.

## Persist the snapshot

After finalizing the report, write it to `.cheers-design/critique/` if a slug was computed.

1. Write the full report body to a temporary file. Exclude any final conversational question that is not part of the critique report.
2. Run:

   ```bash
   CHEERS_DESIGN_CRITIQUE_META='{"target":"<resolved>","score":<total>,"p0":<count>,"p1":<count>}' \
     node <skill-dir>/scripts/critique-storage.mjs write <slug> <body-file>
   ```

   The helper prints the absolute path it wrote.

3. Delete the temporary body file whether the write succeeds or fails.
4. Read trend metadata:

   ```bash
   node <skill-dir>/scripts/critique-storage.mjs trend <slug> 5
   ```

5. Add one concise line after the report:

   > Wrote `.cheers-design/critique/<filename>`. Trend for `<slug>`: 24 -> 28 -> 32.

If this is the first run, say: `First run for <slug>, no trend yet.` If persistence fails, report the error briefly and continue; the chat report remains the primary deliverable.
