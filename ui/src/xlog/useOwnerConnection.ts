// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import { bridge, disconnected, type Bridge, type Command, type Receipt, type View, type StationIdentityBinding } from './api'
import { bearingIdentityKey } from './ownerBearingReader'
import { leaseSnapshot, type SnapshotLease } from './ownerOperatingModel'
import type { ControlPoll, ControlScope } from './controlObservation'
import { createPointEvidence, type PointEvidenceView } from './pointEvidence'

const message = (error: unknown) => error instanceof Error ? error.message : String(error)

export interface OwnerConnectionScope { session: number; connectionToken: string; serverInstance: string; workspaceId: string }

/** Native production discovers xLog locally. Injected adapters remain explicit
 * unless auto is requested; neither path starts a Nexus station engine. */
export function useOwnerConnection(api: Bridge = bridge, auto = api === bridge) {
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
  const nativeConnected = useRef(false)
  const connectionPending = useRef(false)
  const autoPaused = useRef(false)
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
  const isCurrentReadConnection = (scope: OwnerConnectionScope, requiredCapabilities: readonly string[] = []) =>
    heldView.api === api && isCurrentConnection(scope) && acceptedView.current.connected
      && requiredCapabilities.every(capability => acceptedView.current.hello?.capabilities.includes(capability))
  const isCurrentBearingIdentity = (scope: OwnerConnectionScope, binding: StationIdentityBinding, grid: string) => {
    const current = acceptedView.current.snapshot?.station_identity, currentLease = admittedLease.current
    const at = performance.now()
    return isCurrentReadConnection(scope, ['bearing.read']) && currentLease !== null && Number.isFinite(at) && at >= 0
      && currentLease.deadline > at && currentLease.snapshot === acceptedView.current.snapshot
      && current?.transmit_authority === false && current.source === 'saved_station_settings'
      && (current.state === 'current' || current.state === 'incomplete') && current.mygrid === grid
      && bearingIdentityKey(current.binding) !== null && bearingIdentityKey(current.binding) === bearingIdentityKey(binding)
  }
  const accept = (next: View, started: number, control = controlGeneration.current, pendingAtStart = ordinaryPending.current) => {
    const previous = acceptedView.current
    if (previous.connection_token && next.connection_token !== previous.connection_token) {
      retireEvidence(); ordinaryPending.current = false; controlGeneration.current++
      setSession(++epoch.current)
    }
    nativeConnected.current = next.connected
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
    nativeConnected.current = false; autoPaused.current = false; connectionPending.current = true
    retireEvidence(); stopsPending.current = 0
    ordinaryPending.current = false; controlGeneration.current++
    setSession(generation); setView(disconnected); setLease(null)
    setPollError(''); setActionError(''); setReceipt(null); setBusy(false)
    const started = performance.now()
    void api.status().then(next => {
      if (live.current && currentApi.current === api && generation === epoch.current) accept(next, started)
    }, error => { if (live.current && currentApi.current === api && generation === epoch.current) setPollError(message(error)) })
      .finally(() => { if (live.current && currentApi.current === api && generation === epoch.current) connectionPending.current = false })
    return () => { live.current = false; ++epoch.current; retireEvidence() }
  }, [api])
  useEffect(() => {
    if (!auto) return
    let active = true
    let timer: ReturnType<typeof setTimeout>
    const discover = async () => {
      if (!active || currentApi.current !== api) return
      if (!autoPaused.current && !nativeConnected.current && !connectionPending.current) await connection('connect', true)
      if (active) timer = setTimeout(discover, 1500)
    }
    timer = setTimeout(discover, 0)
    return () => { active = false; clearTimeout(timer) }
  }, [api, auto])
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
        // Retire passive-read admission synchronously, before React commits.
        // Keep paired/token identity for the existing independent Stop lane.
        acceptedView.current = { ...acceptedView.current, connected: false }
        setLease(null); setPollError(message(error))
        lastPoll.current = null
        if (!auto) {
          setView(old => ({ ...old, connected: false, snapshot: null }))
          return
        }
        // Local status distinguishes a retired native connection from a slow
        // owner poll. Keep the original facts/identity while their own leases
        // expire, so receiver and FT hooks are not torn down on a timeout.
        try {
          const status = await api.status()
          if (!active || currentApi.current !== api || generation !== epoch.current) return
          if (!status.connected || status.connection_token !== acceptedView.current.connection_token) {
            accept(status, performance.now())
          }
        } catch { /* Continue the bounded poll; a failed status is not revocation. */ }
      }
      if (active && generation === epoch.current) timer = setTimeout(poll, 500)
    }
    timer = setTimeout(poll, 500)
    return () => { active = false; clearTimeout(timer) }
  }, [api, auto, view.connected, session])

  async function connection(kind: 'import' | 'connect' | 'disconnect' | 'forget', automatic = false) {
    if (currentApi.current !== api || (automatic && connectionPending.current)) return
    if (!automatic) autoPaused.current = kind === 'disconnect' || kind === 'forget'
    connectionPending.current = true
    const generation = ++epoch.current
    acceptedView.current = disconnected
    nativeConnected.current = false
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
      if (live.current && currentApi.current === api && generation === epoch.current) {
        if (automatic) setPollError(message(error)); else setActionError(message(error))
      }
    } finally {
      if (live.current && currentApi.current === api && generation === epoch.current) {
        connectionPending.current = false; setBusy(false)
      }
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
  /** Only an explicit caller can request a Point. No resolver or retry. */
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
    receipt: heldView.api === api ? receipt : null, now, connection, command, isCurrentConnection, isCurrentReadConnection,
    poll, observedPoint, isCurrentBearingIdentity,
    // Current observation only: historical dispatch settlement and Stop identity survive.
    retirePointObservation: () => { point.current?.retire() },
    pointEvidence: point.current?.inspect(currentControlScope()) ?? null }
}
