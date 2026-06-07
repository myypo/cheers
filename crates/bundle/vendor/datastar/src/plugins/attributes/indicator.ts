// Icon: streamline:signal-loading-remix
// Slug: Creates an indicator for whether an SSE request is in flight.
// Description: Creates a signal and sets its value to `true` while an SSE request request is in flight, otherwise `false`.

import { attribute } from '@engine'
import { DATASTAR_FETCH_EVENT } from '@engine/consts'
import { mergePath, parseSignalPath } from '@engine/signals'
import type { DatastarFetchEvent } from '@engine/types'
import { FINISHED, STARTED } from '@plugins/actions/fetch'
import { modifyCasing } from '@utils/text'

attribute({
  name: 'indicator',
  requirement: 'exclusive',
  apply({ el, key, mods, value }) {
    const signalName = key != null ? modifyCasing(key, mods) : value
    const signalPath = parseSignalPath(signalName)
    let activeFetches = 0

    mergePath(signalPath, false)

    const watcher = ((event: CustomEvent<DatastarFetchEvent>) => {
      const { type, el: elt } = event.detail
      if (elt !== el) {
        return
      }
      switch (type) {
        case STARTED:
          activeFetches++
          mergePath(signalPath, true)
          break
        case FINISHED:
          activeFetches = Math.max(0, activeFetches - 1)
          mergePath(signalPath, activeFetches > 0)
          break
      }
    }) as EventListener
    document.addEventListener(DATASTAR_FETCH_EVENT, watcher)
    return () => {
      activeFetches = 0
      mergePath(signalPath, false)
      document.removeEventListener(DATASTAR_FETCH_EVENT, watcher)
    }
  },
})
