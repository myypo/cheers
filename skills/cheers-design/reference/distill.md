# Distill

Strip a Cheers UI to its essence. Remove obstacles, not necessary capability.

## Assess

Find complexity sources:

- too many primary actions
- repeated explanations
- redundant cards, wrappers, or borders
- too many colors, fonts, variants, and statuses
- all data visible at once with no hierarchy
- client state or signals carrying UI complexity that could be rendered by backend state
- modal flows where inline or progressive disclosure would work

Identify the one primary user goal before cutting.

## Simplify

### Information architecture

- Keep one primary action per surface.
- Move rare actions to secondary placement.
- Collapse advanced controls behind clear disclosure.
- Combine duplicate concepts and terms.
- Prefer recognition over recall: show current state and next step.

### Visual structure

- Remove decorative containers.
- Replace nested cards with spacing, headings, and dividers.
- Reduce palette and type roles.
- Keep enough hierarchy for the primary action to remain obvious.

### Interaction

- Prefer normal navigation and forms where dynamic behavior adds little.
- Use a single action patch rather than several coordinated client-side updates when possible.
- Use signals only for local affordances.
- Remove unnecessary JS helpers.

### Copy

Cut repeated intro text and vague claims. Preserve specific labels, hints, and error recovery copy.

## Do not

- Remove accessible labels or status messages.
- Hide necessary error or permission information.
- Remove backend validation because the UI looks simpler.
- Oversimplify a genuinely complex expert workflow.

## Verify

Task completion should be faster, the primary action clearer, and code/state simpler. If simplification removed needed state coverage, run `harden` next.
