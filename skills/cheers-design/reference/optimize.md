# Optimize

Optimize Cheers UI by measuring the bottleneck, then choosing the smallest fix. Do not import client-heavy habits from browser-first frameworks unless this app actually uses them.

## Measure first

Identify what is slow:

- initial HTML response
- CSS/font/image loading
- action latency
- SSE stream throughput
- morph cost for large patches
- layout/paint cost
- client JS execution
- browser memory for large DOM/list surfaces

Use project tooling, browser DevTools, server logs, Lighthouse/WebPageTest when relevant, and before/after timings. Do not optimize what is not slow.

## Cheers/Datastar optimization priorities

### 1. Avoid unnecessary JS

- Prefer server-rendered `Render` components and Datastar attributes.
- Use inline Datastar expressions for tiny behavior.
- Use `js!` for reused fragments.
- Use `include_js_bundle!` only for reusable static helpers with enough code to justify a bundle.
- Remove unused dependencies and dead scripts.

### 2. Patch at the right granularity

- Default: patch a component whose rendered root has the target id.
- Use fat morph when it simplifies correctness; morphing preserves state and updates only changed DOM.
- Use `.id(...)`, `.selector(...)`, and `.mode(...)` only when targeting differs from the rendered element or doing append/prepend/remove/etc.
- Avoid many tiny patches when one coherent component patch is clearer.
- Avoid huge full-page patches for tiny status changes if a small component patch is obvious and stable.

### 3. Keep signals small

- Signals are for local affordances, form input, and small display values.
- Do not preload large datasets into signals.
- Do not mirror backend state into signals for convenience.
- Remove signals that can be represented by rendered HTML from the backend.

### 4. Stream efficiently

For SSE / `EventReceiver`:

- send meaningful events, not noisy heartbeats unless needed
- compress streams with Brotli/gzip where the stack supports it
- prefer patching rendered chunks over imperative scripts
- stop streams on disconnect
- model CQRS when one long-lived read stream plus short-lived write actions fits the workflow

### 5. Optimize assets

- Serve correctly sized responsive images with useful dimensions and `loading="lazy"` below the fold.
- Prefer AVIF/WebP where the project supports it.
- Do not lazy-load above-fold hero/LCP images.
- Subset or reduce font weights.
- Use `font-display: swap` or `optional` where appropriate.
- Keep SVG/icon strategy coherent, often via `include_svg_sprite!`.

### 6. Optimize CSS and paint

- Use CSS variables/tokens to avoid duplication.
- Avoid unbounded blur/filter/shadow effects.
- Use `transform` and `opacity` for common movement.
- Do not casually animate layout-driving properties.
- Use `content-visibility: auto` or pagination for very long independent content when measured helpful.
- Keep DOM depth reasonable by extracting components, not by nesting wrappers.

## Long lists and data surfaces

Preferred order:

1. Backend pagination or filtering.
2. Server-rendered page chunks patched in.
3. `EventReceiver` for live updates when needed.
4. Browser virtualization only if measured DOM size is the bottleneck and a small JS helper is justified.

Do not reach for client-side virtualization advice in a plain Cheers surface without measuring.

## Loading performance patterns

- Use `@async` for streamed initial rendering with accessible, layout-stable fallbacks.
- Use `!indicator` for action pending state.
- Skeletons are acceptable only as honest loading placeholders. They are not optimistic success.
- Reserve image/media dimensions to prevent layout shift.

## Verify

Report before/after:

- what was measured
- what changed
- observed impact
- trade-offs
- any remaining bottleneck

Then run relevant formatting and tests.
