# Polish

Refine an existing Cheers UI to shipping quality. Polish is refinement, never concealed redesign: preserve the incumbent visual world, content, behavior, and everything outside scope. If the concept itself is wrong, say so and recommend a redesign through [new-work.md](new-work.md) or `bolder` instead of smuggling in a replacement.

Additional context needed: the quality bar and shipping constraints.

## Preconditions

Polish is the last step, not the first. It assumes the core behavior exists; if the feature is undefined or incomplete, run `shape` or new-work first.

## Visitor mode

- **Persuade + Experience:** finish the committed world at full strength: the first viewport, the signature moment, the type voice, real imagery. Never sand the world down toward the category default in the name of cleanliness.
- **Operate + Read:** consistency is the polish. One component vocabulary, one state vocabulary, stable density, familiar affordances, per [operate.md](operate.md); on Read, measure, wayfinding, and the reading column.

## Two isolated assessments

Run these yourself in this order, or as independent subagents when the harness can spawn them. Keep scan output out of the first assessment, then synthesize both before editing.

1. **Polish assessment.** Use the feature yourself in the running app at desktop and mobile in one batched round, with mouse, keyboard, and touch where applicable. Walk the path from the user's side before opening devtools. Determine:
   - whether the path is functionally complete, and what is deliberately unfinished;
   - the intended quality bar and the time available;
   - the states, content lengths, roles, and input methods users will actually meet;
   - where the flow, hierarchy, terminology, or disclosure drifts from neighboring features;
   - where the interaction contract breaks Cheers guardrails: optimistic success, pending that reads as done, signals posing as durable state.
2. **Mechanical scan.** Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) once over the target pages. Its findings are defect evidence, never proof of quality.

**Prior critique.** Resolve the target to the same file path or URL critique would use, then:

```bash
node <skill-dir>/scripts/critique-storage.mjs slug "<resolved>"
node <skill-dir>/scripts/critique-storage.mjs latest <slug>
```

Exit 0 prints the latest snapshot: fold its P0/P1 findings into the backlog and name the snapshot you read. Exit 2 means none exists. The helper does not know whether the target changed since the critique, so confirm each finding against current code before acting on it. Run your own assessment either way.

## Set the system

Read DESIGN.md and representative tokens, shared components, patterns, and neighboring flows. Without a formal system, use the coherent conventions in code. Classify each drift before fixing it:

- **missing token:** the system needs a reusable value;
- **one-off implementation:** an existing shared component or pattern should replace it;
- **conceptual mismatch:** the flow, information architecture, or hierarchy differs from comparable product areas;
- **local defect:** the implementation is simply incomplete or inconsistent.

Fix the cause at the narrowest correct level; patching values does not fix a wrong flow. Ask when a binding system principle cannot be inferred. Before editing, state the authoritative tokens and components, the quality bar, the interaction contract on the path, and the triaged backlog. Then read [craft-floor.md](craft-floor.md).

## Triage

Fix in this order:

1. broken or blocked tasks, data loss, misleading or optimistic state, inaccessible paths;
2. missing loading, empty, pending, error, success, disabled, and permission states;
3. flow, hierarchy, responsive, and design-system drift;
4. visual and motion inconsistencies;
5. code and asset cleanup.

Do not perfect one corner while leaving the rest below the same bar.

## Apply

Build to the craft floor without restating it. Implement through `cheers` for templates, actions, patches, and signals.

- **Flow and hierarchy:** match neighboring mental models, terminology, disclosure, routing, and save behavior. Outcomes stay backend-confirmed even where a neighbor cheats. Make the primary task and current state obvious without flattening everything to equal weight. Connect arrival, transition, empty, and recovery paths instead of leaving isolated screens.
- **Layout and type:** align to the project's grid and spacing scale, optically as well as mathematically. Group related content tightly, separate distinct groups generously. Keep same-role type consistent; test measure, wrapping, localization expansion, zoom, and font loading at every supported viewport, not only the current screenshot.
- **Color, imagery, icons:** semantic tokens with stable meanings across themes; text, control, and focus contrast in every state; one icon family, stroke, size, and optical alignment; images with correct aspect ratios, responsive sources, useful alt text, and no layout shift.
- **Interaction and state:** every control has default, hover, focus-visible, active, disabled, pending, error, and success treatment where relevant. Pending says what is happening and blocks risky repeats; success appears only after the backend confirms it. Keep visible focus, logical tab order, labels, and 44px touch targets. Custom drag, slider, and scroll controls complete their gesture under touch, do not swallow page scroll, and clear their state on cancel. Motion stays coherent, interruptible, and performant; do not add animation to make polish visible.
- **Copy:** consistent terminology, capitalization, and punctuation. Concrete button verbs ("Save project", "Retry upload"). Errors say what happened and how to recover; loading text is truthful ("Saving...", not "Saved"). Remove repeated headings and intros, em dashes, and filler such as "seamless" or "powerful" unless the product voice uses them. Ask before changing factual copy or claims.
- **Code:** remove debug output, dead code, unused imports, obsolete styles, and polish-created duplication. Replace custom implementations with shared components where the system owns the pattern. Promote genuinely reusable values to tokens; never abstract one local exception. Reach for CSS and native browser behavior before signals, and signals before JS.

## Verify

One batched round at mobile, desktop, and an intermediate width whenever the layout reflows between them, fix everything it shows in one batch, then at most one confirming round. Answer each check with rendered or source evidence, not a bare yes:

- loading, empty, pending, error, success, disabled, long-content, missing-content, offline, and slow-connection states;
- zoom, contrast, focus, semantics, screen-reader names, and status announcements on patched regions;
- the primary touch gesture on custom controls, naming what produced the evidence (emulated viewport, synthesized touch, which engine, physical device) and what stayed untested;
- console errors, layout shift, interaction latency, and image loading in every supported browser;
- agreement with DESIGN.md, neighboring features, the surface brief, and the user's scope.

Do not run another scan beyond the one in the assessment; fix real defects and document only narrow intentional exceptions. A clean scan does not replace visual judgment.

Format changed templates with `cargo cheers fmt --rustfmt <files>` and run the targeted checks `cheers` prescribes. Finish with a review of the source diff: remove accidental churn, orphaned code, redundant values, and temporary artifacts. Ship only when the feature is functionally complete and consistently finished across the path.

## Hand off

Report what changed, the states and viewports verified, the checks run, any snapshot read with which of its P0/P1 findings this pass resolved and which remain, and intentional exceptions. When the pass took findings from a snapshot, recommend `cheers-design critique` on the same target to record a fresh snapshot, so the next polish does not inherit a resolved backlog.
