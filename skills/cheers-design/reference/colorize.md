# Colorize

Introduce color as hierarchy, meaning, and atmosphere. Preserve confirmed brand and semantic conventions; do not replace a visual world under the guise of colorizing it.

Needs: the existing brand colors (DESIGN.md, tokens, assets). Ask only when a binding brand decision cannot be inferred.

## Visitor mode

- **Persuade + Experience:** color may carry the voice and own large regions when the selected world calls for it. Committed, Full palette, and Drenched are available.
- **Operate + Read:** color primarily encodes action, selection, status, wayfinding, and reading hierarchy. Rarity gives an accent force, but color still does real jobs in the shell. See [operate.md](operate.md#color).

If the task actually needs a new identity, route through [new-work.md](new-work.md) and DESIGN.md.

## Two isolated assessments

When a sub-agent tool is available and permitted, run these independently; otherwise run them yourself in this order. Do not let scan findings anchor the design assessment.

1. **Color assessment.** Read DESIGN.md, tokens, stylesheets, assets, current themes, and representative states. Answer each with a file, selector, or computed value:
   - Which colors are confirmed brand commitments?
   - What are the current surface, text, action, focus, and semantic roles, and where are they defined?
   - Where does grayscale obscure hierarchy or state?
   - Where does contrast fail, or does color alone carry meaning?
   - What light/dark, data-visualization, or category-reflex palette constraints apply?
   - Does the request ask for more color, or for a new identity?
2. **Mechanical scan.** Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) once against the running page for contrast and palette findings; also check colors set in `html!` templates by hand.

Synthesize both before editing.

## Choose a strategy

Before editing, name the color strategy (Restrained, Committed, Full palette, or Drenched, as in [new-work.md](new-work.md#4-commit-the-world)), the emotional temperature, the dominant relationship, the contrast range, and the dosage. Follow the brief and the selected world, not a fixed percentage rule.

Build roles, not a bag of swatches:

- canvas and elevated surfaces;
- primary and secondary text;
- action, focus, and selection;
- borders and separators;
- success, warning, error, info, and pending;
- data categories or scales when needed.

Use the project's existing color space. For a new web palette, prefer OKLCH so lightness and chroma move predictably. Choose hue from product meaning and the visual direction, never from the category's default association.

## Apply at system scale

Read [craft-floor.md](craft-floor.md) before the first edit; its contrast check and surface refusals (gradient text, decorative glass, side stripes) apply here and are not repeated.

- Let the strongest color own a deliberate region or role instead of scattering tiny accents.
- Keep the primary action easy to find; do not spend its color on decoration.
- Tint neutrals toward the world's hue for cohesion. A true neutral is valid when it serves the world, never as an unexamined default.
- On colored surfaces, derive secondary text from the foreground or surface hue, never a washed-out generic gray.
- Keep semantic meanings consistent, while respecting domain conventions instead of assuming fixed hues.
- For data, vary lightness, chroma, shape, label, or pattern so color is never the only code.
- In dark mode, design surface elevation and contrast explicitly; never invert the light theme mechanically.
- Define primitives and semantic tokens as CSS custom properties in the project's token file. Themes remap semantic roles; templates reference roles only:

```css
:root {
  --ink-900: oklch(22% 0.02 250);
  --sea-600: oklch(52% 0.16 235);
  --color-text: var(--ink-900);
  --color-action: var(--sea-600);
}
```

Decoration without a relationship to hierarchy, state, content, or the visual world is not a color strategy.

### Contrast and perception

Beyond the craft floor's text ratios, controls, icons, and focus indicators need 3:1 against their surroundings. Check computed pairs in interactive states, overlays, text on images, disabled content, and both themes; simulate common vision deficiencies. When deriving OKLCH ramps, reduce chroma near white and black. Prefer explicit colors over chains of translucent overlays whose contrast depends on context.

### Cheers fit

- The pending color is its own role and never borrows success. Success color appears only on backend-confirmed state; error color only on a real failure (backend or validation).
- Backend-rendered states use the same semantic tokens and classes as the component's normal state.
- Status never relies on color alone: pair it with text, an icon, shape, or position.
- Use `cheers` for rendering mechanics. Format changed templates with `cargo cheers fmt --rustfmt <files>`.

## Verify

Inspect in one batched round (desktop and mobile together), fix everything it shows in one batch, confirm with at most one more round, then rerun the scan once. Answer each item with rendered or source evidence; a bare "yes" is not verification.

- Every color has a stable role or a world-specific atmospheric purpose.
- Attention lands on the intended action, content, or state.
- The palette holds across quiet, dense, interactive, pending, error, and empty states.
- Light and dark themes are each composed, not mechanically inverted.
- Contrast and non-color cues pass in every relevant state.
- The result is recognizably this product, not a generic colorful treatment.

When the palette earns its place, hand off to `cheers-design polish`.
