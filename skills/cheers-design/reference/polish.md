# Polish

Refine an existing Cheers UI to shipping quality. Polish is not a rewrite unless the audit reveals structural problems.

## Preconditions

Polish assumes the core behavior exists. If the feature is still undefined or incomplete, run `shape` or `craft` first.

## Order of work

1. **Confirm intent**: primary action, register, target users, and release bar.
2. **Discover the design system**: tokens, component vocabulary, spacing scale, typography, motion, icon set, and neighboring surfaces.
3. **Inspect the rendered UI** when practical. Source-only polish misses alignment, rhythm, and state problems.
4. **Fix trust and state coverage**: empty, loading, pending, error, success, disabled, permission, overflow, long text.
5. **Fix accessibility**: semantics, labels, focus, keyboard, status announcements, contrast, touch targets.
6. **Fix visual craft**: hierarchy, spacing, alignment, typography, color, density, responsive behavior.
7. **Fix copy**: labels, button verbs, status text, error recovery, repeated intros, filler claims.
8. **Validate through `cheers` guidance**: formatting, targeted tests, and implementation checks.

## Design-system alignment

For every drift, classify the root cause:

- **Missing token**: a semantic value should exist but does not.
- **One-off implementation**: a shared pattern already exists but was bypassed.
- **Conceptual misalignment**: the flow, IA, or hierarchy does not match neighboring features.

The fix differs by cause. Do not just patch values when the flow itself is wrong.

## Visual polish checklist

- One primary action is visually and semantically clear.
- Related elements are close; unrelated groups have enough separation.
- Controls have default, hover, focus-visible, active, disabled, pending, error, and success treatments where relevant.
- Spacing follows a rhythm; avoid identical padding everywhere.
- Text has readable measure and does not overflow at mobile or 200% zoom.
- Product UI uses consistent component vocabulary and disciplined accents.
- Brand UI has a specific visual idea and real imagery/illustration when content demands it.
- No gradient text, decorative glass, nested card stacks, side-stripe card accents, generic hero metrics, or endless equal icon-card grids.
- Mobile behavior is structural, not just squeezed desktop.

## Copy polish

- Remove repeated headings and intros.
- Prefer concrete verbs on buttons: "Save project", "Invite member", "Retry upload".
- Error copy says what happened and how to recover.
- Loading copy is truthful: "Saving...", "Checking availability...".
- Avoid em dashes and vague filler such as "seamless", "powerful", "easy-to-use" unless the product voice truly uses them.

## Validation

After edits, use the main `cheers` skill for formatting and checks. At minimum, format changed Cheers templates and run targeted tests/checks appropriate to the behavior changed.
