// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import { ftApi, ftOff, type FtAction, type FtApi, type FtView } from './ftApi'
export interface FtScope { session: number; connectionToken: string | null; connected: boolean; capable: boolean }
interface Held { key: string; api: FtApi; view: FtView; deadline: number }
const message = (error: unknown) => error instanceof Error ? error.message : String(error)
const release = (action: FtAction) => action.kind === 'stop' ||
  ((action.kind === 'enable' || action.kind === 'tune') && !action.enabled)

/** Polls observe only. No effect arms TX, starts CQ, or retries a mutation.
 * Native scheduling/heartbeat/Stop continue independently of this React hook. */
export function useOwnerTransmitter(scope: FtScope, api: FtApi = ftApi) {
  const key = JSON.stringify([scope.session, scope.connectionToken])
  const [held, setHeld] = useState<Held>({ key, api, view: ftOff, deadline: 0 })
  const [now, setNow] = useState(performance.now())
  const [error, setError] = useState('')
  const [pollError, setPollError] = useState('')
  const [busy, setBusy] = useState(false)
  const [logNotice, setLogNotice] = useState('')
  const current = useRef({ key, scope, api }); current.current = { key, scope, api }
  const generation = useRef(0), actionSequence = useRef(0), ordinaryBusy = useRef(false)
  const visible = held.key === key && held.api === api && held.view.connection_token === scope.connectionToken
  const view = visible ? held.view : ftOff
  const fresh = visible && scope.connected && scope.capable && held.deadline > now
  const accept = (next: FtView, started: number, capturedKey: string, capturedApi: FtApi) => {
    if (current.current.key !== capturedKey || current.current.api !== capturedApi ||
      next.connection_token !== current.current.scope.connectionToken ||
      !Number.isSafeInteger(next.remaining_valid_ms) || next.remaining_valid_ms < 0 || next.remaining_valid_ms > 1000) return
    setHeld({ key: capturedKey, api: capturedApi, view: next, deadline: started + next.remaining_valid_ms })
  }
  useEffect(() => { const timer = setInterval(() => setNow(performance.now()), 100); return () => clearInterval(timer) }, [])
  useEffect(() => {
    const token = ++generation.current
    ordinaryBusy.current = false; setBusy(false); setError(''); setPollError(''); setLogNotice('')
    setHeld(old => old.key === key && old.api === api ? { ...old, deadline: 0 } : { key, api, view: ftOff, deadline: 0 })
    if (!scope.connected || !scope.capable || !scope.connectionToken) return
    let timer: ReturnType<typeof setTimeout>
    const poll = async () => {
      const sequence = actionSequence.current, started = performance.now()
      try {
        const next = await api.status(scope.connectionToken!)
        if (token !== generation.current) return
        if (sequence === actionSequence.current) { accept(next, started, key, api); setPollError('') }
      } catch (cause) {
        if (token !== generation.current || sequence !== actionSequence.current) return
        setHeld(old => ({ ...old, deadline: 0 })); setPollError(message(cause))
      }
      if (token === generation.current) timer = setTimeout(poll, 200)
    }
    void poll()
    return () => { ++generation.current; clearTimeout(timer) }
  }, [key, api, scope.connected, scope.capable])

  async function action(intent: FtAction) {
    const captured = current.current, connectionToken = captured.scope.connectionToken
    if (!connectionToken) return
    const stopping = release(intent)
    if (!stopping) {
      // A cached status picture is display data, not permission for an explicit
      // operator command. Native dispatch obtains the current owner context.
      if (!captured.scope.capable) {
        setError('This xLog connection does not provide transmit control.'); return
      }
      if (ordinaryBusy.current) { setError('A transmit control request is pending.'); return }
    }
    const token = generation.current, sequence = ++actionSequence.current, started = performance.now()
    if (!stopping) { ordinaryBusy.current = true; setBusy(true) }
    setError('')
    try {
      const next = await captured.api.action(connectionToken, intent)
      if (token === generation.current && sequence === actionSequence.current) accept(next, started, captured.key, captured.api)
    } catch (cause) {
      if (token === generation.current && sequence === actionSequence.current) {
        setHeld(old => ({ ...old, deadline: 0 })); setError(message(cause))
      }
    } finally {
      if (!stopping && token === generation.current) { ordinaryBusy.current = false; setBusy(false) }
    }
  }
  async function log() {
    const captured = current.current
    if (!captured.scope.connectionToken || !captured.scope.connected || ordinaryBusy.current) return
    const token = generation.current
    ordinaryBusy.current = true; setBusy(true); setError('')
    try {
      const entries = await captured.api.log(captured.scope.connectionToken)
      if (token === generation.current) {
        const entry = entries.length === 1 ? entries[0] : null
        setLogNotice(entry == null ? 'No QSO submission receipt.' : typeof entry.state === 'object'
          ? `xLog QSO: ${entry.state.Settled.status}`
          : entry.state === 'NeedsReceipt' ? 'QSO outcome pending; check its xLog receipt.' : 'QSO queued for xLog.')
      }
    }
    catch (cause) { if (token === generation.current) setError(message(cause)) }
    finally { if (token === generation.current) { ordinaryBusy.current = false; setBusy(false) } }
  }
  return { view, fresh, error: error || pollError, busy, action, log, logNotice }
}
