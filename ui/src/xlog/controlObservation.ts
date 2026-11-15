// SPDX-License-Identifier: GPL-3.0-only
import type { Command, Receipt, Snapshot } from './api'

export interface ControlScope {
  connectionToken: string
  sessionGeneration: number
  serverInstance: string
  workspaceId: string
}
/** Deadline belongs to the existing validated snapshot display lease. */
export interface ControlPoll {
  scope: ControlScope
  origin: 'poll'
  requestStartedMono: number
  deadline: number
  snapshot: Snapshot
}
export interface RotorRequest {
  requestId: string
  scope: ControlScope
  command: Extract<Command, { kind: 'rotor.point' }>
  baseline: ControlPoll
}
export type DispatchEvidence =
  | { kind: 'pending' }
  | { kind: 'receipt'; receipt: Receipt }
  | { kind: 'uncertain'; reason: string }
export type RotorObservation =
  | { kind: 'unknown' }
  | { kind: 'snapshot'; matchesTarget: boolean; azimuthDeg: number; moving: boolean | null;
      revision: number; generatedAtUnixMs: number; deadline: number }
export interface ControlEvidence {
  request: RotorRequest
  retired: boolean
  dispatch: DispatchEvidence
  observation: RotorObservation
}

const safe = (n: number) => Number.isSafeInteger(n) && n >= 0
const text = (s: string) => typeof s === 'string' && s.length > 0 && s.length <= 128 && !/[\u0000-\u001f\u007f]/.test(s)
const uuid = (s: string) => /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(s)
const angle = (n: number | null): n is number => typeof n === 'number' && Number.isFinite(n) && n >= 0 && n < 360
const scopeValid = (s: ControlScope) => text(s.connectionToken) && safe(s.sessionGeneration) && text(s.serverInstance) && text(s.workspaceId)
const same = (a: ControlScope, b: ControlScope) => a.connectionToken === b.connectionToken && a.sessionGeneration === b.sessionGeneration && a.serverInstance === b.serverInstance && a.workspaceId === b.workspaceId
const statuses = new Set<Receipt['status']>(['accepted', 'applied', 'observed', 'rejected', 'uncertain'])

/**
 * One explicit rotor.point request; pure local evidence only. Never sends,
 * retries, reconciles a receipt, or handles Stop. Call invalidate synchronously
 * on Stop intent, source retirement or a superseding action. Call inspect with
 * the current scope on renders/timers; a previously returned object is not a lease.
 *
 * Snapshot equality is NOT a causal acknowledgement or proof of a new device
 * sample: today's owner wire omits field sample timestamps. The receipt is
 * immutable and independent of this later, leased state observation.
 */
export function createRotorObserver(input: RotorRequest, now: () => number = () => performance.now()) {
  const request = structuredClone(input)
  let lastNow = now()
  const admittedAt = lastNow
  let retired = false
  let completedAt: number | null = null
  let dispatch: DispatchEvidence = { kind: 'pending' }
  let observation: RotorObservation = { kind: 'unknown' }
  let revision = request.baseline.snapshot.revision
  let generated = request.baseline.snapshot.generated_at_unix_ms
  // Retain consumed evidence after expiry; a repeated snapshot cannot renew it.
  let consumed: { revision: number; generated: number; deadline: number; azimuth: number | null; moving: boolean | null; connected: boolean } | null = null
  const invalidate = () => { retired = true; observation = { kind: 'unknown' } }
  const clock = () => {
    const value = now()
    if (!Number.isFinite(value) || value < lastNow) invalidate()
    else lastNow = value
    return value
  }
  const fresh = (poll: ControlPoll, at: number) => {
    const s = poll.snapshot
    return poll.origin === 'poll' && scopeValid(poll.scope) && same(request.scope, poll.scope)
      && s.protocol_version === 1 && s.server_instance === request.scope.serverInstance && s.workspace_id === request.scope.workspaceId
      && safe(s.revision) && safe(s.generated_at_unix_ms) && s.generated_at_unix_ms > 0
      && safe(s.max_age_ms) && s.max_age_ms > 0 && s.max_age_ms <= 3000
      && Number.isFinite(poll.requestStartedMono) && poll.requestStartedMono >= 0 && poll.requestStartedMono <= at
      && Number.isFinite(poll.deadline) && poll.deadline > at && poll.deadline <= poll.requestStartedMono + s.max_age_ms
  }
  if (!Number.isFinite(admittedAt) || admittedAt < 0 || !scopeValid(request.scope) || !uuid(request.requestId)
      || request.command.kind !== 'rotor.point' || !text(request.command.rotor_id) || !angle(request.command.azimuth_deg)
      || !fresh(request.baseline, admittedAt) || !request.baseline.snapshot.rotor.connected
      || request.baseline.snapshot.rotor.rotor_id !== request.command.rotor_id) {
    throw new Error('A current, identity-bound rotor baseline is required.')
  }
  const current = (scope: ControlScope) => {
    const at = clock()
    if (!scopeValid(scope) || !same(scope, request.scope)) invalidate()
    if (observation.kind === 'snapshot' && at >= observation.deadline) observation = { kind: 'unknown' }
    return at
  }
  const finish = (scope: ControlScope, result: DispatchEvidence) => {
    const at = current(scope)
    if (retired || dispatch.kind !== 'pending') return false
    completedAt = at
    dispatch = structuredClone(result)
    return true
  }
  return {
    invalidate,
    receipt(scope: ControlScope, receipt: Receipt) {
      if (receipt.request_id !== request.requestId || receipt.server_instance !== request.scope.serverInstance
          || receipt.workspace_id !== request.scope.workspaceId || !statuses.has(receipt.status)) return false
      return finish(scope, { kind: 'receipt', receipt })
    },
    uncertain(scope: ControlScope) {
      return finish(scope, { kind: 'uncertain', reason: 'The command outcome is unknown. Check xLog; no retry was sent.' })
    },
    observe(poll: ControlPoll) {
      const at = current(poll.scope)
      if (retired || completedAt === null || poll.origin !== 'poll' || poll.requestStartedMono <= completedAt) return false
      const s = poll.snapshot
      // A new owner/route cannot become this operation's target even if it later returns.
      if (s.server_instance !== request.scope.serverInstance || s.workspace_id !== request.scope.workspaceId
          || s.rotor.rotor_id !== request.command.rotor_id) { invalidate(); return false }
      if (!fresh(poll, at)) { observation = { kind: 'unknown' }; return false }
      // Ignore out-of-order reads without replacing newer evidence.
      if (s.revision < revision || s.generated_at_unix_ms < generated) return false
      if (s.revision <= request.baseline.snapshot.revision || s.generated_at_unix_ms <= request.baseline.snapshot.generated_at_unix_ms) return false
      const moving = typeof s.rotor.moving === 'boolean' ? s.rotor.moving : null
      let deadline = poll.deadline
      if (consumed && (s.revision === consumed.revision || s.generated_at_unix_ms === consumed.generated)) {
        if (s.revision !== consumed.revision || s.generated_at_unix_ms !== consumed.generated
            || s.rotor.azimuth_deg !== consumed.azimuth || moving !== consumed.moving || s.rotor.connected !== consumed.connected) { invalidate(); return false }
        deadline = Math.min(deadline, consumed.deadline)
      }
      revision = s.revision
      generated = s.generated_at_unix_ms
      consumed = { revision, generated, deadline, azimuth: s.rotor.azimuth_deg, moving, connected: s.rotor.connected }
      observation = s.rotor.connected && angle(s.rotor.azimuth_deg) && deadline > at
        ? { kind: 'snapshot', matchesTarget: s.rotor.azimuth_deg === request.command.azimuth_deg,
            azimuthDeg: s.rotor.azimuth_deg, moving, revision, generatedAtUnixMs: generated, deadline }
        : { kind: 'unknown' }
      return observation.kind === 'snapshot'
    },
    inspect(scope: ControlScope): ControlEvidence {
      current(scope)
      return structuredClone({ request, retired, dispatch, observation })
    },
  }
}
