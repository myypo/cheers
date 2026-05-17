# Typeset

Improve typography hierarchy, readability, and font strategy in a Cheers UI.

## Register

- **Product**: system fonts or one familiar sans are often correct. Use a tight fixed rem scale, clear label/body/title roles, and stable density.
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

### Product type

- Use semantic roles: label, body, title, heading, data, caption.
- Keep the scale tight and predictable.
- Do not use display fonts in labels, buttons, or dense data.
- Use tabular numerals where alignment matters.
- Prefer system or familiar sans stacks unless the design system says otherwise.

### Brand type

- Start from voice, not trend. Name what the type should feel like as a physical object.
- Use fluid display sizes and stronger contrast when the surface earns it.
- Choose distinctive type only when it supports the brand, not because it looks designed.
- Avoid reusing the same popular display families across unrelated projects.

### Readability details

- Body line-height: about 1.45-1.7.
- Heading line-height: about 1.0-1.2.
- Increase line-height slightly for light text on dark backgrounds.
- Use `font-variant-numeric: tabular-nums;` for aligned numbers.
- Use `rem`, not fixed pixel font sizes.
- Load only weights you actually use.

## Cheers fit

- Keep type roles in CSS/design docs, not arbitrary inline styles in templates.
- Ensure error, success, empty, and pending states use the same typographic roles as the rest of the component.
- Use `cheers` for exact rendering syntax.

## Verify

Check hierarchy, 200% zoom, mobile wrapping, long translations, contrast, and font loading. Finish with `polish` if spacing or state treatments still need work.
