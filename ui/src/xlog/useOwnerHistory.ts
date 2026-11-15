// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useMemo, useRef, useState } from 'react'
import { historyReads, type View } from './api'
import { createOwnerHistoryReader, type OwnerHistoryQuery, type OwnerReadBinding } from './ownerHistoryReader'
import { canonicalContactId, ownerRecallRead, ownerRecentRead } from './ownerHistoryPresentation'

/** One reader shares revision retirement between Recall and Recent. A successful
 * cycle refreshes after two seconds; failures pause until a new selection,
 * connection or explicit refresh. Only the latest desired selection is retained
 * while a request is in flight: no backlog, overlapping request or stale replay. */
export function useOwnerHistory(view: View, session: number, query: OwnerHistoryQuery | null,
  transport = historyReads) {
  const reader = useMemo(() => createOwnerHistoryReader({ transport }), [transport])
  const binding: OwnerReadBinding = { sessionGeneration: session,
    connectionToken: view.connection_token ?? null, owner: view }
  const bindingKey = JSON.stringify([session, view.connection_token, view.paired, view.connected,
    view.hello?.server_instance, view.hello?.workspace_id, view.hello?.capabilities.includes('logbook.read'),
    view.hello?.capabilities.includes('logbook.contact.read')])
  const queryKey = JSON.stringify(query)
  const [, refresh] = useState(0)
  const [request, requestRefresh] = useState(0)
  const desiredKey = JSON.stringify([bindingKey, queryKey, request])
  const desired = useRef({ key: desiredKey, version: 0, binding, query })
  if (desired.current.key !== desiredKey) desired.current = {
    key: desiredKey, version: desired.current.version + 1, binding, query,
  }
  const cycle = useMemo(() => ({ active: false, visible: !document.hidden, busy: false, lifecycle: 0,
    pausedVersion: null as number | null,
    timer: null as ReturnType<typeof setTimeout> | null }), [reader])
  const pump = useRef<() => void>(() => {})
  pump.current = () => {
    if (!cycle.active || !cycle.visible || cycle.busy || cycle.pausedVersion === desired.current.version) return
    if (cycle.timer !== null) clearTimeout(cycle.timer)
    cycle.timer = null
    cycle.busy = true
    const admitted = desired.current, lifecycle = cycle.lifecycle
    const same = () => cycle.active && cycle.lifecycle === lifecycle && desired.current.version === admitted.version
    void (async () => {
      let succeeded = true
      try {
        if (admitted.query) {
          const result = await reader.readHistory(admitted.binding, admitted.query)
          succeeded = result.state === 'current'
          if (same()) refresh(value => value + 1)
        }
        if (same() && cycle.visible && succeeded) {
          const result = await reader.readRecent(admitted.binding)
          succeeded = result.state === 'current'
          if (same()) refresh(value => value + 1)
        }
      } finally {
        cycle.busy = false
        if (!cycle.active) return
        if (same()) cycle.pausedVersion = succeeded ? null : admitted.version
        if (!cycle.visible) return
        if (!same()) {
          // One latest request after retirement, including A → B → A. There is
          // no queue of intermediate selections and no retry of a failed read.
          queueMicrotask(() => pump.current())
        } else if (succeeded) cycle.timer = setTimeout(() => pump.current(), 2000)
      }
    })()
    refresh(value => value + 1)
  }
  reader.synchronize(binding)
  const history = reader.inspectHistory(binding, query ?? { callsign: '' })
  const recent = reader.inspectRecent(binding)
  useEffect(() => {
    cycle.active = true; cycle.lifecycle++
    cycle.visible = !document.hidden
    pump.current()
    const visibility = () => {
      cycle.visible = !document.hidden
      if (cycle.timer !== null) clearTimeout(cycle.timer)
      cycle.timer = null
      if (cycle.visible) pump.current()
    }
    document.addEventListener('visibilitychange', visibility)
    let inspected = ''
    const timer = setInterval(() => {
      const next = desired.current
      const h = reader.inspectHistory(next.binding, next.query ?? { callsign: '' })
      const r = reader.inspectRecent(next.binding)
      const key = JSON.stringify([h.state, h.contextKey, h.queryKey,
        h.state === 'current' ? h.deadline : h.state === 'unavailable' ? h.reason : '',
        r.state, r.contextKey, r.state === 'current' ? r.deadline : r.state === 'unavailable' ? r.reason : ''])
      if (key !== inspected) { inspected = key; refresh(value => value + 1) }
    }, 100)
    return () => {
      cycle.active = false; cycle.lifecycle++
      document.removeEventListener('visibilitychange', visibility)
      clearInterval(timer)
      if (cycle.timer !== null) clearTimeout(cycle.timer)
      reader.invalidate()
    }
  }, [reader, cycle])
  useEffect(() => { pump.current() }, [desiredKey, cycle])
  const contactRead = view.connected && !!view.hello?.capabilities.includes('logbook.contact.read')
  const canOpenContact = (call: string, id: string | null | undefined) => {
    if (!canonicalContactId(id)) return false
    const current = desired.current
    if (!current.binding.owner.connected || !current.binding.owner.hello?.capabilities.includes('logbook.contact.read')) return false
    const h = reader.inspectHistory(current.binding, current.query ?? { callsign: '' })
    const r = reader.inspectRecent(current.binding)
    // Recall supplies the queried call (which can differ from a portable row).
    if (h.state === 'current' && h.payload.callsign === call
      && h.payload.contacts.filter(row => row.contact_id === id).length === 1) return true
    return r.state === 'current' && r.payload.contacts.filter(row => row.contact_id === id).length === 1
      && r.payload.contacts.some(row => row.contact_id === id && row.callsign === call)
  }
  return { recall: ownerRecallRead(history, true, contactRead), recent: ownerRecentRead(recent, contactRead), canOpenContact,
    refresh: () => { reader.invalidate(); requestRefresh(value => value + 1) } }
}
