# Critique

Run a design-director review of a Cheers UI: resolve one stable target, run two isolated assessments, synthesize one report, persist a snapshot, and ask what to improve next. Critique inspects code and rendered output but fixes nothing.

The chat report is the deliverable. The snapshot under `.cheers-design/critique/` is an archive of the run that later `polish` runs read as a backlog.

## Setup

1. **Resolve the target** to a concrete file path or URL. Prefer a source path over a dev-server URL when both identify the same surface; ports drift, paths do not.
   - "the homepage" -> `src/pages/home.rs` or the module that renders it
   - "the settings modal" -> the primary component file
   - "this page" -> the current URL only if no source path is identifiable
2. **Know the goal.** Read PRODUCT.md, DESIGN.md, and the surface brief (`.cheers-design/surfaces/<slug>.md`) when present; its direction contract is what the render must keep. If the primary action is still unclear, ask: a critique without it is mostly taste.
3. **Compute the slug:**

   ```bash
   node <skill-dir>/scripts/critique-storage.mjs slug "<resolved-path-or-url>"
   ```

   Never hand-write a slug. If it exits non-zero (the target has no stable slug: empty input or the project root), continue the critique and skip persistence and trend.
4. **Read `.cheers-design/critique/ignore.md`** if it exists and drop matching findings silently. It is the only prior-run input critique consumes; do not read earlier snapshots before assessing.

## Visitor mode

Take the mode from the surface brief, or choose it from the requested surface per SKILL.md.

- **Persuade + Experience:** judge commitment and specificity. Does the first viewport make the offer intelligible, the action findable, and the product's proof visible within seconds (Persuade), or let the work lead (Experience)? A clean but category-default page is a finding, not a pass. Heuristics 7 and 10 may score `n/a`.
- **Operate + Read:** judge earned familiarity per [operate.md](operate.md): can a category-fluent user trust every control, density, and term without pausing? Task speed, state vocabulary, and consistency outrank expression; on Read, measure, wayfinding, and the reading column come first.

## Two isolated assessments

When a subagent tool is exposed, run A and B as two isolated subagents in parallel, each with a self-contained prompt: cwd, target, dev-server URL, the reference paths it needs, product context, and the output contract below. They must not see each other's output, and no findings reach the user before synthesis. This is mandatory: running them inline is a degraded run and is not permitted for speed or convenience. Subagents are unavailable only when no subagent tool is exposed, or when the harness asks permission to spawn and the user declines; name that reason. Without subagents, finish and record Assessment A before starting B, and make the report's first line `Degraded: single-context (<reason>)`. A silent degraded run is a failed run.

When browser automation is available, a viewable target requires inspecting the running app at desktop (1440 wide) and mobile (390 wide) in one batched round; each assessment opens its own fresh tab. Prefer the harness's native browser or screenshot tool; hand-write a Playwright or Puppeteer script only when no native tool is exposed. Without a browser, review source and state the limitation. Any server started only for the critique runs in the background and is stopped before reporting unless the user asks to keep it.

### Assessment A: design review

Think like a design director. Answer with rendered or source evidence:

- **Design specificity:** is the composition, interaction, and visual language grounded in this product, or could an unrelated product use it unchanged? Judge this before any scan output exists.
- **Holistic design:** hierarchy, information architecture, discoverability, composition, typography, color, copy, accessibility, responsive structure, states, and edge cases. Hold the render against [craft-floor.md](craft-floor.md)'s Refuse list and Verify checks.
- **Interaction trust:** success, removal, completion, and reordering appear only after backend confirmation; pending states say what is happening without implying it is done; signals stay local affordances and never pose as durable state; the dynamic layer is the smallest one that works.
- **Cognitive load:** the checklist below; name every failed item and every decision point with more than four visible options.
- **Emotional journey:** the peak and the end, emotional valleys, and reassurance at high-stakes moments.
- **Heuristics:** score all ten, marking `n/a` only where the mode rule allows.
- **Personas:** walk two or three through the primary action.

Return: specificity verdict, interaction trust verdict, heuristic scores, cognitive load, emotional journey, 2-3 strengths, 3-5 priority issues, persona red flags, minor observations, and provocative questions.

### Assessment B: mechanical scan

Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) against the rendered page, once at desktop and once with `--viewport 390x844`; for a multi-view target, scan three to five representative pages. When the tool is unavailable, run the manual check SKILL.md describes and say so. Never block on it.

Return: finding counts, rule names, locations, suspected false positives, and any skipped step with its concrete reason. The parent reuses these findings; rerun only when B failed, was truncated, or omitted counts, rules, or locations.

## Heuristics

Score each 0-4: 0 absent or hostile, 1 rare and mostly failing, 2 partial with major gaps, 3 good with minor gaps, 4 genuinely excellent. Most real interfaces land between 20 and 32 of 40.

| # | Heuristic | Check for |
|---|---|---|
| 1 | Visibility of system status | honest pending and progress, backend-confirmed outcomes, current location, inline validation |
| 2 | Match with the real world | the user's terms, natural order, recognizable icons and metaphors |
| 3 | User control and freedom | cancel, back, undo where the backend supports it, clearing filters and selections, escape from multi-step flows |
| 4 | Consistency and standards | one term per concept, same action same result, standard web controls, one component vocabulary |
| 5 | Error prevention | confirmation before destructive actions, constrained inputs, smart defaults, drafts that survive |
| 6 | Recognition over recall | visible options, labeled icons, inline hints, recent items |
| 7 | Flexibility and efficiency | keyboard paths, bulk actions, accelerators that stay out of a novice's way |
| 8 | Aesthetic and focused design | only what the step needs, clear hierarchy, purposeful emphasis, no decorative clutter |
| 9 | Error recovery | plain language, the exact problem, the recovery, errors near their source, input preserved |
| 10 | Help and documentation | contextual, task-focused, concise help reachable without leaving the task |

**Mode applicability:** on Persuade and Experience surfaces, heuristics 7 and 10 may be `n/a`, as may any heuristic that genuinely cannot apply. Write `n/a` with a one-line reason and renormalize: the applicable maximum is 4 times the number scored (/32 when two are `n/a`). Never print /40 over a partial set.

**Bands:** 36-40 Excellent, 28-35 Good, 20-27 Acceptable, 12-19 Poor, 0-11 Critical. With `n/a` items, read the band from the percentage: 90% Excellent, 70% Good, 50% Acceptable, 30% Poor, below that Critical.

## Cognitive load

Intrinsic load comes from the task: structure it with steps, defaults, and progressive disclosure. Extraneous load comes from the design: eliminate it. Germane load is learning that pays off: support it with consistent patterns and confirming feedback.

Checklist; count the failures (0-1 low, 2-3 moderate, 4+ critical):

- **Single focus:** the primary task is free of competing elements.
- **Chunking:** information comes in groups of four or fewer.
- **Grouping:** related items sit together by proximity or shared ground.
- **Hierarchy:** the most important thing is obvious at once.
- **One thing at a time:** one decision before the next.
- **Minimal choices:** four or fewer visible options per decision point.
- **Working memory:** nothing must be remembered from a previous screen.
- **Progressive disclosure:** complexity appears only when needed.

Working memory holds about four items: 5-7 at one decision point pushes the limit, 8+ overloads it. In practice: one primary action with one or two secondaries and the rest grouped; five or fewer top-level nav items; one reading path per article with related links gathered at the end; four or fewer sibling choices per docs-sidebar level; one decision per gallery screen.

Name violations by pattern: wall of options, memory bridge between steps, hidden location, jargon barrier, flat visual noise floor, inconsistent pattern, simultaneous demands, context switch for a single decision.

## Personas

Pick two or three by surface; walk each through the primary action and name exactly what breaks. Never write generic persona descriptions.

- **Alex, power user:** skips onboarding, wants keyboard paths and bulk actions. Red flags: forced tutorials, no keyboard route to the primary action, slow unskippable motion, one-at-a-time work where batch is natural, confirmations on low-risk actions.
- **Jordan, first-timer:** reads everything, takes labels literally. Red flags: icon-only navigation, unexplained jargon, no visible help, ambiguous next step, no confirmation that an action succeeded.
- **Sam, keyboard and screen-reader user:** tabs linearly, relies on headings and labels, zooms to 200%. Red flags: click-only interactions, invisible focus, color-only meaning, unlabeled controls, patched regions that change without a status announcement, time limits without extension.
- **Riley, stress tester:** empty and huge data, long strings, emoji and RTL, refresh mid-flow, two tabs. Red flags: features that appear to work but fail silently, errors that leak internals or break the UI, empty states with no guidance, input lost on refresh or validation error, UI that claims success the backend did not confirm.
- **Casey, distracted mobile user:** one thumb, interrupted, slow connection. Red flags: key actions out of thumb reach, progress lost on interruption, typing where selection would do, heavy assets on every page, targets under 44px or crowded together, custom drag or scroll controls that fail under touch.

| Surface | Personas |
|---|---|
| Landing page, marketing | Jordan, Riley, Casey |
| Dashboard, admin, data-heavy | Alex, Sam |
| Checkout | Casey, Riley, Jordan |
| Onboarding | Jordan, Casey |
| Forms, wizards | Jordan, Sam, Casey |

When PRODUCT.md confirms an audience the five do not cover, add one or two project personas (profile, behaviors, red flags) from it. Never invent audience details.

## Report format

Synthesize; do not concatenate. Say where the design review and the scan agree, what the scan caught alone, and which scan findings are false positives.

```markdown
Method: dual-agent (A: <agent-id> · B: <agent-id>) | Degraded: single-context (<reason>)

## Design Health Score

| # | Heuristic | Score | Key issue |
|---|---|---|---|
| 1 | Visibility of system status | ?/4 | ... |
| ... | ... | ... | ... |
| **Total** |  | **?/<applicable max>** | **<band>** |

## Design specificity verdict

Start here. Authored for this product, or category-interchangeable? The unanchored design judgment first, then the scan summary with counts, locations, and false positives.

## Interaction trust verdict

Pass or fail for backend-confirmed outcomes, honest pending, and local-only affordances, with the evidence.

## Overall impression

What works, what does not, and the single biggest opportunity.

## What's working

2-3 specific strengths and why they work.

## Priority issues

3-5, most impactful first.

- [P0-P3] Issue
  - Location:
  - Why it matters:
  - Fix:
  - Suggested command: `cheers-design <command>`

## Persona red flags

Per persona, the exact elements and interactions that fail.

## Minor observations

## Questions to consider

Provocative questions that might unlock a better solution.
```

Severity:

- **P0 blocking:** prevents the core task, breaks data trust, or blocks access.
- **P1 major:** significant difficulty, a WCAG AA violation, or a backend-confirmed trust violation. If a user would contact support about it, it is at least P1.
- **P2 minor:** an annoyance with a workaround.
- **P3 polish:** no real user impact.

Be direct and specific ("the save button", not "some elements"), say why it matters to users, give concrete fixes, prioritize ruthlessly, and do not soften criticism.

## Deliver, persist, close

1. **Deliver.** Write the full report into the chat response before any persistence. A report that exists only in a snapshot file was never delivered.
2. **Persist** when a slug was computed. Write a copy of the delivered report, from the Method line through Questions to consider, to a temporary file, then:

   ```bash
   CHEERS_DESIGN_CRITIQUE_META='{"target":"<resolved>","score":<total>,"max_score":<applicable max>,"na_heuristics":"<e.g. 7,10 or empty>","p0":<count>,"p1":<count>}' \
     node <skill-dir>/scripts/critique-storage.mjs write <slug> <body-file>
   ```

   The helper prints the path it wrote. Delete the temporary file whether the write succeeds or fails; if deletion fails, mention `temp-file cleanup failed: <reason>` without blocking. The helper does not track whether the target changes afterward; polish verifies findings against current code.
3. **Read the trend:** `node <skill-dir>/scripts/critique-storage.mjs trend <slug> 5`. Add one line after the report and before the questions:

   > Wrote `.cheers-design/critique/<filename>`. Trend for `<slug>`: 24 -> 28 -> 32 (out of 40).

   Treat a missing `max_score` as 40. When entries differ in maximum, print each with its own denominator (`24/32 -> 30/40`) and note the runs are not like-for-like. On a first run say `First run for <slug>, no trend yet.` If persistence fails, report the error in one line and continue. Never show the helper's JSON.
4. **Close with questions.** In the same message, after everything else, ask two to four targeted questions through the structured question tool when available, each tied to specific findings with two or three concrete options:
   - **Priority direction:** which of the top two or three issue categories to tackle first.
   - **Design intent:** whether a tonal mismatch the critique found was intentional, offering directions that would fix it.
   - **Scope:** top three, all issues, or P0/P1 only.
   - **Constraints**, only when findings span many areas: what must stay as is.

   Skip the questions only when the report lists fewer than three priority issues, and then print the literal line `Questions skipped: <reason>` naming the count. A run that ends with neither questions nor that line is incomplete.

## Recommended actions

After the user answers, list commands in priority order, by stated priority first and impact second:

1. **`cheers-design <command>`:** what to fix, with enough context from the findings that the command knows where to focus.

Map every priority issue to a command, skip commands that address nothing, respect the chosen scope and off-limits areas, and end with `cheers-design polish` when any fixes were recommended. Then tell the user they can run these one at a time, all at once, or in any order, and re-run `cheers-design critique` after fixes to see the score move.
