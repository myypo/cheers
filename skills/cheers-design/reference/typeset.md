# Typeset

Typography carries information, hierarchy, and voice. Improve it inside the established visual world; do not replace the identity unless the user asked to.

## Visitor mode

- **Persuade + Experience:** display type may carry the voice. Use decisive contrast and responsive (`clamp()`) display scale when the composition benefits.
- **Operate + Read:** stability, scanability, and measure come first. A single well-tuned family and a fixed `rem` role scale (about 1.125–1.2 between steps) are often right. See [operate.md](operate.md#typography).

If typography replacement would create a new identity, route through [new-work.md](new-work.md) and update DESIGN.md. Otherwise preserve confirmed families and improve their use.

## Two isolated assessments

When a sub-agent tool is available and permitted, run these independently; otherwise run them yourself in this order. Do not let scan findings anchor the design assessment.

1. **Typographic assessment.** Inspect representative pages, states, and stylesheets. Answer every question with a file, selector, or computed value:
   - **Authority and fit:** which faces, weights, and roles are established? Do they fit the product and its world, or are they unexamined defaults? Is every family necessary?
   - **Hierarchy:** can heading, body, label, metadata, and data roles be told apart at a glance? Are adjacent sizes or weights too close to carry different jobs?
   - **Scale and consistency:** is there a deliberate role scale or a collection of arbitrary values? Do repeated roles stay identical across screens and across pending, error, empty, and success states?
   - **Reading:** does body copy hold a comfortable measure? Are line height, paragraph rhythm, contrast, and tracking tuned to the actual face, width, language, and surface?
   - **Stress:** what happens with long headings, localization expansion, 200% zoom, narrow containers, backend-patched content, validation errors, missing weights, and font fallback?
   - **Delivery:** are only used files and weights loaded? Do fallback metrics, `font-display`, and variable-font settings avoid invisible text and disruptive reflow?
2. **Mechanical scan.** Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) with the type scope, for example `impeccable detect --json --scope type <url>` against the dev server. It cannot read `html!` templates, so also check inline style values and font sizes set in templates by hand.

Synthesize both before editing, noting what each caught alone. A clean scan is a floor, not proof of good typography.

## Set the system

Before editing, state:

- the roles the interface needs (for example label, body, title, heading, data, caption, code);
- the intended contrast between those roles;
- the reading measure and density;
- which existing faces and weights are authoritative;
- performance, localization, and accessibility constraints.

Use the fewest roles and families that make the hierarchy unmistakable. Combine size, weight, space, and tone instead of asking size alone to do the work. Name role tokens by purpose, not value: `--text-label`, not `--text-14`.

## Apply

Read [craft-floor.md](craft-floor.md) before the first edit; its type checks (measure, display ceiling, tracking floor, scale steps) apply here and are not repeated.

- Keep body copy readable and zoomable: `1rem` is the ordinary body floor unless a dense role, the design system, or a user setting justifies otherwise. Size in `rem`, never fixed pixels, so browser zoom and user font settings keep working.
- Hold body prose at the craft floor's measure with `max-width` in `ch`; narrow secondary columns may run down to about 45ch. Tune line height to the face, not a universal ratio, and inversely with measure: wider lines need more leading. Headings sit tighter, about 1.0–1.2.
- Compensate light text on dark surfaces on all three axes: slightly more line height, a touch more tracking, and one step more weight when the face needs it.
- Use one paragraph rhythm: spacing or first-line indent, not both.
- Use `font-variant-numeric: tabular-nums` for aligned numbers, and code or label features where the content benefits.
- Do not use display faces in labels, buttons, or dense data.
- Load only used weights and files. Self-host with metric-compatible fallbacks; never block text on a font.
- Let Persuade display type respond to available space; keep Operate and Read surfaces spatially predictable.

Do not make type decorative at the expense of comprehension, and do not add a second family without a role only it can perform.

### Cheers fit

- Keep type roles in the project's stylesheets and tokens, not inline styles in `html!` templates.
- Pending, error, empty, and success states rendered by the backend use the same roles as the component they belong to; a patched error message is not a new typographic voice.
- Use `cheers` for rendering mechanics. Format changed templates with `cargo cheers fmt --rustfmt <files>`.

## Verify

Inspect in one batched round (desktop and mobile together), fix everything it shows in one batch, confirm with at most one more round, then rerun the scan once. Answer each item with rendered or source evidence; a bare "yes" is not verification.

- Primary, secondary, body, and metadata roles are recognizable without reading the copy.
- Long text stays comfortable across relevant widths, languages, and patched states.
- The typography belongs to the product and its established world.
- Loading causes no invisible text or disruptive reflow.
- Zoom, user font settings, focus, contrast, and narrow viewports remain usable.
- The final scan has no unexplained findings.

When the hierarchy holds, hand off to `cheers-design polish`.
