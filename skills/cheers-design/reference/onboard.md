# Onboard

Design first-run, empty, help, and activation flows that get users to value quickly in a Cheers app. Onboarding's job is not to teach the product; it is to reach the moment that proves the product is worth the user's time.

## Visitor mode

- **Persuade and Experience:** a welcome or activation moment may carry the committed world at full strength, as long as the next action stays obvious and working.
- **Operate and Read:** onboarding lives inside the real product surfaces. Familiar affordances, stable layout, and the task outrank ceremony.

Preserve the established visual world, copy, and behavior outside the flow. Identity replacement belongs to [new-work.md](new-work.md).

## Two isolated assessments

Run these independently when a subagent is available; otherwise run them in this order.

1. **Design assessment:** answer each with rendered or source evidence:
   - What is the aha moment, and how many steps stand between a new user and it?
   - Who arrives: experience level, motivation, time available, what they came from?
   - Where do users stall, drop off, or meet a blank screen?
   - Which empty states occur (first use, user cleared, no results, no permission, failed load), and does each say what goes here, why it matters, and what to do?
   - What can experienced users skip, and is the skip visible?
   - What sample data, templates, or defaults can the backend provide? What trust gaps appear before the first action?
2. **Mechanical scan:** run the [mechanical scan](../SKILL.md#mechanical-scan-optional) over the first-run and empty-state pages. Keep its results out of the first assessment, then synthesize both before editing.

## State the flow before editing

Name the aha moment, the minimum a user must learn to reach it, the first action, what is skippable, and which steps or progress markers depend on backend state. When the product tracks it, name the success signal: completion, skip, drop-off, or time to value. Then read [craft-floor.md](craft-floor.md).

## Apply

### Principles

- **Show, don't tell.** Use real product surfaces and working actions, not a separate tutorial mode, unless the domain is high-stakes enough to need a sandbox.
- **Optional when possible.** Never block access to the product; experienced users can skip.
- **Time to value.** Front-load the 20% that delivers 80% of the value; leave advanced features to contextual discovery.
- **Context over ceremony.** Teach a feature when the user meets it. Empty states are onboarding.
- **Respect intelligence.** Be brief; do not explain standard patterns.

### Empty states

Every empty state answers what will appear here, why it matters, and the first action. Where it helps, add an illustration or icon rather than text alone, and a contextual help link. Match the kind:

- **First use:** emphasize value; offer a template or example.
- **User cleared:** light touch; make recreating easy.
- **No results:** suggest another query or a way to clear filters.
- **No permission:** explain why and how to get access.
- **Failed load:** say what happened and offer a retry.

Do not fake real objects; label examples and templates as such.

### First run and first success

- Welcome: what the product is, what the user will accomplish, an honest time estimate, and a skip.
- Setup: collect the minimum, explain why each field is asked, and use smart defaults.
- Introduce one to three core concepts by doing, not reading.
- Keep the first action small and reversible when possible. Show an honest pending state while the work runs, and move to the confirmed created item or next step only after the backend confirms success. Celebrate briefly.

### Contextual help, tours, and announcements

- Prefer inline help, `details`/`summary`, popovers, and small disclosures at the point of use.
- Help patterns: keyboard-shortcut hints on the controls they trigger, searchable help, and "Learn more" links from tooltips and complex features.
- Progressive discovery: badge new or unused features and unlock complexity gradually instead of showing every option at once.
- Tours only for complex or changed interfaces: three to seven steps, interactive with real controls, workflow-framed ("create a project", not "this is the project button"), skippable, replayable from help, never blocking the whole UI.
- Interactive tutorials only when users need hands-on practice: sample data, a clear objective, validation that the user did it, and a graduation moment.
- Announcements say what is new and why it matters, let users try it immediately, and dismiss cleanly.

### Progress and dismissal

- Steps that represent account or data state complete only after backend confirmation. Never mark one done optimistically. Tour steps and device-local hints are local affordances.
- Durable completion and dismissals that matter across sessions or devices live in backend user state. Device-local hint visibility is acceptable for non-critical tips and is a local affordance only.
- Never show the same onboarding twice; returning users do not see initial onboarding again.

Use the main `cheers` skill for exact action, patch, and test mechanics.

## Avoid

- Forced long tours before users can work, or a hidden skip.
- Repeated tooltips that ignore dismissal.
- Modal-first teaching.
- Information dumped upfront instead of disclosed progressively.
- Patronizing explanations of obvious controls.

## Verify

Check with evidence in one batched round (desktop and mobile together), fix what it shows in one batch, and confirm with at most one more round:

- a new user reaches first value in the stated number of steps; an experienced user can skip at every step;
- every empty state kind present answers what, why, and the first action;
- no account or data step, check mark, or progress value appears before the backend confirms it, and pending never reads as done;
- errors are recoverable in place, with input preserved;
- the flow works with keyboard and screen reader, on touch (targets usable, nothing hover-only, no gesture as the only path), on a slow network, and with reduced motion;
- the final mechanical scan has no unexplained findings.

Format changed templates with `cargo cheers fmt --rustfmt <files>`. When the flow holds, hand off to `cheers-design polish`.
