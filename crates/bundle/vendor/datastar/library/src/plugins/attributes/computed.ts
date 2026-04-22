// Icon: streamline-ultimate:wifi-signal-2
// Slug: Creates a computed signal.
// Description: Creates a signal that is computed based on an expression.

import { attribute } from '@engine'
import { computed, mergePatch } from '@engine/signals'
import { updateLeaves } from '@utils/paths'

attribute({
  name: 'computed',
  requirement: {
    key: 'denied',
    value: 'must',
  },
  returnsValue: true,
  apply({ rx, error }) {
    const patch = Object.assign({}, rx() as Record<string, () => any>)
    updateLeaves(patch, (old) => {
      if (typeof old === 'function') {
        return computed(old)
      } else {
        throw error('ComputedExpectedFunction')
      }
    })
    mergePatch(patch)
  },
})
