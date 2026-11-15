// SPDX-License-Identifier: GPL-3.0-only
import { describe, expect, it } from 'vitest'
import type { Receipt, Snapshot } from './api'
import { createRotorObserver, type ControlPoll, type ControlScope, type RotorRequest } from './controlObservation'

const scope: ControlScope = { connectionToken: 'native-session-1', sessionGeneration: 3, serverInstance: 'owner-1', workspaceId: 'log-1' }
const id = '22a34122-3333-4444-8888-222222222222'
function snapshot(revision = 10, generated = 1000, azimuth: number | null = 40): Snapshot {
  return { protocol_version: 1, server_instance: 'owner-1', workspace_id: 'log-1', revision, generated_at_unix_ms: generated,
    max_age_ms: 3000, station: { connected: false, radio_id: null, frequency_hz: null, mode: null, transmitting: null },
    rotor: { connected: true, rotor_id: 'route-worker-1', azimuth_deg: azimuth, moving: null }, spots: [] }
}
function setup() {
  let time = 100
  const request: RotorRequest = { requestId: id, scope: { ...scope }, command: { kind: 'rotor.point', rotor_id: 'route-worker-1', azimuth_deg: 90 },
    baseline: { scope: { ...scope }, origin: 'poll', requestStartedMono: 0, deadline: 2500, snapshot: snapshot() } }
  const observer = createRotorObserver(request, () => time)
  const receipt = (status: Receipt['status'] = 'accepted'): Receipt => ({ request_id: id, server_instance: scope.serverInstance, workspace_id: scope.workspaceId, status, message: 'owner receipt' })
  const poll = (overrides: Partial<ControlPoll> = {}): ControlPoll => ({ origin: 'poll', scope: { ...scope }, requestStartedMono: 201, deadline: 2000, snapshot: snapshot(11, 2000, 90), ...overrides })
  return { observer, request, receipt, poll, tick: (value: number) => { time = value }, view: () => observer.inspect(scope),
    complete: (status: Receipt['status'] = 'accepted') => { time = 200; observer.receipt(scope, receipt(status)); time = 300 } }
}

describe('one explicit owner rotor observation (no command transport)', () => {
  it.each(['accepted', 'applied', 'observed', 'rejected', 'uncertain'] as const)('keeps %s receipt independent of matching later state', status => {
    const h = setup(); h.complete(status)
    expect(h.view().observation).toEqual({ kind: 'unknown' })
    expect(h.observer.observe(h.poll())).toBe(true)
    expect(h.view().dispatch).toEqual({ kind: 'receipt', receipt: h.receipt(status) })
    expect(h.view().observation).toMatchObject({ kind: 'snapshot', matchesTarget: true, moving: null })
  })
  it('requires completion and a poll begun strictly afterward, not a pending or status read', () => {
    const h = setup(); h.tick(300)
    expect(h.observer.observe(h.poll())).toBe(false)
    expect(h.view().dispatch).toEqual({ kind: 'pending' })
    h.tick(400); h.observer.receipt(scope, h.receipt()); h.tick(500)
    for (const start of [100, 399, 400]) expect(h.observer.observe(h.poll({ requestStartedMono: start }))).toBe(false)
    expect(h.observer.observe(h.poll({ origin: 'status' as 'poll', requestStartedMono: 401 }))).toBe(false)
    expect(h.observer.observe(h.poll({ requestStartedMono: 401 }))).toBe(true)
  })
  it('does not change uncertain transport outcome into success', () => {
    const h = setup(); h.tick(200); h.observer.uncertain(scope); h.tick(300)
    h.observer.observe(h.poll())
    expect(h.view().dispatch.kind).toBe('uncertain')
    expect(h.observer.receipt(scope, h.receipt('observed'))).toBe(false)
    expect(h.view().observation).toMatchObject({ matchesTarget: true })
  })
  it('does not mutate original or returned request/receipt objects', () => {
    const h = setup(); h.request.command.azimuth_deg = 180; h.request.scope.connectionToken = 'other'
    h.tick(200); const r = h.receipt(); h.observer.receipt(scope, r); r.status = 'observed'
    const view = h.view(); view.request.command.azimuth_deg = 270
    expect(h.view().request.command.azimuth_deg).toBe(90)
    expect(h.view().dispatch).toMatchObject({ receipt: { status: 'accepted' } })
  })
  it.each(['request_id', 'server_instance', 'workspace_id'] as const)('rejects foreign receipt %s', key => {
    const h = setup(); expect(h.observer.receipt(scope, { ...h.receipt(), [key]: 'foreign' })).toBe(false)
    expect(h.view().dispatch.kind).toBe('pending')
  })
  it.each(['connectionToken', 'sessionGeneration', 'serverInstance', 'workspaceId'] as const)('retires irreversibly on current %s replacement', key => {
    const h = setup(); h.complete(); h.observer.observe(h.poll())
    const changed = { ...scope, [key]: key === 'sessionGeneration' ? 4 : 'different' }
    expect(h.observer.inspect(changed).retired).toBe(true)
    expect(h.observer.observe(h.poll())).toBe(false)
    expect(h.view().observation.kind).toBe('unknown')
  })
  it('retires replacement rotor route, even if original route returns', () => {
    const h = setup(); h.complete(); const p = h.poll(); p.snapshot.rotor.rotor_id = 'replacement'
    h.observer.observe(p); expect(h.observer.observe(h.poll())).toBe(false)
    expect(h.view().retired).toBe(true)
  })
  it('keeps null azimuth unknown and never invents stationary motion from null', () => {
    const h = setup(); h.complete(); h.observer.observe(h.poll({ snapshot: snapshot(11, 2000, null) }))
    expect(h.view().observation.kind).toBe('unknown')
    h.observer.observe(h.poll({ snapshot: snapshot(12, 2001, 90) }))
    expect(h.view().observation).toMatchObject({ matchesTarget: true, moving: null })
  })
  it.each([true, false, null])('preserves moving=%s separately from matching azimuth', moving => {
    const h = setup(); h.complete(); const p = h.poll(); p.snapshot.rotor.moving = moving
    h.observer.observe(p); expect(h.view().observation).toMatchObject({ matchesTarget: true, moving })
  })
  it('uses exact azimuth without invented tolerance or accepting wire 360', () => {
    const h = setup(); h.complete(); h.observer.observe(h.poll({ snapshot: snapshot(11, 2000, 90.00001) }))
    expect(h.view().observation).toMatchObject({ matchesTarget: false })
    h.observer.observe(h.poll({ snapshot: snapshot(12, 2001, 360) }))
    expect(h.view().observation.kind).toBe('unknown')
  })
  it.each([[10, 2000], [11, 1000], [9, 900]])('requires both baseline fences %s/%s', (revision, generated) => {
    const h = setup(); h.complete(); expect(h.observer.observe(h.poll({ snapshot: snapshot(revision, generated, 90) }))).toBe(false)
  })
  it('does not replace newer state with an out-of-order poll', () => {
    const h = setup(); h.complete(); h.observer.observe(h.poll({ snapshot: snapshot(12, 2100, 90) }))
    h.observer.observe(h.poll({ snapshot: snapshot(11, 2000, 40) }))
    expect(h.view().observation).toMatchObject({ revision: 12, matchesTarget: true })
  })
  it('expires on render and repeated evidence cannot extend deadline or revive it', () => {
    const h = setup(); h.complete(); h.observer.observe(h.poll({ deadline: 500 }))
    h.tick(500); expect(h.view().observation.kind).toBe('unknown')
    expect(h.observer.observe(h.poll({ requestStartedMono: 499, deadline: 2000 }))).toBe(false)
    expect(h.view().observation.kind).toBe('unknown')
    expect(h.observer.observe(h.poll({ requestStartedMono: 499, snapshot: snapshot(12, 2001, 90) }))).toBe(true)
  })
  it('rejects a changed payload for the same consumed evidence', () => {
    const h = setup(); h.complete(); h.observer.observe(h.poll())
    h.observer.observe(h.poll({ snapshot: snapshot(11, 2000, 80) })); expect(h.view().retired).toBe(true)
  })
  it('disconnection or an expired display lease removes current observation', () => {
    const h = setup(); h.complete(); h.observer.observe(h.poll())
    const p = h.poll({ snapshot: snapshot(12, 2100, 90) }); p.snapshot.rotor.connected = false
    h.observer.observe(p); expect(h.view().observation.kind).toBe('unknown')
    expect(h.observer.observe(h.poll({ deadline: 300 }))).toBe(false)
  })
  it('Stop/supersession invalidation is immediate and cannot be restored by late receipt/poll', () => {
    const h = setup(); h.observer.invalidate(); h.complete()
    expect(h.observer.observe(h.poll())).toBe(false)
    expect(h.view()).toMatchObject({ retired: true, dispatch: { kind: 'pending' }, observation: { kind: 'unknown' } })
  })
  it('fails closed on monotonic clock regression', () => {
    const h = setup(); h.complete(); h.observer.observe(h.poll()); h.tick(299)
    expect(h.view().retired).toBe(true)
  })
  it('requires a live route-bound baseline and rejects unsupported command targets', () => {
    const h = setup()
    expect(() => createRotorObserver({ ...h.request, baseline: { ...h.request.baseline, deadline: 100 } }, () => 100)).toThrow()
    expect(() => createRotorObserver({ ...h.request, command: { ...h.request.command, azimuth_deg: 360 } }, () => 100)).toThrow()
  })
})
