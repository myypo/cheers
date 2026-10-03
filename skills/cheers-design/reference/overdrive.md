# Overdrive

Push a Cheers interface past conventional limits using the full power of the browser: a table that stays smooth over huge data, a dialog that morphs from its trigger, validation that streams back as the user types, a page transition that feels cinematic. Overdrive changes how an interface feels, not what the product does; real-time collaboration, offline support, or new backend capabilities are product decisions, not UI enhancements.

Start your response with:

```text
──────────── ⚡ CHEERS OVERDRIVE ────────────
》》》 Entering design overdrive mode...
```

## Gate: propose before building

This command has the highest potential to misfire.

1. Think through 2-3 directions that differ in technique, ambition, and aesthetic. Describe what each would look and feel like.
2. Get the user's pick before writing any code. Put each direction's description and its trade-offs (browser support, performance cost, accessibility, maintainability, JS weight) inside the option itself. A structured question can hide the message it rides in, so directions written alongside it may be invisible while the user chooses.
3. Build only the direction the user confirms.

No optimistic UI. Extraordinary feedback still waits for backend confirmation before showing success or committed state; the wow lives in honest pending states and the confirmed transition, not in pretending.

## Visitor mode

Context decides what extraordinary means. A particle system on a portfolio is impressive; the same system on a settings page is embarrassing. Ask what would make a user of this specific interface say "that's nice".

- **Persuade + Experience:** the wow is often sensory: a scroll-driven reveal, a shader or canvas ground, a cinematic transition between views, art-directed image treatment, type choreography.
- **Operate + Read:** the wow is in how it feels: a dialog that morphs from its button, a precise pending-to-confirmed transition, keyboard-first density, filtering that never flickers. On data-heavy screens, fluid transitions between data states and rendering that handles real volume. On performance-critical screens, the wow is invisible: the interface never hesitates.

Overdrive works inside the established world. A new identity routes through [new-work.md](new-work.md).

## Toolkit, in design order

Reach for the earliest layer that achieves the effect. Use `cheers` for exact implementation of anything server-rendered, patched, or streamed.

1. **CSS and native features.** View Transitions (same-document broadly supported; cross-document not in Firefox), `@starting-style` for entry from `display: none`, scroll-driven animations (`animation-timeline: scroll()`, behind `@supports` with a static fallback), `@property` to animate typed custom properties, container queries, SVG and SVG filter chains.
2. **Hypermedia.** Server-rendered confirmed states, coherent update regions, server-side paging or windowing for large tables, and streams for live data or streaming validation.
3. **Local affordances.** Signals for reveal, selection, focus, and pending state.
4. **Static JS helpers,** only when the experience cannot stay declarative and the trade-off is worth it: Web Animations API for composable, cancellable choreography; spring physics; client-side virtual scrolling when the server cannot window the data.
5. **Canvas, WebGL, WASM,** only when the effect or scale demands it: Canvas 2D or OffscreenCanvas in a Worker to keep the main thread free; WebGL for shaders and large datasets; WebGPU (Chrome/Edge, Safari 26+, Firefox on Windows and macOS) always with a WebGL2 fallback; Web Workers or WASM for heavy computation.

Sound and device APIs need explicit opt-in or permission and a user gesture.

## Implementation discipline

- **Progressive enhancement is mandatory.** Feature-detect every technique; the experience without the enhancement must still be good.
- **Performance:** target 60fps and simplify below 50. Lazy-initialize heavy resources (WebGL contexts, WASM modules) near the viewport and pause off-screen rendering. Check on a mid-range device profile, not only the development machine.
- **Reduced motion** gets an intentional static alternative, not a broken one.
- **Backend truth stays authoritative.** Normal navigation and history keep working; never replace them with a fragile custom state machine.
- **Polish the last 20%:** the easing curve, the stagger offset, the secondary motion that makes a transition feel physical. Ship the version that feels inevitable, not the first that works.
- Read [craft-floor.md](craft-floor.md) before editing; format changed templates with `cargo cheers fmt --rustfmt <files>`.

## Never

- Use technical ambition to mask weak layout, copy, or hierarchy; fix those first with other commands.
- Ship effects that jank on mid-range devices or bleeding-edge APIs without a working fallback.
- Layer several competing wow moments. Focus creates impact; excess creates noise.
- Show success, removal, or reordering before the backend confirms it.

## Verify

Ambitious effects rarely land on the first try, so check the render instead of assuming it. Inspect desktop and mobile in one batched round, refine everything it shows in one batch, confirm with at most one more round, and report what still falls short rather than looping. Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) once over the changed page. Answer each with evidence:

- **Wow:** would someone seeing it fresh react?
- **Removal:** take it away. If nobody would notice, remove it.
- **Device:** still smooth on a phone and a mid-range laptop?
- **Context:** does it fit this world and this audience?
- **Trust:** keyboard, reduced motion, slow network, and a failed backend action all behave honestly.

Present limitations and fallbacks plainly. When the effect holds, hand off to `cheers-design polish`.
