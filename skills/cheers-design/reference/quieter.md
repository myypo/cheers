# Quieter

Reduce visual intensity in a Cheers UI that is too loud, aggressive, or overstimulating without losing personality or turning generic. Quiet design is harder than bold design: subtlety needs precision, and quiet without intent collapses to the category default.

## Visitor mode

- **Persuade + Experience:** a more restrained palette, more whitespace, more typographic air. Drama is reduced, not eliminated; the point of view stays intact.
- **Operate + Read:** less visual noise. Fewer background accents, flatter cards, less color, less motion. The tool should disappear more completely into the task.

Quieting refines the established world. If the user wants a different, calmer identity rather than a calmer surface, that is a redesign and routes through [new-work.md](new-work.md).

## Two isolated assessments

Run these in order, and do not let scan findings anchor the design assessment. With subagents available, run them independently.

1. **Design assessment:** find the intensity sources, with file, selector, or rendered evidence:
   - saturation: too many bright or fully saturated colors;
   - contrast extremes: high-contrast juxtaposition everywhere instead of where it matters;
   - weight: bold, heavy elements competing, with no clear anchor;
   - surfaces: too many borders, shadows, backgrounds, cards, or icons;
   - motion: dramatic or decorative effects where users are trying to work;
   - scale: everything large, no hierarchy;
   - copy: dense text or repeated explanations;
   - inconsistency: one component treated several ways.

   Then name what is working and the core message to preserve. Some contexts need energy; if purpose, audience, or what must survive is unclear from the code and briefs, ask rather than guess.
2. **Mechanical scan:** run the [mechanical scan](../SKILL.md#mechanical-scan-optional) over the target page.

Synthesize both before editing.

## State the system

Before editing, name the few elements that stay bold (very few), what recedes, what can be removed outright, and how restraint will signal quality. Keep the world's palette, faces, and motif; quieter changes their intensity, not their identity.

## Apply

Read [craft-floor.md](craft-floor.md) first.

### Color

- Desaturate rather than replace: shift loud hues toward 70–85% of their saturation instead of swapping them for gray.
- Use fewer colors more deliberately; let tinted neutrals do more work and color act as accent (about 10%).
- On Operate and Read surfaces move toward Restrained, reserving accent for primary action, selection, focus, and semantic status.
- Keep high contrast where it matters: text, controls, and status. Never gray text on color; use a darker shade of that color or transparency instead.

### Weight and type

- Step weights down (900 to 600, 700 to 500) and narrow scale jumps, but keep clear hierarchy and a few anchors.
- Let spacing and alignment carry emphasis that color and boldness carried before.
- Trim copy before shrinking it.

### Surfaces

- Remove decorative gradients, patterns, textures, glows, and stacked shadows.
- Remove unnecessary card wrappers; flatten layering where elevation does not communicate interaction or state.
- Replace heavy borders with spacing, tonal contrast, or hairlines. Simplify extreme radii and custom shapes.
- Bring rogue elements back to the grid and even out arbitrary spacing jumps.

### Motion

- Remove decorative animation; keep feedback, reveal, loading, and transition motion.
- Shorten distances (10–20px rather than 40px) and durations; ease out gently with ease-out-quart (see [animate.md](animate.md) tokens), never bounce or elastic.
- Reduced motion must still feel finished.

## Never

- Make everything the same size or weight; hierarchy still matters.
- Remove all color; quiet is not grayscale.
- Strip personality instead of refining it.
- Weaken affordances, focus, or contrast for calm.

## Cheers fit

Quieting must not hide state. Pending, error, disabled, focus-visible, and semantic status treatments stay visible, and backend-confirmed failures are never muted to make the UI calmer. Use `cheers` for implementation; format changed templates with `cargo cheers fmt --rustfmt <files>`.

## Verify

Inspect desktop and mobile in one batched round, fix everything in one batch, confirm with at most one more round. Answer each with rendered or source evidence:

- Users can still complete the task as easily, and controls read as controls.
- It still has character; the point of view survived the cuts.
- Long text is easier to read.
- Every state is still distinguishable, and the rerun scan has no unexplained findings.

If the result turned vague, run `cheers-design clarify` or `cheers-design layout`. When it feels right, hand off to `cheers-design polish`.
