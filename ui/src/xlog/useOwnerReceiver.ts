// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import { rxApi, rxOff, type RxApi, type RxMode, type RxView } from './rxApi'
import { receiveNeeds, revokeNeeds, type NeedsLeases, type OwnerIdentity } from './rxNeeds'

interface Scope {
  session: number
  owner: OwnerIdentity | null
  connected: boolean
  fresh: boolean
  audioCapable: boolean
  needsCapable: boolean
}
interface Held {
  api: RxApi
  scope: string
  view: RxView
  leases: NeedsLeases
}
const errorText = (error: unknown) => error instanceof Error ? error.message : String(error)

/** Native worker observation for the original cockpit. Polling never starts RX.
 * The native worker owns audio; no PCM or device handle crosses into the UI. */
export function useOwnerReceiver(scope: Scope, api: RxApi = rxApi) {
  const key = JSON.stringify([scope.session, scope.owner?.server_instance, scope.owner?.workspace_id])
  const [held, setHeld] = useState<Held>({ api, scope: key, view: rxOff, leases: new Map() })
  const [mode, setMode] = useState<RxMode>('FT8')
  const [pending, setPending] = useState(false)
  const [observation, setObservation] = useState(0)
  const [pollError, setPollError] = useState('')
  const [actionError, setActionError] = useState('')
  const [now, setNow] = useState(performance.now())
  const generation = useRef(0)
  const action = useRef(0)
  const pendingAction = useRef(false)
  const current = useRef({ api, key, scope })
  current.current = { api, key, scope }
  const visible = held.api === api && held.scope === key && scope.connected && scope.audioCapable
  const view = visible ? held.view : rxOff
  const leases = visible && scope.fresh && scope.needsCapable && !pending
    ? held.leases : new Map<string, { evidence: string; deadline: number }>()

  useEffect(() => {
    const timer = setInterval(() => setNow(performance.now()), 100)
    return () => clearInterval(timer)
  }, [])
  useEffect(() => {
    if (!scope.fresh || !scope.needsCapable) {
      setHeld(old => ({ ...old, leases: revokeNeeds(old.leases) }))
    }
  }, [scope.fresh, scope.needsCapable])

  function accept(next: RxView, started: number, acceptKey: string, acceptApi: RxApi) {
    const admitted = current.current
    if (admitted.api !== acceptApi || admitted.key !== acceptKey) return
    setHeld(old => {
      const previous = old.api === acceptApi && old.scope === acceptKey ? old.leases : new Map()
      const nextLeases = receiveNeeds(previous, next, started, admitted.scope.owner ?? undefined)
      return { api: acceptApi, scope: acceptKey, view: next,
        leases: admitted.scope.fresh && admitted.scope.needsCapable && !pendingAction.current
          ? nextLeases : revokeNeeds(nextLeases) }
    })
    if (next.mode) setMode(next.mode)
    if (!pendingAction.current) setObservation(value => value + 1)
  }

  useEffect(() => {
    const token = ++generation.current
    pendingAction.current = false
    setPending(false); setPollError(''); setActionError('')
    setHeld({ api, scope: key, view: rxOff, leases: new Map() })
    if (!scope.connected || !scope.audioCapable) return
    let timer: ReturnType<typeof setTimeout>
    const poll = async () => {
      const sequence = action.current, started = performance.now()
      try {
        const next = await api.status()
        if (token !== generation.current) return
        if (sequence === action.current) { accept(next, started, key, api); setPollError('') }
      } catch (error) {
        if (token !== generation.current) return
        if (sequence === action.current) {
          setHeld(old => ({ ...old, leases: revokeNeeds(old.leases) }))
          setPollError(errorText(error))
        }
      }
      if (token === generation.current) timer = setTimeout(poll, 500)
    }
    void poll()
    return () => { ++generation.current; clearTimeout(timer) }
  }, [api, key, scope.connected, scope.audioCapable])

  async function receiving(start: boolean) {
    if (start && (!scope.connected || !scope.fresh || !scope.audioCapable)) return
    const token = generation.current, sequence = ++action.current
    const started = performance.now()
    pendingAction.current = true; setPending(true); setActionError('')
    setHeld(old => ({ ...old, leases: revokeNeeds(old.leases) }))
    try {
      const next = await (start ? api.start(mode) : api.stop())
      if (token === generation.current && sequence === action.current) accept(next, started, key, api)
    } catch (error) {
      if (token === generation.current && sequence === action.current) setActionError(errorText(error))
    } finally {
      if (token === generation.current && sequence === action.current) {
        pendingAction.current = false; setPending(false); ++action.current
      }
    }
  }
  function selectMode(next: RxMode) {
    if (!view.enabled && !pending) setMode(next)
  }
  return { view, leases, mode, selectMode, pending, receiving, pollError, actionError, now, observation }
}
