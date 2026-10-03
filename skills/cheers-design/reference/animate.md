# Animate

Use motion to explain state, relationship, and hierarchy, or to create one authored moment the surface has earned. Decoration without purpose is animation debt. In Cheers, motion must never hide latency or imply success before the backend confirms it.

Needs: performance constraints and target devices. Ask only when a material constraint cannot be inferred.

## Visitor mode

- **Persuade + Experience:** motion may carry the voice. Prefer one rehearsed focal sequence to repeated section reveals.
- **Operate + Read:** motion serves feedback, state, and continuity. Keep routine transitions fast (150–250ms) and never make users wait through page-load choreography. See [operate.md](operate.md#motion).

Refine inside the established motion language; a new motion identity is part of a new world and belongs to [new-work.md](new-work.md).

## Two isolated assessments

When a sub-agent tool is available and permitted, run these independently; otherwise run them yourself in this order. Do not let scan findings anchor the design assessment.

1. **Motion assessment.** Inspect the existing motion language, interaction states, backend round trips, target devices, and performance budget. Answer with a file, selector, or computed value. Find only the places where motion would:
   - acknowledge an action;
   - make a state change or spatial relationship legible;
   - preserve continuity through navigation, layout change, or a backend patch;
   - direct attention at a meaningful moment;
   - embody the selected visual world.
   Also find motion that already lies: animations that complete, remove, or reorder before confirmation, and loaders that run without saying what is happening.
2. **Mechanical scan.** Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) once against the running page, and check transitions and keyframes in the stylesheets by hand.

Do not animate a static area merely because it exists.

## Set the motion thesis

Before editing, write:

- **Focal moment:** the one sequence or interaction that deserves authorship, if any. It comes from this product and surface; a generic fade-and-rise, hover lift, parallax layer, or scroll reveal is not a thesis.
- **Continuity:** the state, layout, navigation, or patch changes that need explanation.
- **Feedback:** the controls and outcomes that need acknowledgment, split into pending (local, immediate) and confirmed (after the backend responds).
- **Budget:** which effects are expensive and how often they run.

## Choose material by meaning

Read [craft-floor.md](craft-floor.md) before the first edit; its motion check (one authored moment, ease-out from a visible default, the wider property palette) applies here.

- **Continuity and relationship:** view transitions, FLIP-style transforms, deliberate spatial movement.
- **Focus and depth:** bounded blur, filter, backdrop, light, or shadow changes.
- **Reveal and composition:** masks, clip paths, cropping, `grid-template-rows` reveals.
- **Material and energy:** color, gradient position, texture, distortion, shader, or canvas effects when the world and budget support them; anything beyond CSS falls under the JS-is-exceptional guardrail.
- **State and feedback:** the smallest change that makes cause and result unmistakable.

Do not stack techniques for spectacle. Stagger siblings only when a list appears as a list, cap the total delay, and never treat every scrolled section as a staggered list.

## Timing and easing

| Duration | Typical use |
|---|---|
| 100–150ms | immediate feedback, press, toggle |
| 150–300ms | routine state change, small reveal |
| 300–500ms | layout, overlay, dialog, view transition |
| 500–800ms | a deliberately authored focal entrance |

Exit faster than entrance. Long feedback feels like latency. Keep easing in the project's tokens; no bounce or elastic curves by reflex:

```css
:root {
  --ease-out-quart: cubic-bezier(.25, 1, .5, 1);
  --ease-out-expo: cubic-bezier(.16, 1, .3, 1);
}
```

## Backend-confirmed motion

- Animate the pending affordance (a disabled control, a progress mark, a status line saying what is happening), never the outcome. No check draws, exit animations, slide-into-list, or reordering until the backend confirms.
- Success and failure motion play only on backend-rendered state. On failure, the pending motion stops and the original state stays visible with the error near the decision point.
- For long operations, prefer real or staged progress over an endless spinner.
- Transition a backend-rendered validation error in with color, icon, or helper text; avoid shake, especially on repeat.
- A patch that changes layout may carry continuity with a view transition; `cheers` owns how patches and signals drive it.

## Implement with the smallest tool

1. CSS transitions and keyframes for hover, focus, reveal, state classes, and bounded sequences.
2. Native features: `dialog`, `popover`, scroll snapping, view transitions, scroll-driven animation only when the scroll relationship carries meaning, with a fallback.
3. Datastar signals for local toggles and pending affordances, and backend patches for confirmed updates.
4. A static JS helper (Web Animations API) only when interruption, sequencing, or dynamic values cannot stay declarative. Do not add a JS animation library for an effect CSS expresses cleanly.

Keep content visible in the default state so a failed script or dropped stream never hides the page. Do not animate layout-driving properties (`width`, `height`, `top`, `left`, margins) casually; use transforms, FLIP, or grid techniques. Bound blur, filter, shadow, canvas, and shader work to isolated regions. Apply `will-change` only during a known animation. Measure on target devices rather than assuming transform means fast.

## Accessibility and control

- Every animation has a `prefers-reduced-motion` path with an intentional alternative: remove or reduce spatial movement while keeping opacity, color, and state changes that carry meaning. Reduced motion means fewer and gentler animations, not disabling all motion. Reduced motion must still feel finished, and feedback confirming an action stays legible.
- Never rely on motion alone to communicate state; keep keyboard and screen-reader behavior intact.
- Respect autoplay and sound preferences. Stop nonessential loops when offscreen or hidden.

## Verify

Inspect in one batched round (desktop and mobile together, plus reduced motion), fix everything it shows in one batch, confirm with at most one more round. Answer each item with rendered or source evidence; a bare "yes" is not verification.

- The focal motion is specific to the selected world and surface.
- Every supporting animation explains feedback, state, or relationship.
- Pending motion never reads as done; outcome motion plays only after backend confirmation, and failure leaves an honest state.
- Interruption and repeated use behave correctly.
- Desktop, mobile, touch, and keyboard paths remain usable; the console is clean.
- The reduced-motion path reduces movement without erasing meaningful feedback or state changes.
- Expensive effects stay smooth on the target device.
- Removing an animation would lose meaning or authored character, not merely decoration.

Format changed templates with `cargo cheers fmt --rustfmt <files>`. When motion earns its place, hand off to `cheers-design polish`.
