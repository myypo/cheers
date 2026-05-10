# Overdrive

Push a Cheers interface beyond conventional quality with technically ambitious but context-appropriate polish. This command has a high risk of excess; propose before building.

Start your response with:

```text
──────────── ⚡ CHEERS OVERDRIVE ────────────
》》》 Entering Datastar-safe overdrive mode...
```

## Gate

Before code, propose 2-3 directions and ask the user to pick. Include trade-offs for accessibility, browser support, performance, and maintainability. Do not implement until a direction is confirmed.

No optimistic UI. Extraordinary feedback still waits for backend confirmation before showing success or committed state.

## What extraordinary means

- **Brand surface**: cinematic composition, scroll-linked reveals, generative background, art-directed image treatment, distinctive type choreography.
- **Product surface**: interaction that feels inevitable: instant-feeling but honest pending state, backend-streamed progress, precise morphs, keyboard-first density, large data that stays smooth.
- **Data-heavy UI**: server-side filtering/pagination, efficient patching, Canvas/SVG only where measured useful, live updates through `EventReceiver`.
- **Performance-critical UI**: less visible drama, more absence of hesitation.

## Toolkit, in Cheers order

1. Semantic HTML and CSS: View Transitions, `@starting-style`, container queries, scroll-driven animations with fallbacks, custom properties, SVG.
2. Server-rendered components and `PatchElements` for confirmed visual states.
3. Datastar: signals for local reveal/selection, `!indicator` for pending, `EventReceiver` for streamed progress or live updates.
4. `JsScript` only for server-pushed imperative behavior that cannot stay declarative.
5. Static JS bundle only for reusable helpers such as complex canvas rendering, advanced keyboard interaction, or measured virtualization.
6. Workers/WASM/Canvas/WebGL only when the effect or scale demands it and fallback is defined.

## Implementation discipline

- Progressive enhancement is mandatory.
- Reduced-motion fallback must look intentional.
- Heavy resources lazy-initialize and stop when off-screen.
- Keep backend truth authoritative.
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
