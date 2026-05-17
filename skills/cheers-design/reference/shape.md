# Shape

Shape the UX/UI for a Cheers feature before code. Output a task-specific design brief that makes purpose, visual direction, states, and interaction contract explicit.

## Discovery

Ask only what is missing from the request, existing product docs, design docs, or code. With a sparse prompt, ask 2-3 questions and wait instead of inventing the whole brief.

Cover:

- **Purpose**: what problem this surface solves and the primary user action.
- **User context**: who uses it, where, under what pressure, and how often.
- **Success**: what the user should understand, complete, or trust afterward.
- **Content/data**: what is shown or collected, including min, typical, max, empty, and error cases.
- **Scope**: sketch, mid-fi, high-fi, or production-ready; one component, screen, or flow; static or interactive.
- **Register**: product or brand. Infer when obvious and ask for confirmation when consequential.
- **Visual direction**: color strategy, scene sentence, typography tone, density, imagery/media needs, references, anti-references.
- **Constraints**: existing design system, accessibility, localization, browser support, performance budget, and implementation boundaries.

## Interaction contract

Every brief must name how the interface changes over time, but keep it at design level:

1. **Backend-confirmed state**: durable data, derived view models, and success/error outcomes.
2. **User input**: what the user submits or edits, and how it survives validation errors.
3. **Local affordances**: open/closed, focus, selection, pending, reveal, and other client-only UI feelings.
4. **Refresh boundaries**: which conceptual regions update after actions or streams.
5. **Long-lived updates**: whether live progress or collaboration changes need a stream.
6. **JS-worthy behavior**: whether any behavior truly needs a static client helper rather than CSS/native/Cheers interaction.

Reject briefs that require optimistic success, broad backend state mirrored into client affordances, or custom browser history for normal navigation.

## Brief structure

Present the brief in this shape and ask for explicit confirmation:

1. **Feature summary**: 2-3 sentences.
2. **Primary user action**: the one thing to make obvious.
3. **Register and visual direction**: product/brand, scene sentence, color strategy, typography/density, imagery, references or anti-references.
4. **Scope**: fidelity, breadth, interactivity, time intent.
5. **Layout strategy**: hierarchy, rhythm, grouping, responsive approach.
6. **Component and pattern plan**: likely UI patterns and reusable pieces, in design terms.
7. **Interaction contract**: backend-confirmed outcomes, inputs, local affordances, refresh boundaries, streams, JS need.
8. **Key states**: default, empty, loading, pending, error, success, disabled, permissions, overflow, long text, mobile.
9. **Content requirements**: headings, labels, microcopy, error copy, alt text, dynamic ranges.
10. **Build references**: which cheers-design reference files should guide implementation.
11. **Open questions**: unresolved choices that affect code or design.

Stop after asking for confirmation unless the user already supplied a confirmed brief or explicitly asks you to continue without confirmation.

## Quality checks

Before presenting it, verify:

- The primary action is clear.
- The visual direction is specific enough to avoid category reflex.
- State coverage is complete enough for the requested fidelity.
- Loading and errors are honest and accessible.
- Responsive behavior is structural.
- The interaction contract can be implemented without optimistic UI.
