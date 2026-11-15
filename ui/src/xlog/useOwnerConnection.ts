// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import { bridge, disconnected, type Bridge, type Command, type Receipt, type View } from './api'
import { leaseSnapshot, type SnapshotLease } from './ownerOperatingModel'
import type { ControlPoll, ControlScope } from './controlObservation'
import { createPointEvidence, type PointEvidenceView } from './pointEvidence'

const message = (error: unknown) => error instanceof Error ? error.message : String(error)

export interface OwnerConnectionScope { session: number; connectionToken: string; serverInstance: string; workspaceId: string }

/** Connection lifecycle only. Mounting reads status; it never pairs, starts RX,
 * changes a radio, opens a feed, or starts the upstream station engine. */
export function useOwnerConnection(api: Bridge = bridge) {
  const [heldView, holdView] = useState({ api, value: disconnected })
  const view = heldView.api === api ? heldView.value : disconnected
  const setView = (next: View | ((old: View) => View)) => holdView(old => ({ api,
    value: typeof next === 'function' ? next(old.api === api ? old.value : disconnected) : next }))
  const [heldLease, holdLease] = useState<{ api: Bridge; value: SnapshotLease | null }>({ api, value: null })
  const lease = heldLease.api === api ? heldLease.value : null
  const admittedLease = useRef<SnapshotLease | null>(null)
  const ordinaryPending = useRef(false)
  const controlGeneration = useRef(0)
  const setLease = (value: SnapshotLease | null) => { admittedLease.current = value; holdLease({ api, value }) }
  const currentApi = useRef(api)
  currentApi.current = api
  const [session, setSession] = useState(0)
  const [busy, setBusy] = useState(false)
  const [pollError, setPollError] = useState('')
  const [actionError, setActionError] = useState('')
  const [receipt, setReceipt] = useState<Receipt | null>(null)
  const [now, setNow] = useState(performance.now())
  const epoch = useRef(0)
  const commandSequence = useRef(0)
  const live = useRef(true)
  const acceptedView = useRef<View>(disconnected)
  const lastPoll = useRef<ControlPoll | null>(null)
  const pollBarrier = useRef(0)
  const stopsPending = useRef(0)
  const point = useRef<ReturnType<typeof createPointEvidence> | null>(null)
  const [, changedEvidence] = useState(0)
  const evidenceChanged = () => { if (live.current) changedEvidence(value => value + 1) }
  const currentControlScope = (): ControlScope | null => {
    const accepted = acceptedView.current
    return live.current && currentApi.current === api && heldView.api === api && accepted.paired && accepted.connection_token && accepted.hello
      ? { connectionToken: accepted.connection_token, sessionGeneration: epoch.current,
        serverInstance: accepted.hello.server_instance, workspaceId: accepted.hello.workspace_id } : null
  }
  const retireEvidence = () => { point.current?.retire(); lastPoll.current = null; pollBarrier.current++ }
  const isCurrentConnection = (scope: OwnerConnectionScope) => {
    const accepted = acceptedView.current
    return live.current && currentApi.current === api && epoch.current === scope.session && accepted.paired
      && accepted.connection_token === scope.connectionToken && accepted.hello?.server_instance === scope.serverInstance
      && accepted.hello.workspace_id === scope.workspaceId
  }
  const accept = (next: View, started: number, control = controlGeneration.current, pendingAtStart = ordinaryPending.current) => {
    acceptedView.current = next
    setView(next)
    const nextLease = !pendingAtStart && !ordinaryPending.current && control === controlGeneration.current
      ? leaseSnapshot(next, started, Date.now()) : null
    setLease(nextLease)
    point.current?.inspect(currentControlScope())
    return nextLease
  }
  useEffect(() => {
    live.current = true
    const generation = ++epoch.current
    acceptedView.current = disconnected
    retireEvidence(); stopsPending.current = 0
    ordinaryPending.current = false; controlGeneration.current++
    setSession(generation); setView(disconnected); setLease(null)
    setPollError(''); setActionError(''); setReceipt(null); setBusy(false)
    const started = performance.now()
    void api.status().then(next => {
      if (live.current && currentApi.current === api && generation === epoch.current) accept(next, started)
    }, error => { if (live.current && currentApi.current === api && generation === epoch.current) setPollError(message(error)) })
    return () => { live.current = false; ++epoch.current; retireEvidence() }
  }, [api])
  useEffect(() => {
    const timer = setInterval(() => setNow(performance.now()), 100)
    return () => clearInterval(timer)
  }, [])
  useEffect(() => {
    if (!view.connected) return
    const generation = epoch.current
    let active = true
    let timer: ReturnType<typeof setTimeout>
    const poll = async () => {
      const started = performance.now()
      const control = controlGeneration.current, pendingAtStart = ordinaryPending.current
      const barrier = pollBarrier.current, stopAtStart = stopsPending.current > 0
      try {
        const next = await api.poll()
        if (!active || currentApi.current !== api || generation !== epoch.current) return
        const nextLease = accept(next, started, control, pendingAtStart)
        const scope = currentControlScope()
        // Only this actual api.poll call establishes read provenance. Initial
        // status/connect replies can supply display facts, never a Point baseline.
        if (nextLease && scope && barrier === pollBarrier.current && !stopAtStart && stopsPending.current === 0) {
          const observed: ControlPoll = { scope, origin: 'poll', requestStartedMono: started,
            deadline: nextLease.deadline, snapshot: nextLease.snapshot }
          lastPoll.current = observed; point.current?.observe(observed)
        } else lastPoll.current = null
        // A known route replacement retires observation even while Point is
        // pending or this poll is inadmissible as a post-command observation.
        const intent = point.current?.inspect(scope).intent
        if (intent && next.snapshot?.rotor.rotor_id !== intent.command.rotor_id) point.current?.retire()
        setPollError('')
      } catch (error) {
        if (!active || currentApi.current !== api || generation !== epoch.current) return
        setLease(null); setPollError(message(error))
        lastPoll.current = null
        setView(old => ({ ...old, connected: false, snapshot: null }))
        return
      }
      if (active && generation === epoch.current) timer = setTimeout(poll, 500)
    }
    timer = setTimeout(poll, 500)
    return () => { active = false; clearTimeout(timer) }
  }, [api, view.connected, session])

  async function connection(kind: 'import' | 'connect' | 'disconnect' | 'forget') {
    if (currentApi.current !== api) return
    const generation = ++epoch.current
    acceptedView.current = disconnected
    retireEvidence(); stopsPending.current = 0
    ordinaryPending.current = false; controlGeneration.current++
    setSession(generation); setBusy(true); setActionError(''); setPollError(''); setReceipt(null)
    setLease(null); setView(old => ({ ...old, connected: false, snapshot: null }))
    const started = performance.now()
    try {
      const next = kind === 'import' ? await api.importPairing() : kind === 'connect'
        ? await api.connect() : await api.disconnect(kind === 'forget')
      if (!live.current || currentApi.current !== api || generation !== epoch.current) return
      if (next) accept(next, started)
      else {
        const statusStarted = performance.now()
        const status = await api.status()
        if (live.current && currentApi.current === api && generation === epoch.current) accept(status, statusStarted)
      }
    } catch (error) {
      if (live.current && currentApi.current === api && generation === epoch.current) setActionError(message(error))
    } finally {
      if (live.current && currentApi.current === api && generation === epoch.current) setBusy(false)
    }
  }

  async function dispatch(value: Command, requestId?: Parameters<Bridge['command']>[1], observed?: ReturnType<typeof createPointEvidence>): Promise<void> {
    if (currentApi.current !== api || heldView.api !== api) throw new Error('The xLog connection changed.')
    const ordinary = value.kind !== 'rotor.stop'
    const currentLease = admittedLease.current
    // The owner remains the final capability/admission authority. The host also
    // refuses ordinary commands after its displayed observations have expired.
    if (ordinary && (ordinaryPending.current || !view.connected || !currentLease || currentLease.deadline <= performance.now())) {
      throw new Error('Wait for a current xLog station observation before controlling it.')
    }
    if (observed && (!acceptedView.current.hello?.capabilities.includes('rotor.point') || !observed.submit(currentControlScope()))) {
      throw new Error('A current, capable xLog poll and exact rotor target are required before Point.')
    }
    const generation = epoch.current, sequence = ++commandSequence.current
    lastPoll.current = null; pollBarrier.current++
    if (!ordinary) stopsPending.current++
    setReceipt(null); setActionError('')
    if (ordinary) {
      // Synchronous admission: a second handler in the same React turn cannot
      // reuse its old lease. Emergency rotor Stop bypasses this ordinary gate.
      ordinaryPending.current = true; controlGeneration.current++; setLease(null)
    }
    try {
      const next = requestId === undefined ? await api.command(value) : await api.command(value, requestId)
      observed?.receipt(next)
      if (!live.current || currentApi.current !== api || generation !== epoch.current || sequence !== commandSequence.current) return
      setReceipt(next)
    } catch (error) {
      observed?.uncertain('The submitted Point outcome is unknown. Check xLog; no retry was sent.')
      if (live.current && currentApi.current === api && generation === epoch.current && sequence === commandSequence.current) setActionError(message(error))
      throw error
    } finally {
      if (ordinary && live.current && currentApi.current === api && generation === epoch.current) {
        ordinaryPending.current = false; controlGeneration.current++; setLease(null)
      }
      if (!ordinary && generation === epoch.current && currentApi.current === api) {
        stopsPending.current = Math.max(0, stopsPending.current - 1)
        lastPoll.current = null; pollBarrier.current++
      }
      evidenceChanged()
    }
  }
  /** Existing callers keep their void/throw contract and independent Stop lane. */
  async function command(value: Command): Promise<void> {
    retireEvidence()
    await dispatch(value)
  }
  /** Unwired preparation seam; only an explicit caller can request a Point.
   * No bearing resolver, automatic retry, UI action or native fallback. */
  async function observedPoint(value: Extract<Command, { kind: 'rotor.point' }>, expectedScope: ControlScope,
    requestId = crypto.randomUUID()): Promise<PointEvidenceView> {
    // The eventual bearing/action caller supplies its captured scope. Resolving
    // it from a mutable current ref here could retarget an old UI callback.
    const scope = expectedScope, baseline = lastPoll.current
    point.current?.retire()
    const evidence = createPointEvidence({ requestId, scope, command: value, baseline })
    point.current = evidence
    try { await dispatch(value, requestId, evidence) }
    catch (error) {
      // This transition affects only attempts for which no API call was made.
      evidence.refuse(message(error))
    }
    evidenceChanged()
    return evidence.inspect(currentControlScope())
  }
  const poll = lastPoll.current && lastPoll.current.deadline > now && !ordinaryPending.current && stopsPending.current === 0
    && currentControlScope() ? lastPoll.current : null
  return { view, lease, session, busy: heldView.api === api && busy,
    pollError: heldView.api === api ? pollError : '', actionError: heldView.api === api ? actionError : '',
    receipt: heldView.api === api ? receipt : null, now, connection, command, isCurrentConnection,
    poll, observedPoint, pointEvidence: point.current?.inspect(currentControlScope()) ?? null }
}
