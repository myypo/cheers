import type { EventCallbackHandler, Modifiers } from '@engine/types'

type ViewTransitionTarget = (Document | Element) & {
  startViewTransition: (callback: () => void) => void
}

export const supportsViewTransitions = (
  target: Document | Element = document,
): target is ViewTransitionTarget => 'startViewTransition' in target

export const modifyViewTransition = (
  callback: EventCallbackHandler,
  mods: Modifiers,
): EventCallbackHandler => {
  if (mods.has('viewtransition') && supportsViewTransitions()) {
    const cb = callback // I hate javascript
    callback = (...args: any[]) =>
      document.startViewTransition(() => cb(...args))
  }

  return callback
}
