import { describe, expect, it } from 'vitest'
import type { Receipt, Snapshot } from './api'
import type { ControlPoll, ControlScope } from './controlObservation'
import { createPointEvidence, type PointIntent } from './pointEvidence'
const scope: ControlScope = { connectionToken: 'local-token', sessionGeneration: 1, serverInstance: 'owner', workspaceId: 'workspace' }
const requestId = '11111111-1111-4111-8111-111111111111'
const snapshot = (revision = 1): Snapshot => ({ protocol_version: 1, server_instance: 'owner', workspace_id: 'workspace', revision,
  generated_at_unix_ms: 10_000 + revision, max_age_ms: 3000,
  station: { connected: false, radio_id: null, frequency_hz: null, mode: null, transmitting: null },
  rotor: { connected: true, rotor_id: 'rotor-route', azimuth_deg: 90, moving: null }, spots: [] })
const poll = (revision = 1, started = 10): ControlPoll => ({ scope, origin: 'poll', requestStartedMono: started, deadline: 1000, snapshot: snapshot(revision) })
const intent = (): PointIntent => ({ requestId, scope, command: { kind: 'rotor.point', rotor_id: 'rotor-route', azimuth_deg: 90 }, baseline: poll() })
const receipt = (status: Receipt['status'] = 'accepted'): Receipt => ({ request_id: requestId, server_instance: 'owner', workspace_id: 'workspace', status, message: null })
describe('historical Point dispatch evidence', () => {
  it('never marks a missing-baseline attempt pending or submitted', () => {
    const e = createPointEvidence({ ...intent(), baseline: null }, () => 100)
    expect(e.submit(scope)).toBe(false); expect(e.inspect(scope).submitted).toBe(false)
    expect(e.inspect(scope).settlement.kind).toBe('locally_refused')
  })
  it('preserves a correctly bound historical receipt after Stop retires observation', () => {
    let now = 100; const e = createPointEvidence(intent(), () => now)
    expect(e.submit(scope)).toBe(true); e.retire(); now = 110; e.receipt(receipt())
    expect(e.inspect(scope)).toMatchObject({ submitted: true, retired: true, settlement: { kind: 'receipt', receipt: { status: 'accepted' } }, observation: { kind: 'unknown' } })
  })
  it('preserves uncertainty after source retirement without turning it into local refusal', () => {
    const e = createPointEvidence(intent(), () => 100); e.submit(scope)
    e.inspect({ ...scope, connectionToken: 'new-token' }); e.uncertain('reply lost'); e.refuse('local')
    expect(e.inspect(scope).settlement).toEqual({ kind: 'uncertain', reason: 'reply lost' })
    expect(e.inspect(scope).retired).toBe(true)
  })
  it.each(['accepted', 'rejected', 'uncertain'] as const)('keeps %s distinct from a later exact target match', status => {
    let now = 100; const e = createPointEvidence(intent(), () => now); e.submit(scope)
    now = 120; e.receipt(receipt(status)); now = 150; e.observe(poll(2, 130))
    expect(e.inspect(scope).settlement).toEqual({ kind: 'receipt', receipt: receipt(status) })
    expect(e.inspect(scope).observation).toMatchObject({ kind: 'snapshot', matchesTarget: true, moving: null })
    now = 1000; expect(e.inspect(scope).observation).toEqual({ kind: 'unknown' })
    expect(e.inspect(scope).settlement.kind).toBe('receipt')
  })
  it('rejects a mismatched receipt as uncertain and never replaces it with a later answer', () => {
    const e = createPointEvidence(intent(), () => 100); e.submit(scope)
    e.receipt({ ...receipt(), request_id: '22222222-2222-4222-8222-222222222222' }); e.receipt(receipt())
    expect(e.inspect(scope).settlement.kind).toBe('uncertain')
  })
  it('retires on target replacement while dispatch is pending, then preserves its eventual receipt', () => {
    const e = createPointEvidence(intent(), () => 100); e.submit(scope)
    const replacement = poll(2, 90); replacement.snapshot.rotor.rotor_id = 'replacement'
    e.observe(replacement); e.receipt(receipt('applied'))
    expect(e.inspect(scope)).toMatchObject({ retired: true, settlement: { kind: 'receipt' }, observation: { kind: 'unknown' } })
  })
  it('copies immutable intent/receipt bytes and refuses dispatch after original baseline expires', () => {
    let now = 100; const draft = intent(), e = createPointEvidence(draft, () => now)
    draft.command.azimuth_deg = 180; expect(e.inspect(scope).intent.command.azimuth_deg).toBe(90)
    now = 1000; expect(e.submit(scope)).toBe(false); expect(e.inspect(scope).settlement.kind).toBe('locally_refused')
  })
  it.each(['rollback', 'throw', 'nonfinite'] as const)('latches a pre-dispatch %s clock fault as local refusal', failure => {
    let faulty = false
    const e = createPointEvidence(intent(), () => {
      if (faulty && failure === 'throw') throw Error('clock unavailable')
      if (faulty && failure === 'nonfinite') return NaN
      return faulty ? 99 : 100
    })
    faulty = true
    expect(e.submit(scope)).toBe(false)
    faulty = false
    expect(e.submit(scope)).toBe(false)
    expect(e.inspect(scope)).toMatchObject({ submitted: false, retired: true, settlement: { kind: 'locally_refused' } })
  })
  it('retains the exact successful receipt when the observation clock throws after submission', () => {
    let faulty = false
    const e = createPointEvidence(intent(), () => { if (faulty) throw Error('clock unavailable'); return 100 })
    expect(e.submit(scope)).toBe(true)
    faulty = true
    expect(() => e.receipt(receipt('applied'))).not.toThrow()
    expect(e.inspect(scope)).toMatchObject({ submitted: true, retired: true,
      settlement: { kind: 'receipt', receipt: receipt('applied') }, observation: { kind: 'unknown' } })
    e.uncertain('metadata failure')
    expect(e.inspect(scope).settlement).toEqual({ kind: 'receipt', receipt: receipt('applied') })
  })
})
