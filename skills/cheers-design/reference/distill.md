# Distill

Strip a Cheers UI to its essence. Remove what does not earn its place: redundant elements, repeated information, decorative noise, cosmetic complexity. Simplicity removes obstacles between users and their goals, not necessary capability.

## Visitor mode

- **Persuade + Experience:** distill toward one message and one action per section. Keep the world and its signature move; cut what dilutes them.
- **Operate + Read:** distill toward the task path and the reader's question. Match complexity to the real task: an expert workflow stays as capable as it needs to be.

Distilling works inside the established world. Cutting so deep that the identity changes is a redesign and routes through [new-work.md](new-work.md).

## Two isolated assessments

Run these in order, and do not let scan findings anchor the design assessment. With subagents available, run them independently.

1. **Design assessment:** find the complexity sources, with rendered or source evidence:
   - too many elements: competing primary actions, redundant information, clutter;
   - excessive variation: colors, fonts, sizes, and component variants without purpose;
   - information overload: everything visible at once, no progressive disclosure;
   - visual noise: borders, shadows, backgrounds, wrappers, nested cards;
   - confusing hierarchy: unclear what matters most;
   - feature creep: too many options, paths, or steps;
   - interaction weight: signals, JS helpers, or several coordinated update regions carrying complexity that a form, a page, or the rendered state could carry.

   Then find the essence: the one primary user goal, what is necessary versus nice to have, and what can be removed, deferred, or combined. If the goal is unclear from the code, briefs, and PRODUCT.md, ask rather than guess.
2. **Mechanical scan:** run the [mechanical scan](../SKILL.md#mechanical-scan-optional) over the target page.

Synthesize both before editing.

## State the system

Before editing, name the core purpose, the essential elements, what moves behind disclosure, what consolidates, and what is removed. Removing a capability the product offers is a product decision: confirm it with the user first.

## Apply

Read [craft-floor.md](craft-floor.md) first.

### Information architecture

- One primary action per surface, few secondary actions, everything else tertiary or disclosed.
- Hide advanced or rare controls behind a clear entry point: native `<details>`, a secondary page, or a step. Not a modal by reflex.
- Merge duplicate concepts, terms, buttons, and forms. If it is said elsewhere, do not repeat it here.
- Prefer recognition over recall: show current state and the next step.

### Visual structure

- Reduce to the world's committed color roles (on a Restrained surface, one or two plus neutrals); one family, three or four sizes, two or three weights.
- Remove decorative containers. Replace nested cards with spacing, headings, and dividers; use one spacing scale.
- Pick left or center alignment and stick with it.
- Prefer a linear flow over complex grids or sidebars where the content allows.
- Keep enough hierarchy that the primary action stays obvious.

### Interaction

- Prefer normal navigation and forms where dynamic behavior adds little.
- Prefer one understandable update region over several coordinated changes.
- Use smart defaults; ask only when necessary. Can the flow lose a step?
- Keep signals only where they reduce cognitive load; remove JS helpers that no longer earn their place.

### Copy

Cut every sentence in half, then do it again. Active voice ("Save changes", not "Changes will be saved"), plain language, no headers restating intros, no marketing fluff or hedging. Preserve specific labels, hints, and error recovery copy.

### Code

Remove dead CSS, unused components, and orphaned helpers. Flatten deep template nesting and collapse component variants that three can cover. Use `cheers` for implementation; format changed templates with `cargo cheers fmt --rustfmt <files>`.

## Never

- Remove necessary functionality, information users need to decide, or backend validation because the UI looks simpler.
- Remove accessible labels, status messages, error, or permission information.
- Make things so minimal they turn mysterious, or flatten hierarchy completely.
- Oversimplify a genuinely complex domain.

## Verify

Inspect desktop and mobile in one batched round, fix everything in one batch, confirm with at most one more round. Answer each with rendered or source evidence:

- The task completes faster, and it is obvious what to do and what matters most.
- Every necessary capability is still reachable.
- The simpler design loads faster.
- State coverage (pending, error, empty, success) survived the cuts, and the rerun scan has no unexplained findings.

Report what was removed, why, and where anything moved. If simplification exposed missing state coverage, run `cheers-design harden`. When the cuts feel right, hand off to `cheers-design polish`.
