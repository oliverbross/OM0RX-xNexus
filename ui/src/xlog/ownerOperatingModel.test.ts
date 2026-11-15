// SPDX-License-Identifier: GPL-3.0-only
import { describe, expect, it } from 'vitest'
import type { Snapshot, View } from './api'
import { leaseSnapshot, ownerOperatingModel } from './ownerOperatingModel'
import { rxOff } from './rxApi'

function fixture(): View {
  const snapshot: Snapshot = {
    protocol_version: 1, server_instance: 'owner-A', workspace_id: 'workspace-A',
    revision: 8, generated_at_unix_ms: 10000, max_age_ms: 500,
    station: { connected: true, radio_id: 'radio-A', frequency_hz: 10136000, mode: 'DIGU', transmitting: null },
    rotor: { connected: true, rotor_id: 'rotor-A', azimuth_deg: 171, moving: null },
    spots: [{ id: 'spot-A', callsign: 'K1ABC', frequency_hz: 10136000, mode: 'FT8',
      observed_at_unix_ms: 10000, source: 'owner cluster', need: { status: 'unknown', score: 0, reasons: [] } }],
    station_identity: { schema_version: 1, source: 'saved_station_settings', state: 'current',
      mycall: 'OM0RX', mygrid: 'JN88', transmit_authority: false,
      binding: { workspace_generation: 'workspace-A', worker_generation: 7, settings_revision: 3,
        identity_epoch: 'identity-A', identity_revision: 3, operating_module: 'normal_logging' } },
  }
  return { paired: true, connected: true, endpoint: 'http://127.0.0.1:12345',
    hello: { application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'owner-A',
      workspace_id: 'workspace-A', capabilities: ['station.read', 'cluster.read'] }, snapshot }
}
function model(view: View, lease = leaseSnapshot(view, 1000, 10100), now = 1100) {
  return ownerOperatingModel({ view, lease, now, session: 7, receive: rxOff,
    needsLeases: new Map(), decoderMode: 'FT4', rxMarkerHz: 1250 })
}
function expectNoOwnerFacts(value: ReturnType<typeof model>) {
  expect(value.fresh).toBe(false)
  expect(value.station).toBeNull()
  expect(value.rotor).toBeNull()
  expect(value.identity).toBeNull()
  expect(value.spots).toEqual([])
}

describe('original cockpit owner observations', () => {
  it('charges IPC delay against the original observation lifetime', () => {
    const view = fixture()
    const lease = leaseSnapshot(view, 1000, 10100)!
    expect(lease.deadline).toBe(1400)
    expect(model(view, lease, 1399).fresh).toBe(true)
    expectNoOwnerFacts(model(view, lease, 1400))
  })
  it('preserves measured facts and explicit unknowns without granting TX authority', () => {
    const view = fixture(), current = model(view)
    expect(current.station).toBe(view.snapshot!.station)
    expect(current.rotor).toBe(view.snapshot!.rotor)
    expect(current.spots).toBe(view.snapshot!.spots)
    expect(current.station!.transmitting).toBeNull()
    expect(current.rotor!.moving).toBeNull()
    expect(current.spots[0].need.status).toBe('unknown')
    expect(current.identity!.mycall).toBe('OM0RX')
    expect(current.identity!.transmit_authority).toBe(false)
    expect(current.transmit.available).toBe(false)
    expect(current.timing).toEqual({ utcSyncAccuracyUs: null, remoteRfLatencyUs: null })
    expect(current.decoderMode).toBe('FT4')
    expect(current.station!.mode).toBe('DIGU')
  })
  it.each(['paired', 'connected'] as const)('revokes facts immediately when %s becomes false', key => {
    const view = fixture(), lease = leaseSnapshot(view, 1000, 10100)
    const changed = { ...view, [key]: false }
    expect(leaseSnapshot(changed, 1000, 10100)).toBeNull()
    expectNoOwnerFacts(model(changed, lease))
  })
  it.each(['server_instance', 'workspace_id'] as const)('rejects replacement %s while a prior lease remains', key => {
    const view = fixture(), lease = leaseSnapshot(view, 1000, 10100)
    const changed = { ...view, hello: { ...view.hello!, [key]: 'replacement' } }
    expect(leaseSnapshot(changed, 1000, 10100)).toBeNull()
    expectNoOwnerFacts(model(changed, lease))
  })
  it('does not lend a previous accepted object lease to a newly delivered snapshot', () => {
    const view = fixture(), lease = leaseSnapshot(view, 1000, 10100)
    expectNoOwnerFacts(model({ ...view, snapshot: structuredClone(view.snapshot) }, lease))
  })
  it.each([9999, 10500, Number.NaN, Number.POSITIVE_INFINITY])('refuses future, expired or invalid wall-clock evidence (%s)', wallNow => {
    expect(leaseSnapshot(fixture(), 1000, wallNow)).toBeNull()
  })
  it.each([Number.NaN, Number.POSITIVE_INFINITY])('refuses invalid request-start clocks (%s)', start => {
    expect(leaseSnapshot(fixture(), start, 10100)).toBeNull()
  })
  it('keeps incomplete saved identity partial and does not infer a grid', () => {
    const view = fixture()
    view.snapshot!.station_identity = { ...view.snapshot!.station_identity!, state: 'incomplete', mygrid: null }
    expect(model(view).identity).toMatchObject({ state: 'incomplete', mycall: 'OM0RX', mygrid: null })
  })
  it.each(['unavailable', 'other-workspace', 'missing'] as const)('does not use %s saved identity', kind => {
    const view = fixture()
    if (kind === 'missing') delete view.snapshot!.station_identity
    else if (kind === 'unavailable') view.snapshot!.station_identity!.state = 'unavailable'
    else view.snapshot!.station_identity!.binding!.workspace_generation = 'workspace-B'
    expect(model(view).identity).toBeNull()
    expect(model(view).station).toBe(view.snapshot!.station)
  })
  it('does not replace absent owner station measurements with zero or false', () => {
    const view = fixture()
    view.snapshot!.station = { connected: false, radio_id: null, frequency_hz: null, mode: null, transmitting: null }
    view.snapshot!.rotor = { connected: false, rotor_id: null, azimuth_deg: null, moving: null }
    expect(model(view).station).toEqual(view.snapshot!.station)
    expect(model(view).rotor).toEqual(view.snapshot!.rotor)
  })
})
