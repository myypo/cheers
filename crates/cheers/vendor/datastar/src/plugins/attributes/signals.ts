// Icon: streamline:wifi-signal-full-remix
// Slug: Patches signals into the existing signals.
// Description: Patches (adds, updates or removes) one or more signals into the existing signals.

import { attribute } from '@engine'
import { mergePatch } from '@engine/signals'

attribute({
  name: 'signals',
  requirement: {
    key: 'denied',
    value: 'must',
  },
  returnsValue: true,
  apply({ mods, rx }) {
    const ifMissing = mods.has('ifmissing')
    const patch = Object.assign({}, rx?.() as Record<string, any>)
    mergePatch(patch, { ifMissing })
  },
})
