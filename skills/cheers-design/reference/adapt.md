# Adapt

Adapt an existing Cheers design to another viewport, device, input method, or usage context. Adaptation is not scaling pixels; it rethinks structure and interaction for the new context. Ask for target devices and usage context when the request and product docs do not name them.

## Visitor mode

- **Persuade + Experience:** each context may get its own composition when the world earns it. A phone first viewport is a composition of its own, not a squeezed desktop, and still makes the offer intelligible with the primary action in reach.
- **Operate + Read:** the same information architecture, terms, and navigation model in every context. Structure collapses, reflows, and discloses; it never reorganizes into something the user must relearn.

Refinement preserves: keep the visual world, copy, behavior, and everything outside the adapted contexts. Identity replacement belongs to [new-work.md](new-work.md).

## Two isolated assessments

When a subagent tool is available and permitted, run these independently; otherwise run them yourself in this order.

1. **Adaptation assessment.** Inspect the target in its source context and each target context. Answer each question with rendered or source evidence:
   - **Source assumptions:** What was it designed for: screen size, pointer, hover, keyboard, connection, posture, glance or focus? What works well there and must survive?
   - **Target context:** Phone, tablet, desktop wide, kiosk, print, email, embedded panel, slow network? Portrait and landscape? Touch, mouse, keyboard, or several at once?
   - **Fit:** What will not fit: wide tables, dense sidebars, multi-column forms, long navigation? What can move behind progressive disclosure without hiding core functionality?
   - **Input:** What depends on hover, precise pointing, right-click, drag, or keyboard shortcuts? Which targets are too small or too close?
   - **Gestures:** Which custom controls (sliders, drag surfaces, carousels, swipe rows, scrollable control strips) exist, and does each have a non-gesture path?
   - **Regions:** Do backend-refreshed regions keep one stable home across breakpoints, or does a duplicated mobile tree leave a patch updating only one copy?
2. **Mechanical scan.** Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) with `--scope layout`, once at desktop width and once with `--viewport 390x844`, or check by hand when it is unavailable. Also look for fixed widths, horizontal overflow, and `user-scalable=no` the scan may miss.

Keep scan results out of the first assessment, then synthesize both before editing. A clean scan says nothing about whether a gesture works.

## Set the adaptation plan

Before editing, state for each target context: what leads, what moves behind disclosure, what changes interaction model, where the content (not a device list) forces each breakpoint, and which regions reflow, collapse, or stay fixed. State the interaction contract for any control whose input model changes.

## Apply

Read [craft-floor.md](craft-floor.md) before editing. Implement through `cheers`.

### Mobile

- One clear flow: single column, full-width components, primary content first, secondary content in disclosure.
- Primary actions within thumb reach; the bottom of the screen is easier to reach than the top.
- Navigation collapses to a drawer, disclosure, or a short bar; keep a way back and a sense of place.
- Wide tables become summary rows with detail pages, stacked label-value cards, or a horizontally scrollable table with a visible affordance, chosen by what the user compares.
- Forms stay short; split only when it reduces cognitive load. Body and input text at least 16px, or iOS Safari zooms focused inputs and breaks the layout.

### Tablet

- Two columns, master-detail, or a collapsible side panel, adapting to orientation or container size.
- Support touch and pointer together; touch-size targets with denser layout than a phone.

### Desktop and wide

- Use horizontal space for persistent navigation, filters, side panels, and comparison.
- Cap prose and forms with a max width; never stretch to the viewport.
- Add hover detail, keyboard shortcuts, multi-select, and drag where they speed power workflows, each with a non-hover, non-drag path. Desktops have touchscreens too.

### Print and email

- **Print:** hide navigation and interactive controls, expand disclosed content that matters, show full URLs, break pages at logical points, keep heading order, and use print CSS rather than a separate data model.
- **Email:** about 600px, single column, inline styles, table layout for client compatibility, large obvious buttons, no hover, deep links back to the app for anything interactive.

### Touch and input

- Touch targets at least 44x44 CSS px with spacing between them. A small visible mark can sit inside a larger hit area.
- Detect input, not screen size: `(hover: hover)` and `(pointer: coarse)` queries. Nothing essential lives behind hover; touch gets a visible control or an active state instead.
- Every swipe, drag, pinch, or multi-finger gesture has a single-tap alternative: buttons for carousel steps, a menu or visible action for swipe-to-delete, steppers or inputs beside a slider, move buttons beside drag reorder.
- Long-press is never the only way to an action; surface the same action in a visible menu.
- Never disable pinch zoom. Custom pan or zoom surfaces set `touch-action` so the page still scrolls around them.
- Gesture results that change data follow the guardrails: a swipe-delete or drag-reorder shows a pending state and changes the list only after the backend confirms.
- Give touch feedback on press; keep it on the control, not on an image inside it.

### Responsive technique

- Mobile-first: base styles for narrow, `min-width` queries layer complexity. Usually three content-driven breakpoints suffice.
- Container queries for components that appear in contexts of different widths.
- Safe areas: `env(safe-area-inset-*)` with `viewport-fit=cover`, using `max()` for fallbacks on fixed bars.
- Responsive images: `srcset` with width descriptors and accurate `sizes`; `<picture>` only for art direction.
- `<details>` and `<summary>` for native progressive disclosure; signals for local open and closed affordances.
- `display: none` still downloads the hidden content's assets; hide sparingly, and prefer not rendering it or `<picture>` and `srcset` for imagery.
- Prefer CSS over duplicate component trees. When structure truly differs, compose smaller reusable pieces so each backend-refreshed region still exists once.

Never hide core functionality on a device class, switch information architecture between contexts, forget landscape, or assume desktop means a powerful device.

## Verify

Inspect in one batched round: narrow phone (320 and 390), phone landscape, tablet, desktop wide, keyboard only, touch, and 200% zoom, on the major engines (Chromium, WebKit/Safari, Firefox) and a throttled connection, plus any named target. Fix everything it shows in one batch and confirm with at most one more round. For action-driven UI, exercise the interaction that changes state in each context.

Exercise each custom control in scope in the same round:

- **Primary gesture:** tap it and confirm it responds as designed, then drag it with the target input; the drag must complete, not just start.
- **Scroll across it:** a swipe along the page's scroll axis across the control scrolls the page or container without activating it; a drag that starts on the control along its own axis moves the control, not the page. Neither failure throws an error, so try both.
- **Alternative path:** the non-gesture path reaches the same result.

Say what produced the evidence: an emulated viewport, synthesized touch through a browser tool, which engine ran it (Chromium is not Safari), or a physical device. Screenshots and resized viewports verify layout, never a gesture. Name what stayed untested and move on; unreachable hardware is a reported gap, not a blocker.

Format changed templates with `cargo cheers fmt --rustfmt <files>`. When each context feels native, hand off to `cheers-design polish`.
