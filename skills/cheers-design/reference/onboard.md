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

## Patterns

### Empty states

Every empty state should answer:

1. What will appear here?
2. Why does it matter?
3. What is the first action?

Render empty states from backend state. Do not fake objects in client signals unless they are clearly examples.

### Guided first action

Use normal forms/actions:

- template selection or seed data comes from backend state
- submit via generated `...Action`
- show `!indicator` while work is happening
- patch the confirmed created item or next step

### Contextual help

Use inline help, details/summary, popovers, or small local signals for visibility. Track dismissals in backend user preferences when they matter across sessions. Local storage is acceptable only for non-critical, device-local hints.

### Progress

Progress indicators should reflect backend-known completion when it affects account state. Do not mark steps complete optimistically.

## Avoid

- Forced long tours before users can work.
- Repeated tooltips that ignore dismissal.
- Modal-first teaching.
- Separate tutorial mode disconnected from real data unless the domain is high-stakes.
- Patronizing obvious explanations.

## Cheers implementation checklist

- Empty, first-run, and permission states are `Render` components.
- Actions use generated structs and backend-confirmed patches.
- Hints controlled by signals are local affordances only.
- Persistent onboarding completion belongs in backend state.
- Browser tests cover first successful action when Datastar behavior matters.

## Verify

Can a new user reach first value quickly? Can an experienced user skip? Are errors recoverable? Does the flow work with keyboard, mobile, slow network, and reduced motion?
