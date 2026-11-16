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
  autoStart?: boolean
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
  const [mode, setMode] = useState<RxMode>(() => {
    try { return localStorage.getItem('xlog.rx.mode') === 'FT4' ? 'FT4' : 'FT8' } catch { return 'FT8' }
  })
  const [pending, setPending] = useState(false)
  const [observation, setObservation] = useState(0)
  const [pollError, setPollError] = useState('')
  const [actionError, setActionError] = useState('')
  const [now, setNow] = useState(performance.now())
  const generation = useRef(0)
  const action = useRef(0)
  const pendingAction = useRef(false)
  const autoStarted = useRef('')
  const current = useRef({ api, key, scope })
  current.current = { api, key, scope }
  const visible = held.api === api && held.scope === key && scope.connected && scope.audioCapable
  const view = visible ? held.view : rxOff
  const leases = visible && scope.needsCapable && !pending
    ? held.leases : new Map<string, { evidence: string; deadline: number }>()

  useEffect(() => {
    const timer = setInterval(() => setNow(performance.now()), 100)
    return () => clearInterval(timer)
  }, [])
  useEffect(() => {
    if (!scope.needsCapable) {
      setHeld(old => ({ ...old, leases: revokeNeeds(old.leases) }))
    }
  }, [scope.needsCapable])

  function accept(next: RxView, started: number, acceptKey: string, acceptApi: RxApi) {
    const admitted = current.current
    if (admitted.api !== acceptApi || admitted.key !== acceptKey) return
    setHeld(old => {
      const previous = old.api === acceptApi && old.scope === acceptKey ? old.leases : new Map()
      const nextLeases = receiveNeeds(previous, next, started, admitted.scope.owner ?? undefined)
      return { api: acceptApi, scope: acceptKey, view: next,
        leases: admitted.scope.needsCapable && !pendingAction.current
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
        if (sequence === action.current) {
          accept(next, started, key, api); setPollError('')
          const admitted = current.current.scope
          if (admitted.autoStart && admitted.fresh && autoStarted.current !== key) {
            if (next.enabled) autoStarted.current = key
            else if (!pendingAction.current) await receiving(true)
          }
        }
      } catch (error) {
        if (token !== generation.current) return
        if (sequence === action.current) {
          // Read-only history survives an observation gap. Native revision and
          // session fences still invalidate facts; controls use scope.fresh.
          setPollError(errorText(error))
        }
      }
      if (token === generation.current) timer = setTimeout(poll, 500)
    }
    void poll()
    return () => { ++generation.current; clearTimeout(timer) }
  }, [api, key, scope.connected, scope.audioCapable])

  async function receiving(start: boolean) {
    const admitted = current.current.scope
    if (start && (!admitted.connected || !admitted.fresh || !admitted.audioCapable)) return
    if (!start) autoStarted.current = key // Ordinary Stop stays stopped in this connection.
    const token = generation.current, sequence = ++action.current
    const started = performance.now()
    pendingAction.current = true; setPending(true); setActionError('')
    setHeld(old => ({ ...old, leases: revokeNeeds(old.leases) }))
    try {
      const next = await (start ? api.start(mode) : api.stop())
      if (token === generation.current && sequence === action.current) {
        if (start && next.enabled) autoStarted.current = key
        accept(next, started, key, api)
      }
    } catch (error) {
      if (token === generation.current && sequence === action.current) setActionError(errorText(error))
    } finally {
      if (token === generation.current && sequence === action.current) {
        pendingAction.current = false; setPending(false); ++action.current
      }
    }
  }
  function selectMode(next: RxMode) {
    if (!view.enabled && !pending) {
      setMode(next)
      try { localStorage.setItem('xlog.rx.mode', next) } catch { /* optional preference */ }
    }
  }
  return { view, leases, mode, selectMode, pending, receiving, pollError, actionError, now, observation }
}
