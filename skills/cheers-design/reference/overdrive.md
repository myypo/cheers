# Overdrive

Push a Cheers interface beyond conventional quality with ambitious but context-appropriate polish. This command has a high risk of excess; propose before building.

Start your response with:

```text
──────────── ⚡ CHEERS OVERDRIVE ────────────
》》》 Entering design overdrive mode...
```

## Gate

Before code, propose 2-3 directions and ask the user to pick. Include trade-offs for accessibility, browser support, performance, maintainability, and fit with the register. Do not implement until a direction is confirmed.

No optimistic UI. Extraordinary feedback still waits for backend confirmation before showing success or committed state.

## What extraordinary means

- **Brand surface**: cinematic composition, scroll-linked reveals, generative background, art-directed image treatment, distinctive type choreography.
- **Product surface**: interaction that feels inevitable: instant-feeling but honest pending state, precise state transitions, keyboard-first density, large data that stays smooth.
- **Data-heavy UI**: clearer filtering, comparison, paging, density, and motion restraint before technical spectacle.
- **Performance-critical UI**: less visible drama, more absence of hesitation.

## Toolkit, in design order

1. Semantic HTML and CSS: View Transitions, container queries, scroll-driven animations with fallbacks, custom properties, SVG.
2. Server-rendered confirmed states and coherent update regions.
3. Local affordances for reveal, selection, focus, and pending state.
4. Server-pushed or static JS only when the experience cannot stay declarative and the trade-off is worth it.
5. Canvas/WebGL/WASM only when the effect or scale demands it and fallback is defined.

Use `cheers` for exact technical implementation.

## Implementation discipline

- Progressive enhancement is mandatory.
- Reduced-motion fallback must look intentional.
- Heavy resources lazy-initialize and stop when off-screen when possible.
- Backend truth remains authoritative.
- Do one browser inspection and at least one refinement pass.
- If the effect does not improve the experience when removed, remove it.

## Never

- Add technical spectacle to mask weak layout, copy, or hierarchy.
- Ship janky effects.
- Use bleeding-edge APIs without fallback.
- Add sound without explicit opt-in.
- Layer multiple competing wow moments.
- Replace normal navigation/history with a fragile custom state machine.

## Verify

Run the wow test, removal test, mid-range device test, reduced-motion test, keyboard test, and backend-failure test. Present limitations honestly.
