# Shape

Discover what should be made and how it should work in a Cheers app, then return a confirmed design brief without code. The brief makes purpose, direction, states, and the interaction contract explicit.

## Phase 1: discovery interview

Do not write code or choose visual direction yet.

### Cadence

- Use the structured question tool when available; otherwise ask and stop.
- Ask two or three related questions per round, then wait. One round is the default; add a second only when the answers expose a material gap.
- Do not dump a questionnaire, repeat settled facts from PRODUCT.md, DESIGN.md, or code, or turn obvious facts into menus. Assert the likely reading and invite correction.
- A sparse prompt requires at least one answer round. A precise prompt may need only a compact confirmation.

### Round 1: purpose, people, and outcome

Choose the two or three questions that most change the result:

- What is this surface or feature for, and what problem must it solve?
- Who specifically reaches it, in what situation and state of mind, and how often?
- What is the primary thing they must understand or do? What would success look like?
- What is uniquely true here that a neighboring product or generic template could not claim?

### Round 2: material, behavior, and boundaries

Run only for material unresolved decisions:

- What real content, evidence, data, and assets must the experience carry? What are realistic minimum, typical, and maximum ranges?
- Which states and transitions matter: first-run, empty, loading, pending, error, success, permissions, overflow, or expert use?
- What is the intended fidelity, breadth, and interactivity: exploration, production-ready screen, full flow, or broader surface?
- What must remain untouched? What would make the result feel wrong even if it looked polished?
- Which accessibility, localization, browser, performance, or delivery constraints are binding?

Never ask for CSS values or canned aesthetic lanes. New-work owns visual-world and concept choices.

## Phase 2: resolve the design direction

For new surfaces, brand expansion, or replacement, follow [new-work.md](new-work.md) through visual authority, any direction round, and the concept choice. Reuse discovery, then return here before its direction contract, persistence, or implementation. Inside an established world, use its structure round only when composition or interaction remains materially open.

## Phase 3: name the interaction contract

Every brief names how the interface changes over time, at design level:

1. **Backend-confirmed state:** durable data, derived view models, and success or error outcomes.
2. **User input:** what the user submits or edits, and how it survives validation errors.
3. **Local affordances:** open/closed, focus, selection, pending, reveal, and other client-only UI feelings.
4. **Refresh boundaries:** which conceptual regions update after actions or streams.
5. **Long-lived updates:** whether live progress or collaboration needs a stream.
6. **JS-worthy behavior:** whether anything truly needs a static client helper rather than CSS, native browser behavior, or Cheers interaction.

Reject briefs that require optimistic success, broad backend state mirrored into client affordances, or custom browser history for normal navigation.

## Phase 4: write the brief

Write the smallest useful brief:

1. **Job and audience:** who arrives, their context, need, and visitor mode.
2. **Outcome and proof:** primary task or action, success, real evidence, and product-specific truth.
3. **Selected direction:** visual authority, structural or interaction thesis, sequence, focal moment, and implementation consequence.
4. **Scope and boundaries:** fidelity, breadth, interactivity, named target, what remains untouched, and explicit anti-goals.
5. **States, ranges, and content:** realistic content and data ranges, required headings, labels, microcopy, error copy, and alt text, and the material states: default, empty, loading, pending, error, success, disabled, permissions, overflow, long text, mobile.
6. **Interaction and layout:** the interaction contract above, plus hierarchy, topology, responsiveness, affordances, feedback, and transitions. Intent, not CSS.
7. **Constraints and open decisions:** accessibility, localization, reusable components, relevant cheers-design references for the build, and choices a builder must not invent.

Use three to five bullets when the task is settled; use the full structure only for ambiguous, multi-screen, or standalone planning. Do not restate the conversation.

Before presenting, check that the primary action is clear, the direction is specific enough to avoid category reflex, state coverage is complete enough for the requested fidelity, loading and errors are honest and accessible, responsive behavior is structural, and the contract can be built without optimistic UI.

## Confirm and stop

Present the brief for explicit confirmation or one correction round, then stop: shape never writes code or a direction contract.

When no human or structured answer mechanism exists, mark assumptions plainly, return the brief, and stop.
