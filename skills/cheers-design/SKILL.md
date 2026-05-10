---
name: cheers-design
description: "Use when a Cheers task requires UI/UX design judgment: shaping a new visible surface, materially changing layout, hierarchy, copy, interaction states, motion, onboarding, or requested design audit/polish."
---

# Cheers Design

Design and iterate production-grade interfaces for Cheers apps. This skill is a design-craft overlay on top of `cheers`, not a replacement for it.

## Required setup

Before design work or file edits:

1. Read `../cheers/SKILL.md` unless it is already loaded in the conversation.
2. If a sub-command is used, read its reference file from `reference/`.
3. Inspect the target app's existing layout, components, styles, state shape, actions, tests, and conventions before changing code.
4. Classify the work as **product** or **brand**:
   - **Product**: authenticated app UI, dashboards, settings, tools, forms, data surfaces. Design serves a task.
   - **Brand**: landing pages, marketing, long-form, portfolio, campaign, public storytelling. Design is the product.
5. State the Datastar interaction layer before implementation: normal navigation, action patch, signal, stream, `JsScript`, or JS bundle.

If the task is visually open-ended, use `shape` first. Do not implement an unconfirmed major direction unless the user explicitly asks you to proceed without a separate brief.

## Datastar-first laws

These override generic frontend advice.

1. **Backend owns truth.** Most app state lives in backend/use-case state. The frontend displays backend-confirmed state.
2. **No optimistic UI.** Never show success, remove items, reorder data, or commit state before the backend confirms it. Use indicators and patch the confirmed result back.
3. **Smallest dynamic layer wins:** anchor/form/redirect → `#[action]` returning `PatchElements` → sparse signals → `EventReceiver` → `JsScript` → static JS bundle. Do not jump to JS first.
4. **Use generated names.** Use generated `...Action` structs, generated ids, `form_names!(...)`, and helper methods. Do not hardcode generated action URLs, signal paths, or patch ids.
5. **Signals are affordance state, not app state.** Use `#[signal]` or `scoped_signal!` for local visibility, input binding, focus/selection, and pending indicators. Use `#[signal(global)]` only when a handler must receive the value.
6. **Patch rendered components.** Prefer `PatchElements::new().element(Component { ... })` when the rendered element has the target id. Add `.id(...)`, `.selector(...)`, or `.mode(...)` only for non-default targets or operations.
7. **Trust morphing.** Send meaningful HTML chunks, even large ones, when that is simpler and correct. Avoid client-side fine-grained DOM bookkeeping.
8. **Loading is honest.** Use `!indicator`, disabled states, `aria-busy`, and clear copy for in-progress work. Success appears only after the backend response.
9. **Navigation is normal.** Use anchors, form submissions, redirects, and browser history by default. View Transitions can enhance navigation, but should not replace it.
10. **JS is an exception.** Use inline Datastar expressions for tiny fragments, `js!` for reused fragments, `JsScript` for server-pushed imperative code, and `include_js_bundle!` only when static client helpers are justified.

## Shared design laws

Apply these without fighting the Datastar-first laws.

- Start with user purpose, content, states, and constraints before layout.
- Keep semantic HTML and ARIA correct: headings, landmarks, labels, button/link semantics, focus-visible, live regions where needed.
- Favor real content over placeholders. Handle empty, loading, error, success, overflow, long text, permissions, and first-run states.
- Product UI should feel trustworthy, consistent, task-focused, and familiar where familiarity helps.
- Brand UI should have a point of view: typography, imagery, composition, and color should feel specific, not generated.
- Avoid generic AI tells: gradient text, decorative glassmorphism, nested cards, endless equal card grids, hero metric blocks, side-stripe card accents, modal-first flows, redundant copy, and em dashes.
- Cards are not a default container. Use them only when grouping or affordance demands it.
- Motion must clarify hierarchy, feedback, loading, reveal, or transition. Respect `prefers-reduced-motion`; do not animate layout properties casually.
- Keep examples and implementation advice in Cheers/Rust/Datastar terms, not client-framework terms.

## Commands

| Command | Use | Reference |
|---|---|---|
| `teach` | Capture strategic context in PRODUCT.md | [reference/teach.md](reference/teach.md) |
| `document` | Create or refresh DESIGN.md from Cheers UI code | [reference/document.md](reference/document.md) |
| `shape [target]` | Produce a task-specific UX/UI brief before code | [reference/shape.md](reference/shape.md) |
| `craft [target]` | Build a confirmed brief into Cheers code and iterate | [reference/craft.md](reference/craft.md) |
| `extract [target]` | Extract reusable Render components and design tokens | [reference/extract.md](reference/extract.md) |
| `critique [target]` | Design-director review with prioritized feedback | [reference/critique.md](reference/critique.md) |
| `audit [target]` | Technical/design implementation report without fixing | [reference/audit.md](reference/audit.md) |
| `polish [target]` | Refine an existing UI to shipping quality | [reference/polish.md](reference/polish.md) |
| `harden [target]` | Add production state, error, i18n, a11y, and edge-case resilience | [reference/harden.md](reference/harden.md) |
| `optimize [target]` | Improve UI performance in the Cheers/Datastar model | [reference/optimize.md](reference/optimize.md) |
| `clarify [target]` | Improve labels, microcopy, loading, success, and error text | [reference/clarify.md](reference/clarify.md) |
| `adapt [target]` | Adapt UI to another viewport, device, or input context | [reference/adapt.md](reference/adapt.md) |
| `onboard [target]` | Design first-run, empty, and activation flows | [reference/onboard.md](reference/onboard.md) |
| `layout [target]` | Fix spacing, rhythm, hierarchy, and responsive structure | [reference/layout.md](reference/layout.md) |
| `typeset [target]` | Improve typography hierarchy, readability, and font strategy | [reference/typeset.md](reference/typeset.md) |
| `colorize [target]` | Introduce strategic semantic or brand color | [reference/colorize.md](reference/colorize.md) |
| `bolder [target]` | Make a safe design more confident without AI effects | [reference/bolder.md](reference/bolder.md) |
| `quieter [target]` | Reduce visual noise while preserving intent | [reference/quieter.md](reference/quieter.md) |
| `distill [target]` | Remove clutter and reduce interaction/state complexity | [reference/distill.md](reference/distill.md) |
| `delight [target]` | Add appropriate, backend-confirmed moments of personality | [reference/delight.md](reference/delight.md) |
| `animate [target]` | Add purposeful CSS/Datastar/native motion | [reference/animate.md](reference/animate.md) |
| `overdrive [target]` | Propose ambitious Datastar-safe polish; build after confirmation | [reference/overdrive.md](reference/overdrive.md) |

`live` is intentionally not migrated yet.

Routing:

1. If the first word matches a command, read that reference and follow it.
2. If there is no command, treat the request as general UI design/craft: perform setup, apply the laws above, and choose the nearest reference when useful.
3. If implementation changes Cheers templates, run formatting, for example:

```bash
cargo cheers fmt --rustfmt <edited-files-or-directories>
```

Before finishing an implementation task, run the project's relevant checks where practical: targeted tests, `cargo test`, clippy, or app-specific validation.
