# Optimize

Improve perceived and measured UI performance by measuring the bottleneck, then choosing the smallest fix. Do not import client-heavy habits from browser-first frameworks unless this app actually uses them.

## Measure first

Identify what is slow or unstable:

- initial HTML response
- CSS, font, and image loading
- action latency and perceived pending time
- live update throughput
- size or frequency of DOM updates
- layout, paint, blur, filter, shadow, or animation cost
- client JS execution
- browser memory for large DOM/list surfaces

Use project tooling, browser DevTools, server logs, Lighthouse/WebPageTest when relevant, and before/after timings. Do not optimize what is not slow.

## User-centered priorities

### 1. Make waiting honest and useful

- Show clear pending state near the action.
- Preserve layout during loading.
- Use progress, step labels, or partial results when the backend can truthfully provide them.
- Do not use skeletons or shimmer as a substitute for understanding the wait.

### 2. Reduce visual and asset weight

- Serve correctly sized images with dimensions.
- Prefer modern formats where the project supports them.
- Lazy-load below-fold media, not above-fold LCP media.
- Subset fonts and load only used weights.
- Remove icon sets, scripts, or styles that are not serving the surface.

### 3. Simplify dynamic surfaces

- Prefer one understandable region update over many scattered micro-updates when that is clearer.
- Avoid full-page updates for tiny status changes when a stable local region is obvious.
- Keep local client affordances small.
- For long lists, prefer backend pagination, filtering, or chunking before browser virtualization.

Use the main `cheers` skill for exact patch, stream, signal, and JS implementation choices.

### 4. Optimize CSS and motion

- Use transform and opacity for common movement.
- Avoid unbounded blur/filter/shadow effects.
- Do not casually animate layout-driving properties.
- Use `content-visibility` or pagination for very long independent content only when measured helpful.
- Keep DOM depth reasonable by removing wrapper noise and extracting meaningful components.

## Loading patterns

- Initial loading fallbacks should be accessible and layout-stable.
- Action pending states should be adjacent to the control that caused them.
- Empty, partial, and error states should not cause large layout jumps.
- Performance-critical product UI often improves by removing spectacle, not adding it.

## Verify

Report:

- what was measured
- what changed
- observed impact
- trade-offs
- remaining bottleneck

Then run relevant formatting and tests through the main `cheers` guidance.
