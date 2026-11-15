// SPDX-License-Identifier: GPL-3.0-only
import type { Command, Receipt } from './api'
import { createRotorObserver, type ControlPoll, type ControlScope, type RotorObservation } from './controlObservation'

export interface PointIntent {
  requestId: string
  scope: ControlScope | null
  command: Extract<Command, { kind: 'rotor.point' }>
  baseline: ControlPoll | null
}
export type PointSettlement =
  | { kind: 'locally_refused'; reason: string }
  | { kind: 'pending' }
  | { kind: 'receipt'; receipt: Receipt }
  | { kind: 'uncertain'; reason: string }
export interface PointEvidenceView {
  intent: PointIntent
  submitted: boolean
  retired: boolean
  settlement: PointSettlement
  observation: RotorObservation
}
const same = (a: ControlScope, b: ControlScope) => a.connectionToken === b.connectionToken && a.sessionGeneration === b.sessionGeneration
  && a.serverInstance === b.serverInstance && a.workspaceId === b.workspaceId
const statuses = new Set<Receipt['status']>(['accepted', 'applied', 'observed', 'rejected', 'uncertain'])

/** One immutable attempt. Historical dispatch settlement survives retirement of
 * current-target observation authority. No IO, retry, bearing or motion inference. */
export function createPointEvidence(input: PointIntent, now: () => number = () => performance.now()) {
  const intent = structuredClone(input)
  let submitted = false, retired = false
  let settlement: PointSettlement = { kind: 'locally_refused', reason: 'Not dispatched.' }
  let observer: ReturnType<typeof createRotorObserver> | null = null
  let lastClock: number | null = null, clockFailed = false
  // Clock/observation metadata must never throw away a receipt from an already
  // attempted command. A fault is latched; it cannot regain admission later.
  const clock = () => {
    if (clockFailed) return NaN
    try {
      const stamp = now()
      if (!Number.isFinite(stamp) || stamp < 0 || (lastClock !== null && stamp < lastClock)) throw Error('Invalid observation clock')
      lastClock = stamp; return stamp
    } catch {
      clockFailed = true; retired = true; observer?.invalidate(); return NaN
    }
  }
  try {
    if (!intent.scope || !intent.baseline) throw Error('A current xLog poll baseline is required.')
    observer = createRotorObserver({ ...intent, scope: intent.scope, baseline: intent.baseline }, clock)
  } catch (error) { settlement = { kind: 'locally_refused', reason: String(error) }; retired = true }
  const retire = () => { retired = true; observer?.invalidate() }
  const uncertain = (reason: string) => {
    if (!submitted || settlement.kind !== 'pending') return
    settlement = { kind: 'uncertain', reason }
    if (intent.scope) observer?.uncertain(intent.scope)
  }
  return {
    retire,
    refuse(reason: string) { if (!submitted) { settlement = { kind: 'locally_refused', reason }; retire() } },
    submit(scope: ControlScope | null) {
      if (!observer || retired || submitted || !scope || !intent.scope || !same(scope, intent.scope)) return false
      const stamp = clock()
      if (!Number.isFinite(stamp) || stamp < 0 || !intent.baseline || stamp >= intent.baseline.deadline) {
        settlement = { kind: 'locally_refused', reason: 'The Point baseline expired before dispatch.' }; retire(); return false
      }
      submitted = true; settlement = { kind: 'pending' }; return true
    },
    receipt(receipt: Receipt) {
      if (!submitted || settlement.kind !== 'pending') return
      if (!intent.scope || receipt.request_id !== intent.requestId || receipt.server_instance !== intent.scope.serverInstance
        || receipt.workspace_id !== intent.scope.workspaceId || !statuses.has(receipt.status)) {
        uncertain('The returned receipt did not match this request. No retry was sent.'); return
      }
      settlement = { kind: 'receipt', receipt: structuredClone(receipt) }
      // The helper may already be retired. The historical receipt above remains.
      observer?.receipt(intent.scope, receipt)
    },
    uncertain,
    observe(poll: ControlPoll) {
      if (!intent.scope || !same(intent.scope, poll.scope) || poll.snapshot.rotor.rotor_id !== intent.command.rotor_id) retire()
      if (!retired) observer?.observe(poll)
    },
    inspect(scope: ControlScope | null): PointEvidenceView {
      if (!scope || !intent.scope || !same(scope, intent.scope)) retire()
      const current = !retired && intent.scope && observer ? observer.inspect(intent.scope) : null
      if (current?.retired) retire()
      const observation = !retired && current ? current.observation : { kind: 'unknown' } as const
      return structuredClone({ intent, submitted, retired, settlement, observation })
    },
  }
}
