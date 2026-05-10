# Typeset

Improve typography hierarchy, readability, and font strategy in a Cheers UI.

## Register

- **Product**: system fonts or one familiar sans are often correct. Use a tight fixed `rem` scale, clear label/body/title roles, and stable density.
- **Brand**: typography can carry the voice. Use stronger scale contrast, distinctive headings, and fluid display sizes when the surface earns it.

## Assess

Check:

- Are headings, body, labels, captions, code/data, and buttons visually distinct?
- Is the scale too flat?
- Is body text at least `1rem` and readable at 200% zoom?
- Are line lengths reasonable, especially prose at 65-75ch?
- Are too many font families or weights loaded?
- Do data columns need tabular numbers?
- Does type overflow in patched states, validation errors, empty states, or long localized labels?

## Improve

### Product type scale

Use fixed roles and semantic CSS variables:

```css
:root {
  --font-ui: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  --text-label: .8125rem;
  --text-body: 1rem;
  --text-title: 1.125rem;
  --text-heading: 1.375rem;
}
```

Use 1.125-1.2 ratio between steps. Do not use display fonts in labels, buttons, or dense data.

### Brand type scale

Use fluid display sizes and stronger contrast:

```css
:root {
  --text-display: clamp(3rem, 9vw, 7rem);
  --text-kicker: .78rem;
}
```

Choose type that matches the actual brand voice, not generic elegance. Avoid defaulting to the same popular display families across projects.

### Readability details

- Body line-height: about 1.45-1.7.
- Heading line-height: about 1.0-1.2.
- Increase line-height slightly for light text on dark backgrounds.
- Use `font-variant-numeric: tabular-nums;` for aligned numbers.
- Use `rem`, not fixed pixel font sizes.
- Load only weights you actually use.

## Cheers considerations

- Keep type roles in CSS/design docs, not duplicated as arbitrary inline styles in templates.
- Ensure patched error/success/empty states use the same typographic roles as the rest of the component.
- Render borrowed strings with `(@&value)` when only displaying text.

## Verify

Check hierarchy, 200% zoom, mobile wrapping, long translations, contrast, and font loading. Finish with `polish` if spacing or state treatments still need work.
