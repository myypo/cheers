# Colorize

Introduce color strategically in a Cheers UI. More color is not better. Color must carry meaning, hierarchy, or brand voice.

## Register

- **Product**: semantic-first, usually Restrained. Accent means primary action, selected state, focus, or status.
- **Brand**: color can be the voice. Committed, Full palette, and Drenched are valid when intentional.

## Assess

Identify:

- existing brand colors or CSS tokens
- current neutral temperature
- semantic states needed: success, error, warning, info, selected, disabled, loading
- places where color would improve wayfinding or hierarchy
- contrast failures or gray-on-color problems

## Plan

Choose a strategy before values:

1. Restrained: tinted neutrals plus one accent.
2. Committed: one saturated color owns much of the surface.
3. Full palette: 3-4 named roles with strict use.
4. Drenched: color is the environment.

Use OKLCH when possible and reduce chroma near very light or very dark values.

## Apply

### Product

- Primary button and active nav use the accent.
- Error/warning/success/info use consistent semantic colors.
- Focus rings are visible and color-safe.
- Loading indicators use the pending affordance color without implying success.
- Charts and badges use palettes that are distinguishable beyond color alone.

### Brand

- Let the chosen color strategy affect composition, not just buttons.
- Use image/illustration/color together where the brief calls for a visual world.
- Avoid default tech gradients and decorative glow.

## Cheers implementation

- Put palette values in CSS variables or the existing token system.
- Keep Datastar patched states using the same semantic classes/tokens.
- Do not use color-only status. Add text, icon, or shape.
- Do not use side-stripe card accents. Use full hairline borders, tints, leading icons, or labels.

## Verify

Check WCAG contrast, color blindness, dark/light variants if present, reduced motion, and all backend-patched states. Color should clarify what changed after a patch.
