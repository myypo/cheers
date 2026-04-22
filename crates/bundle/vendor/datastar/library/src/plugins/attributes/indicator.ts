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

    mergePath(signalPath, false)

    const watcher = ((event: CustomEvent<DatastarFetchEvent>) => {
      const { type, el: elt } = event.detail
      if (elt !== el) {
        return
      }
      switch (type) {
        case STARTED:
          mergePath(signalPath, true)
          break
        case FINISHED:
          mergePath(signalPath, false)
          break
      }
    }) as EventListener
    document.addEventListener(DATASTAR_FETCH_EVENT, watcher)
    return () => {
      mergePath(signalPath, false)
      document.removeEventListener(DATASTAR_FETCH_EVENT, watcher)
    }
  },
})
