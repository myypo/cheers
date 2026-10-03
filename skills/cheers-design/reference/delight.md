# Delight

Make a Cheers UI memorable at moments that earn it. Delight is not a layer of generic whimsy; it is product character revealed through a useful interaction, a humane response, or an unexpectedly considered detail.

Know the product's emotional range. Ask only when it, or the stakes, cannot be inferred from PRODUCT.md, DESIGN.md, and the surface.

## Visitor mode

- **Persuade + Experience:** personality may run through voice, composition, motion, and discovery, provided the artifact stays the focus.
- **Operate + Read:** concentrate delight at meaningful moments such as first use, completion, recovery, or mastery. Reliability carries everything else.

Delight works inside the established world and voice. It never introduces a new identity.

## Two isolated assessments

Run these in order, and do not let scan findings anchor the design assessment. With subagents available, run them independently.

1. **Design assessment:** inspect the target, DESIGN.md, product voice, repeat frequency, and emotional context. Look for:
   - effort worth acknowledging;
   - waiting that can become informative;
   - an empty or first-use state that can orient;
   - an error or recovery moment that needs empathy;
   - an interaction whose physical or verbal response could express the brand;
   - a useful capability people might enjoy discovering.

   Do not manufacture a celebration for an ordinary click.
2. **Mechanical scan:** run the [mechanical scan](../SKILL.md#mechanical-scan-optional) over the target page.

Synthesize both before editing.

## State the delight thesis

In one sentence, state what the user should feel and why that feeling belongs to this product. Then name the moment, its trigger in the interaction contract, and the smallest system that delivers it:

- a distinctive response to a meaningful action;
- product-specific language that clarifies while carrying voice;
- an interaction or transition with a recognizable material behavior;
- an illustration, sound (opt-in), or environmental detail grounded in the product world;
- a discovery reward that reveals real utility.

Derive the treatment from the product mechanism and visual world, not a stock catalog.

## Apply

Read [craft-floor.md](craft-floor.md) first; for authored motion, also load [animate.md](animate.md).

- **Success:** fires only once the backend-confirmed state is rendered, never on click. Match the response to effort and consequence: milestones can expand; routine saves should simply feel certain.
- **Waiting:** truthful progress, useful context, or product-specific activity ("Importing 24 of 120 rows"). Never fake work, imply completion, or delay the result to stage a flourish.
- **Empty and first use:** make the next action clear before adding personality.
- **Error and recovery:** lead with the problem and the recovery. Warmth may lower stress; jokes must not trivialize loss, money, privacy, or blocked work.
- **Repeated interaction:** still satisfying on the hundredth use. Vary only when it stays coherent and predictable enough to trust.
- **Discovery:** reward curiosity without hiding required functionality.
- **Local micro-interactions:** hover, focus, active, and reveal details may use signals as local affordances; keep them independent of durable state.

Copy uses the product's language. Generic whimsy is worse than neutral clarity.

## Protect the experience

Delight must not:

- delay, block, or obscure the primary task;
- override native conventions or accessibility;
- add unrequested factual claims;
- play sound without opt-in or ignore mute settings;
- become mandatory, unskippable, or exhausting on repeat;
- cost a dependency, JS helper, or asset weight out of proportion to the moment.

Respect screen readers, keyboard, touch, localization, and cultural context. Nonessential loops stop when hidden. Reduced-motion users get a finished static version. Use `cheers` for implementation; format changed templates with `cargo cheers fmt --rustfmt <files>`.

## Verify

Inspect in one batched round, fix everything in one batch, confirm with at most one more round. Answer each with rendered or source evidence:

- The moment is specific enough that a neighboring product could not use it unchanged.
- It improves comprehension, confidence, motivation, or emotional recovery.
- It appears only after backend confirmation, and a failed action shows no celebration.
- The interface stays fast and obvious without the flourish, on a slow network too.
- Repetition does not turn charm into friction; muted, keyboard, touch, reduced-motion, and localized paths work.
- It feels like the established world, and the rerun scan has no unexplained findings.

When the personality feels earned, hand off to `cheers-design polish`.
