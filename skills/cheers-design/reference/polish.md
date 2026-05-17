# Polish

Refine an existing Cheers UI to shipping quality. Polish is not a rewrite unless the audit reveals structural problems.

## Order of work

1. **Confirm intent**: primary action, register, target users, and release bar.
2. **Inspect the rendered UI** when practical, not just the source.
3. **Fix correctness first**: no optimistic UI, generated actions/ids/forms, missing `Scripts`, broken patches.
4. **Fix state coverage**: empty, loading, error, success, disabled, permission, overflow, long text.
5. **Fix accessibility**: semantics, labels, focus, keyboard, status announcements, contrast.
6. **Fix visual craft**: hierarchy, spacing, alignment, typography, color, density, responsive behavior.
7. **Remove noise**: redundant copy, placeholder content, dead controls, decorative effects that do not help.
8. **Validate and format**.

## Cheers polish checklist

- Generated names are destructured inside `render_to` with `self.ids()`, `self.signals()`, and `self.form_names()`.
- Action attrs use generated `...Action` structs.
- Patch targets have stable generated ids or semantic ids.
- `PatchElements` defaults are used unless a non-default target/mode is necessary.
- Signals are local/sparse; no broad backend model mirrored into the client.
- `!indicator` is used for request pending state where useful.
- Buttons that trigger requests are disabled or protected against repeated submission when needed.
- Success/error is rendered from backend-confirmed state.
- Forms preserve user input and show server validation errors near fields.
- Pages using Datastar attrs include `Scripts`.

## Visual polish checklist

- One primary action is visually and semantically clear.
- Controls have default, hover, focus-visible, active, disabled, loading, error, and success states where relevant.
- Spacing follows a rhythm; avoid identical padding everywhere.
- Text has readable measure and does not overflow at mobile or 200% zoom.
- Product UI uses consistent component vocabulary and restrained accents.
- Brand UI has a specific visual idea and real imagery/illustration when content demands it.
- No gradient text, decorative glass, nested card stacks, side-stripe card accents, generic hero metrics, or endless equal icon-card grids.
- Mobile behavior is structural, not just squeezed desktop.

## Copy polish

- Remove repeated headings/intros.
- Prefer concrete verbs on buttons: "Save project", "Invite member", "Retry upload".
- Error copy says what happened and how to recover.
- Loading copy is truthful: "Saving...", "Checking availability...".
- Avoid em dashes and vague filler like "seamless", "powerful", "easy-to-use" unless the product voice truly uses them.

## Validation

After edits, run:

```bash
cargo cheers fmt --rustfmt <edited-files-or-directories>
```

Then run targeted tests/checks. For client behavior, prefer `cheers::test::App` only when render or handler tests cannot prove the behavior.
