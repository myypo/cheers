---
name: cheers-design
description: "Use when a Cheers task requires UI/UX design judgment: building a new visible surface, redesigning or materially changing layout, hierarchy, copy, interaction states, motion, onboarding, or requested design critique, audit, or polish."
argument-hint: "[init|teach|document [--seed]|shape|craft|extract|critique|audit|polish|harden|optimize|clarify|adapt|onboard|layout|typeset|colorize|bolder|quieter|distill|delight|animate|overdrive] [target]"
user-invocable: true
---

# Cheers Design

Design and iterate production-grade interfaces for Cheers apps. Approach every task as a design director with a clear point of view, deep understanding of the client and users, and exceptional craft: safe, timid, average work is the failure mode. This skill owns UX, visual direction, interaction design, content, accessibility, state coverage, and design-system judgment. The main `cheers` skill owns the technical Cheers/Rust/Datastar implementation rules.

Core principles:

- **Go all out.** The deliverable is complete, except assets only the user can provide.
- **Dream big.** Distinct, specific, inspiring work over the category default.
- **Verify in bounded passes, not a loop.** The ceiling covers the whole cycle: screenshots, scans, micro-edits, and rebuilds alike. Build fully, inspect once with a batched round (desktop and mobile together), fix everything it shows in one batch, confirm with at most one more round, and stop. Open-ended self-QA spends the user's time doing worse what a fresh review does better.

## Setup

Before design work or file edits:

1. If the task involves code edits and `cheers` is not already loaded, read it for implementation mechanics. Do not duplicate those mechanics here.
2. Read existing `PRODUCT.md`, `DESIGN.md`, and the target's surface brief in `.cheers-design/surfaces/` (see [new-work.md](reference/new-work.md#5-record-the-decision)) when present. Do not invent missing context.
3. Load the request's playbook: its Commands-table reference for an explicit or implied sub-command, or [reference/new-work.md](reference/new-work.md) for a new surface or replacement visual world. Inspect the target and the incumbent visual truth (running app, templates, CSS, tokens, components, assets) before editing. When the app cannot run, start from committed visual-regression goldens or screenshot fixtures; check they are current against tokens, CSS, components, and assets, resolve conflicts, and compare theme or variant captures.
4. Choose the surface's visitor mode (below) and, for Operate or Read surfaces, read [reference/operate.md](reference/operate.md).
5. State the interaction contract at design level before implementation: normal navigation, backend-confirmed action, local affordance signal, stream, server-pushed script, or static JS helper.
6. After analysis and direction are settled, read [reference/craft-floor.md](reference/craft-floor.md) immediately before any UI edit, including small refinements. Do not load it for planning-only work.

## How to design

- **The brief wins.** Honor pinned aesthetics, eras, materials, fonts, and palettes even when they conflict with a saturated-pattern warning. Redirecting a clear brief toward your taste is failure.
- **Refinement preserves; redesign replaces.** Refinement keeps the incumbent identity, behavior, copy, and everything outside scope. Ask before replacing factual copy or adding claims. Redesign keeps product truth, content, function, native affordances, and constraints, but treats the old look as evidence and anti-reference: choose a replacement world in new-work and replace DESIGN.md. Never split the difference into polish on the discarded look.
- **Visual authority is evidence, not a filename.** A missing DESIGN.md alone does not make a project greenfield; new-work decides whether to preserve, expand, or replace the incumbent world.

## Visitor modes

The mode names what the visitor's success looks like on this surface.

- **Persuade:** the visitor decides and acts; design is the product. Landing pages, marketing, campaigns, pricing. Earn attention and action. Ship real imagery when the brief needs it; follow the committed world, not category habit.
- **Operate:** the visitor completes a task. App UI, dashboards, editors, admin, settings, tools. Scanability, consistency, familiar affordances, and the real usage scene outrank expression. Brand lives in precise details.
- **Read:** the visitor understands something. Docs, articles, guides, help, changelogs. Structure for comprehension, then make the reading experience worth staying in.
- **Experience:** the visitor is inside the work itself. Portfolios, galleries, showcases. Let the artifact lead from the first viewport; the interface recedes.

Choose the mode from the requested surface, not the product, and persist it only in that surface brief. A tool's landing page is still Persuade; a fashion house's documentation is still Read; a docs index is Read, not Persuade. See [modes.md](reference/modes.md) for how far each mode lets a visual world reach, and [operate.md](reference/operate.md) for Operate and Read depth.

## Skill boundary

Use this skill to decide:

- user purpose, primary action, information architecture, and flow
- visitor mode, visual world, hierarchy, composition, typography, color, motion, and imagery
- empty, loading, pending, error, success, permission, overflow, mobile, touch, and first-run states
- labels, microcopy, status messages, help text, and recovery copy
- design-system alignment, reusable UI patterns, and visual quality bar

Use the main `cheers` skill to decide exact implementation details such as generated ids/actions/forms, `Render` mechanics, Datastar attribute syntax, patch APIs, streams, tests, and formatting. In this skill, keep Cheers guidance at the level of design constraints and interaction contracts.

## Cheers interaction guardrails

These are design constraints, not a duplicate implementation manual.

1. **Backend-confirmed trust.** Do not design optimistic success, irreversible removal, completed steps, or reordered data before the backend confirms them.
2. **Honest pending states.** Pending UI may say what is happening, disable risky repeated actions, and show progress. It must not pretend the work is done.
3. **Smallest dynamic layer wins.** Prefer normal navigation and forms when enough; use backend-confirmed actions for structural updates; use signals for local affordances; reserve streams and JS for interactions that truly need them.
4. **Signals are affordances, not app models.** Local open/closed, focus, selection, pending, and lightweight input affordances are fine. Broad backend state belongs in rendered state.
5. **Semantic HTML remains the design substrate.** Headings, landmarks, labels, focus, live status, keyboard paths, and native browser behavior are part of the interface, not implementation afterthoughts.
6. **JS is exceptional.** Reach for CSS, native browser features, and Cheers hypermedia first. Static JS helpers need a clear experiential reason.

## Shared design laws

The craft floor holds the mechanical checks and bans. These laws hold the judgment:

- Start with user purpose, content, states, constraints, and primary action before layout or styling.
- Pick a color strategy before values: Restrained, Committed, Full palette, or Drenched. Use OKLCH when possible; tint neutrals rather than defaulting to pure black, pure white, or flat gray.
- Choose light or dark from a scene sentence: who uses this, where, under what ambient light, in what mood. Do not default by category.
- Use typography for hierarchy and voice. Vary spacing for rhythm; same padding everywhere is monotony.
- Motion must clarify hierarchy, feedback, loading, reveal, or transition. Respect reduced motion.
- Use real content over placeholders. Every word earns its place; avoid filler claims, vague button labels, em dashes in product copy, and status text that overpromises.
- Run the category-reflex check: if the theme, palette, typography, or layout could be guessed from the product category alone, or from category-plus-avoidance, rework the direction.
- Keep examples and implementation advice in Cheers/Rust/Datastar terms when code is needed, but prefer referencing `cheers` over repeating syntax here.

## Mechanical scan (optional)

Command references pair their design assessment with a mechanical anti-pattern scan. Upstream Impeccable's `impeccable detect` can scan a running Cheers app by URL; it cannot read Rust `html!` templates, so point it at the rendered page or at stylesheet files.

- If `impeccable` is on `PATH`, run `impeccable detect --json [--scope type|layout] <url-or-css-files>` against the dev server. Add `--viewport 390x844` for a mobile pass. It drives an installed Chrome or Chromium. Exit 0 means clean, 2 means findings, 1 means a target could not be scanned.
- If only `npx` is available, ask before running `npx impeccable detect`, since it downloads a package.
- Otherwise, or when the dev server is not running, run the scan by hand: check the rendered page and source against [craft-floor.md](reference/craft-floor.md)'s Refuse list and Verify checks.

Run the design assessment before reading scan results so findings do not anchor it, and synthesize both before editing. A clean scan is a floor, not proof of quality. Never block a task on the scan.

## Commands

| Command | Use | Reference |
|---|---|---|
| `init` (alias `teach`) | Capture durable product truth in PRODUCT.md | [reference/init.md](reference/init.md) |
| `document [--seed]` | Create or refresh DESIGN.md from Cheers UI code; `--seed` records a direction before code exists | [reference/document.md](reference/document.md) |
| `shape [target]` | Plan UX/UI as a confirmed brief before code | [reference/shape.md](reference/shape.md) |
| `craft [target]` | Deprecated alias for an ordinary new-work request | [reference/craft.md](reference/craft.md) |
| `extract [target]` | Extract reusable UI components, patterns, and design tokens | [reference/extract.md](reference/extract.md) |
| `critique [target]` | Design-director review with heuristic scoring | [reference/critique.md](reference/critique.md) |
| `audit [target]` | Technical UI-quality report (a11y, performance, responsive) without fixing | [reference/audit.md](reference/audit.md) |
| `polish [target]` | Final quality pass before shipping | [reference/polish.md](reference/polish.md) |
| `harden [target]` | Make states, errors, a11y, i18n, and edge cases production-ready | [reference/harden.md](reference/harden.md) |
| `optimize [target]` | Improve perceived and measured UI performance | [reference/optimize.md](reference/optimize.md) |
| `clarify [target]` | Improve labels, microcopy, loading, success, and error text | [reference/clarify.md](reference/clarify.md) |
| `adapt [target]` | Adapt UI to another viewport, device, or input method | [reference/adapt.md](reference/adapt.md) |
| `onboard [target]` | Design first-run, empty, and activation flows | [reference/onboard.md](reference/onboard.md) |
| `layout [target]` | Fix spacing, rhythm, hierarchy, and responsive structure | [reference/layout.md](reference/layout.md) |
| `typeset [target]` | Improve typography hierarchy, readability, and font strategy | [reference/typeset.md](reference/typeset.md) |
| `colorize [target]` | Introduce strategic semantic or brand color | [reference/colorize.md](reference/colorize.md) |
| `bolder [target]` | Make a safe design more confident without AI effects | [reference/bolder.md](reference/bolder.md) |
| `quieter [target]` | Reduce visual noise while preserving intent | [reference/quieter.md](reference/quieter.md) |
| `distill [target]` | Remove clutter and reduce interaction/state complexity | [reference/distill.md](reference/distill.md) |
| `delight [target]` | Add appropriate, backend-confirmed moments of personality | [reference/delight.md](reference/delight.md) |
| `animate [target]` | Add purposeful motion and state feedback | [reference/animate.md](reference/animate.md) |
| `overdrive [target]` | Propose ambitious Cheers-safe polish; build after confirmation | [reference/overdrive.md](reference/overdrive.md) |

### Routing rules

1. **No argument:** read [routing.md](reference/routing.md) and present its context-aware menu; never auto-run a command.
2. **Explicit or clearly implied command:** load its reference and follow it. Ask once if two commands fit.
3. **Workflow or command-selection question:** answer from [routing.md](reference/routing.md#workflow-questions) without executing.
4. **Otherwise:** treat the request as general design work. A new surface or replacement world with no PRODUCT.md goes through `init`, then new-work. A narrow refinement of existing code proceeds on the incumbent implementation and offers `init` afterward rather than blocking on it.
5. `teach` aliases `init`. `craft` is a deprecated alias for ordinary new-work and adds nothing. `shape` owns task discovery, then enters new-work only for visual-world and surface-concept decisions.
6. If implementation changes Cheers templates, use the main `cheers` validation guidance. At minimum, format changed templates with `cargo cheers fmt --rustfmt <files>` and run targeted checks appropriate to the change.
