# Onboard

Design first-run, empty, help, and activation flows that get users to value quickly in a Cheers app.

## Principle

Onboarding should get users to their first useful outcome, not teach the whole product. Use real product surfaces and backend-confirmed actions where possible.

## Discover

Ask or infer:

- What is the aha moment?
- What does a new user need before reaching it?
- What can be skipped by experienced users?
- Which empty states are common?
- What sample data, templates, or defaults can the backend provide?
- What fears or trust gaps appear before the first action?

## Patterns

### Empty states

Every empty state should answer:

1. What will appear here?
2. Why does it matter?
3. What is the first action?

Do not fake real objects unless they are clearly examples or templates.

### Guided first action

- Keep the first action small and reversible when possible.
- Use templates, seed data, or defaults when they reduce blank-page anxiety.
- Show truthful pending state while work is happening.
- Move to the confirmed created item or next step after success.

### Contextual help

Use inline help, details/summary, popovers, or small local disclosures. Persist dismissals in backend user preferences only when they matter across sessions. Device-local hints are acceptable for non-critical tips.

### Progress

Progress indicators should reflect backend-known completion when it affects account state. Do not mark steps complete optimistically.

## Avoid

- Forced long tours before users can work.
- Repeated tooltips that ignore dismissal.
- Modal-first teaching.
- Separate tutorial mode disconnected from real data unless the domain is high-stakes.
- Patronizing obvious explanations.

## Cheers fit

- Empty, first-run, and permission states should be first-class UI patterns.
- Local hint visibility can be a client affordance; durable completion belongs in backend state.
- Use `cheers` for exact action, patch, and test mechanics.

## Verify

Can a new user reach first value quickly? Can an experienced user skip? Are errors recoverable? Does the flow work with keyboard, mobile, slow network, and reduced motion?
