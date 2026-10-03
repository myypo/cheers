# Optimize

Improve perceived and measured UI performance by measuring the bottleneck for this interface, then choosing the smallest fix. A Cheers app renders HTML on the server and patches it with a small Datastar runtime, so the usual cost centers are server response time, asset weight, patch size and frequency, and paint; do not import bundle-splitting and memoization habits from client-heavy frameworks unless this app actually ships that code.

## Visitor mode

- **Persuade + Experience:** the first viewport, its imagery, and its one authored motion must arrive fast and stable. Keep the committed world; bound expensive effects instead of deleting them.
- **Operate + Read:** speed is the feature. Action latency, list and table size, and update churn matter more than first paint; removing spectacle often helps more than adding infrastructure.

Refinement preserves: optimize inside the established world and behavior. A fix that changes what the user sees, removes a state, or weakens trust is a design change; name it and ask.

## Two isolated assessments

When a subagent tool is available and permitted, run these independently; otherwise run them yourself in this order.

1. **Perceived-performance assessment.** Use the surface as the user does and answer each question with rendered evidence:
   - **First view:** How long until the primary content and action are usable? Does anything shift, flash unstyled, or swap fonts visibly?
   - **Actions:** Does feedback appear next to the control within about 100ms? Does the wait say what is happening, or does the UI freeze?
   - **Updates:** Do streamed or patched regions flicker, jump, steal focus, or reset scroll?
   - **Motion and paint:** Does scrolling or animation stutter, especially on blur, filters, shadows, or large fixed layers?
   - **Scale:** How does the surface behave with realistic data volume, on a slow phone, on a throttled connection?
2. **Measurement.** The mechanical pass here is numbers, not the anti-pattern scan. Record before values for what the first assessment flagged:
   - Core Web Vitals: LCP (under 2.5s), INP (under 200ms), CLS (under 0.1).
   - Server: initial HTML response time and action round trip, from server logs or DevTools timing.
   - Assets: CSS, font, image, and JS bytes and request count; cache headers on static assets.
   - Live updates: patch payload size, frequency, and morph cost; open stream connections per page.
   - Runtime: long tasks, frame rate, DOM size, memory on large lists.

Use browser DevTools (Performance, Network, Lighthouse), WebPageTest, and server logs, on throttled CPU and network. Keep the measurements out of the first assessment, then synthesize: optimize what is both slow and felt.

## Set the budget

Before editing, name the bottleneck, the metric that proves it, the target value, and the smallest fix that could reach it. Fix the biggest bottleneck first; skip micro-optimizations while a major one stands.

## Apply

Read [craft-floor.md](craft-floor.md) before editing UI. Implement through `cheers` for patch, stream, signal, asset, and JS choices.

### Make waiting honest

- Pending feedback sits next to the control that caused it and appears immediately.
- Preserve layout during loading; reserve space for content that will arrive.
- Show progress, step labels, or partial results only when the backend can truthfully provide them. Never invent progress.
- No optimistic UI as a speed trick. Perceived speed comes from immediate, honest pending feedback and fast backend confirmation, never from showing success early.
- Skeletons and shimmer do not substitute for understanding the wait.

### Server and HTML

- Slow first HTML is usually a slow query or blocking work in the handler; fix it there before touching the front end.
- Stream or defer slow, independent regions so the primary content renders first, each with an honest loading state.
- Paginate, filter, or chunk on the backend before reaching for browser virtualization.
- Compress responses and cache static assets with long-lived, fingerprinted URLs.
- Prefetch a likely next page only when the request is a safe, side-effect-free GET.

### Assets

- Images: correct dimensions or `aspect-ratio`, `srcset` and `sizes`, modern formats (AVIF, WebP), compression around 80 to 85% quality. Lazy-load below-fold media; never the LCP image, which can be preloaded.
- Fonts: subset, load only used weights, `font-display: swap` or `optional`, preload the critical face, and use metric-compatible fallbacks. System fonts are legitimate on Operate surfaces.
- CSS: remove unused rules, keep critical CSS small, use `contain` for independent regions.
- JS: remove icon sets, scripts, and third-party tags that do not serve the surface. Static JS helpers load only on pages that use them.

### Dynamic updates

- Patch the smallest stable region that reads clearly; prefer one understandable region update over many scattered micro-updates, and avoid full-page patches for tiny status changes.
- Throttle or coalesce high-frequency stream updates to what a person can read.
- Keep signals small and local; they are affordances, not a client-side data store.
- Debounce search-as-you-type input before it reaches the backend.
- Never inject content above existing content, especially from patches and streams; it shifts layout (CLS).

### Rendering and motion

- Animate transform and opacity for common movement; blur, filters, masks, clip paths, and shadows stay available when bounded to small, isolated areas.
- Do not casually animate layout-driving properties (width, height, top, left, margins).
- `will-change` only for known expensive operations, removed when done.
- `content-visibility: auto` for long, independent content when measurement shows it helps.
- Keep DOM depth reasonable by removing wrapper noise and extracting meaningful components.
- In remaining static JS helpers, batch DOM reads before writes, use `requestAnimationFrame` for visual updates, throttle scroll and resize handlers, break up long tasks, and defer non-critical JS (INP).
- Never sacrifice accessibility, reduced-motion support, or working behavior for a number.

## Verify

Re-measure the same metrics under the same conditions, desktop and throttled mobile in one batched round, including a low-end Android device and real-user monitoring where available. Fix what remains in one batch and confirm with at most one more round. Check that every state and action still works and that it feels faster, not only scores better.

Report:

- what was measured, and how
- what changed
- before and after values
- trade-offs
- the remaining bottleneck

Format changed templates with `cargo cheers fmt --rustfmt <files>` and run targeted checks through `cheers` guidance. When the user-facing numbers move, hand off to `cheers-design polish`.
