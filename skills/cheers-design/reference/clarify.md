# Clarify

Rewrite unclear interface text in a Cheers UI so users understand what happened, what matters, and what to do next. Preserve factual meaning, product terminology, and brand voice. Clear copy reduces mistakes and makes backend-confirmed state understandable.

## Visitor mode

- **Persuade + Experience:** copy carries the voice. Headlines may be bold and specific, but the offer, the proof, and the action stay plain enough to act on within seconds.
- **Operate + Read:** plain language and stable terms first. Labels, statuses, and errors are scanned under pressure; cleverness costs time.

Refinement preserves: keep voice, terms the audience genuinely knows, and every factual claim. Ask before changing factual claims, legal meaning, or a term that may be domain-specific, and never add claims.

## Two isolated assessments

When a subagent tool is available and permitted, run these independently; otherwise run them yourself in this order.

1. **Language assessment.** Read the whole interaction path in context, not isolated strings. Infer audience knowledge and emotional state from PRODUCT.md and the surrounding UI; ask only when they do not answer it. Find:
   - vague labels and outcomes: "Submit", "Error", "Invalid input", "Done";
   - internal jargon, model names, or assumed knowledge;
   - missing consequences, recovery, or timing;
   - inconsistent nouns, verbs, and capitalization for the same concept;
   - redundancy: a heading and intro saying the same thing, helper text restating the control;
   - pending or success copy that claims more than the backend has confirmed;
   - tone that ignores stress, risk, success, or urgency;
   - text that breaks at realistic widths or in translation.
2. **Mechanical scan.** Run the [mechanical scan](../SKILL.md#mechanical-scan-optional) or check by hand: kickers and eyebrows, em dashes in product copy, icon-only controls without accessible names, placeholder-only labels, link text that fails out of context.

Keep scan results out of the first assessment, then synthesize both before editing.

## Set the message hierarchy

For each state, decide before rewriting:

1. the one fact the user needs now;
2. the action available next;
3. supporting context that changes the decision;
4. the tone for this moment.

Say each idea once. If the heading already explains the state, the introduction adds new information or disappears. When inconsistency spans the product, keep a short terminology glossary and apply it everywhere; do not vary words for literary effect in an interface.

## Rewrite by function

### Actions and navigation

- Use a specific verb and object when the outcome is not already obvious: "Save settings", "Invite member", "Retry upload". Describe the outcome, not the gesture.
- Use the same noun and verb for the same concept across navigation, headings, forms, and buttons.
- Destructive actions name the object and consequence: "Delete deployment `green-42`? This removes its logs from the dashboard." The confirm button repeats the action ("Delete deployment"), never `Yes`, `OK`, or `Submit`.
- Prefer backend-modeled undo over confirmation when recovery is safe.

### Forms

- Persistent visible labels; placeholders are examples, not labels. Bad: "DOB". Good: "Date of birth" with the hint "Use YYYY-MM-DD."
- Format and eligibility requirements appear before submission. Explain why information is requested only when it is not obvious. Treat required and optional consistently.
- Validation messages come from the backend, the source of truth for the action, and appear at the field: what needs attention and how to fix it, without blame.

When changing fields, use `cheers` to keep labels, hints, and errors wired to the form.

### Errors and permissions

An actionable error answers what failed, why when known and useful, and how to recover or what alternative remains.

- Bad: "Forbidden". Good: "You do not have permission to edit this project. Ask an owner for access."
- Never lead with internal codes or raw errors. Never promise a cause or resolution the system cannot know.
- Privacy, payment, deletion, access loss, and blocked work get serious copy; warmth is welcome, jokes are not.
- Newly appearing important errors are announced through live status, not only shown.

### Pending, empty, and success

- Pending text names the real operation: "Saving...", "Checking availability...". Set an honest expectation when the wait is meaningful; show determinate progress only when the backend provides it.
- Bad: "Done" before the response arrives. Good: "Saving..." while pending, then "Saved" or the error, only after backend confirmation.
- Empty states distinguish first use, no results, active filters, permissions, and failure; each explains the state and offers the next useful action.
- Success confirms the completed outcome and mentions the next consequence only when it changes what the user should do. Routine success stays brief.

### Help text

Helper text answers an implicit question instead of restating the control. Put uncommon detail behind progressive disclosure. Link text makes sense out of context; icon-only controls get accessible names that match their visible meaning.

## Voice, accessibility, and localization

Voice stays consistent; tone adapts to the moment. Use the product's own language, with no filler adjectives, em dashes, or generic loading jokes.

- Write complete translatable messages, never concatenated fragments; keep variables and numbers structured so translators can reorder them, and handle plurals properly.
- Allow for 30 to 40% expansion instead of abbreviating early.
- Alt text conveys the image's information; decorative images get empty alt.
- Screen-reader names match visible labels and outcomes.
- Punctuation, color, or an icon never carries the message alone.

## Verify

Read the flow in context, desktop and mobile in one batched round, and confirm with evidence:

- a first-time user knows what to do next without hidden product knowledge;
- errors, empty states, and decision points are actionable, specific, and blame-free;
- claims are factually unchanged and terms match everywhere;
- copy scans at target widths and 200% zoom and survives long names, translation, plurals, and dynamic values;
- accessible names and announced state changes match the visible copy;
- tone is appropriate to the consequence and emotional context;
- every success message renders only from backend-confirmed state.

Fix what the round shows in one batch and confirm with at most one more round. The final copy is as short as it can be without losing meaning or recovery. Format changed templates with `cargo cheers fmt --rustfmt <files>`. When the language reads cleanly, hand off to `cheers-design polish`.
