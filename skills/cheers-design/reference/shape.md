# Shape

Shape the UX/UI for a Cheers feature before code. Output a brief that makes the backend state, Datastar behavior, and visual direction explicit.

## Discovery

Ask only what is missing from the request, existing product docs, or code. With a sparse prompt, ask 2-3 questions and wait instead of inventing the whole brief.

Cover these areas:

- **Purpose**: What problem does this surface solve? What is the primary user action?
- **User context**: Who uses it, where, under what pressure, and how often?
- **Success**: What should the user understand or complete? What should the system confirm?
- **Content/data**: What is displayed or collected? What are min, typical, max, empty, and error cases?
- **Scope**: sketch, mid-fi, high-fi, or production-ready; one component, one screen, or flow; static or interactive.
- **Register**: product or brand. If unclear, infer from the surface and explain.
- **Visual direction**: color strategy, theme scene, typography tone, density, imagery needs, and anti-goals.
- **Technical constraints**: existing components, CSS setup, browser support, performance budget, tests.

## Datastar state map

Every brief must include a state map. Separate:

1. **Backend-owned state**: durable data and derived view models.
2. **Submitted input**: `#[form]` fields, generated form type, or explicit `Form<T>`.
3. **Client-only signals**: local visibility, input binding, selected tab, pending affordance, menu open state.
4. **Global signals**: only values a handler must receive.
5. **Patch targets**: generated ids or stable semantic containers.
6. **Streams**: whether `EventReceiver` is needed for long-lived updates.
7. **JS**: whether a static JS bundle is truly justified.

Reject designs that require broad backend state mirrored into signals or optimistic success.

## Brief structure

Present the brief in this shape and ask for explicit confirmation:

1. **Feature summary**: 2-3 sentences.
2. **Primary user action**: the one thing to make obvious.
3. **Register and visual direction**: product/brand, scene sentence, color strategy, typography/density, references or anti-references.
4. **Scope**: fidelity, breadth, interactivity, time intent.
5. **Layout strategy**: hierarchy, rhythm, grouping, responsive approach.
6. **Component plan**: likely `Render` components and where generated ids/forms/signals belong.
7. **Datastar state map**: backend state, signals, actions, forms, patch targets, streams, JS.
8. **Interaction model**: clicks, form submits, loading, validation, success, failure, keyboard/touch behavior.
9. **Key states**: default, empty, loading, error, success, disabled, permissions, overflow, long text, mobile.
10. **Content requirements**: headings, labels, microcopy, error copy, alt text, dynamic ranges.
11. **Implementation references**: which files in this skill and `../cheers/SKILL.md` matter during build.
12. **Open questions**: unresolved choices that affect code or design.

Stop after asking for confirmation unless the user already gave a confirmed brief or explicitly asks you to continue without confirmation.

## Brief quality checks

Before presenting it, verify:

- No optimistic updates are proposed.
- Every structural update is mapped to a backend action, redirect, or stream.
- Every signal has a narrow reason to exist.
- Patch targets can be generated ids or existing stable ids.
- Loading and errors are honest and accessible.
- The brief includes responsive and edge-state expectations.
