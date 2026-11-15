import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Snapshot, Spot, View } from './api'
import { ownerSpotsSource, type OwnerSpotsInput } from './ownerSpots'
import { externalSpotsCurrent, selectExternalSpot } from '../components/spotsExternal'

const spot = (over: Partial<Spot> = {}): Spot => ({
  id: 'spot-1', callsign: 'K1ABC', frequency_hz: 14_074_123, mode: 'FT8', source: 'xLog cluster',
  observed_at_unix_ms: 9_000, country: null, band: null,
  need: { status: 'needed', score: 71, reasons: ['New Call'] }, ...over,
})
function input(spots = [spot()]): OwnerSpotsInput {
  const snapshot: Snapshot = { protocol_version: 1, server_instance: 'server', workspace_id: 'workspace',
    revision: 4, generated_at_unix_ms: 10_000, max_age_ms: 3000,
    station: { connected: false, radio_id: null, frequency_hz: null, mode: null, transmitting: null },
    rotor: { connected: false, rotor_id: null, azimuth_deg: null, moving: null }, spots }
  const view: View = { paired: true, connected: true, connection_token: 'local-connection', endpoint: null,
    hello: { application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'server', workspace_id: 'workspace', capabilities: ['cluster.read'] }, snapshot }
  return { view, lease: { snapshot, deadline: 1000 }, session: 1 }
}
afterEach(() => vi.restoreAllMocks())

describe('owner Spots presentation (synthetic wire fixtures)', () => {
  it('keeps exact Hz and immutable IDs without inventing unavailable native facts', () => {
    const current = input([spot(), spot({ id: 'spot-2', frequency_hz: 14_074_456, source: 'different feed' })])
    const source = ownerSpotsSource(current, () => current, vi.fn())
    expect(source.rows.map(row => [row.id, row.frequencyHz])).toEqual([['spot-1', 14_074_123], ['spot-2', 14_074_456]])
    expect(source.rows[0]).toMatchObject({ country: null, band: null, assessment: { status: 'needed', score: 71 } })
    for (const key of ['licensed', 'spotter', 'tags', 'zone', 'grid', 'heardMe', 'latitude', 'longitude']) expect(source.rows[0]).not.toHaveProperty(key)
  })
  it('unknown with old score/award-looking statuses exposes no score or tags', () => {
    const current = input([spot({ call_status: 'New Call', dxcc_status: 'ATNO',
      need: { status: 'unknown', score: 9999, reasons: ['Waiting for reclassification'] } })])
    const row = ownerSpotsSource(current, () => current, vi.fn()).rows[0]
    expect(row.assessment).toEqual({ status: 'unknown', explanation: 'Waiting for reclassification' })
    expect(row).not.toHaveProperty('dxcc_status')
  })
  it('does not reinterpret worked as confirmed or modify the owner score', () => {
    const current = input([spot({ need: { status: 'worked', score: -12, reasons: ['Worked'] } })])
    expect(ownerSpotsSource(current, () => current, vi.fn()).rows[0].assessment).toEqual({ status: 'worked', score: -12, reasons: ['Worked'] })
  })
  it('zero observed timestamp is unknown, without changing snapshot authority', () => {
    const current = input([spot({ observed_at_unix_ms: 0 })])
    const source = ownerSpotsSource(current, () => current, vi.fn())
    expect(source.rows[0].observedAtUnixMs).toBeNull()
    expect(externalSpotsCurrent(source, 100)).toBe(true)
  })
  it('future observed time is unknown even when the snapshot lease is current', () => {
    const current = input([spot({ observed_at_unix_ms: 10_001 })])
    const source = ownerSpotsSource(current, () => current, vi.fn())
    expect(source.rows[0].observedAtUnixMs).toBeNull()
    expect(externalSpotsCurrent(source, 100)).toBe(true)
  })
  it('selection preserves the exact source row and fails at the original deadline', () => {
    vi.spyOn(performance, 'now').mockReturnValue(100)
    const current = input(), select = vi.fn(), source = ownerSpotsSource(current, () => current, select)
    expect(selectExternalSpot(source, source.rows[0], 100)).toBe(true)
    expect(select).toHaveBeenCalledExactlyOnceWith(source.rows[0])
    expect(selectExternalSpot(source, { ...source.rows[0] }, 100)).toBe(false)
    expect(selectExternalSpot(source, source.rows[0], 1000)).toBe(false)
    expect(select).toHaveBeenCalledTimes(1)
  })
  it.each(['session', 'token', 'server', 'workspace', 'snapshot', 'lease', 'disconnected', 'capability'] as const)('retires old row actions after %s replacement', kind => {
    vi.spyOn(performance, 'now').mockReturnValue(100)
    let current = input()
    const select = vi.fn(), source = ownerSpotsSource(current, () => current, select), row = source.rows[0]
    if (kind === 'session') current = { ...current, session: 2 }
    if (kind === 'token') current = { ...current, view: { ...current.view, connection_token: 'new' } }
    if (kind === 'server' || kind === 'workspace') current = { ...current, view: { ...current.view,
      hello: { ...current.view.hello!, [kind === 'server' ? 'server_instance' : 'workspace_id']: 'new' } } }
    if (kind === 'snapshot') current = { ...current, view: { ...current.view, snapshot: structuredClone(current.view.snapshot) } }
    if (kind === 'lease') current = { ...current, lease: null }
    if (kind === 'disconnected') current = { ...current, view: { ...current.view, connected: false } }
    if (kind === 'capability') current = { ...current, view: { ...current.view, hello: { ...current.view.hello!, capabilities: [] } } }
    expect(externalSpotsCurrent(source, 100)).toBe(false)
    expect(selectExternalSpot(source, row, 100)).toBe(false)
    source.select(row) // direct callback also rechecks; UI is not the final fence.
    expect(select).not.toHaveBeenCalled()
  })
  it('absence is not current empty, and reading cannot extend a lease', () => {
    const current = input([]), source = ownerSpotsSource(current, () => current, vi.fn())
    expect(source.rows).toEqual([])
    expect(externalSpotsCurrent(source, 999)).toBe(true)
    expect(externalSpotsCurrent(source, 1000)).toBe(false)
    expect(externalSpotsCurrent(source, 1001)).toBe(false)
    const absent = { ...current, view: { ...current.view, connection_token: null } }
    expect(ownerSpotsSource(absent, () => absent, vi.fn()).state).toBe('unavailable')
  })
  it('a populated snapshot without cluster.read cannot grant rows or selection', () => {
    vi.spyOn(performance, 'now').mockReturnValue(100)
    const original = input(), select = vi.fn()
    const missing = { ...original, view: { ...original.view, hello: { ...original.view.hello!, capabilities: [] } } }
    const source = ownerSpotsSource(missing, () => missing, select)
    expect(source.state).toBe('unavailable')
    expect(source.rows).toEqual([])
    expect(externalSpotsCurrent(source, 100)).toBe(false)
    expect(select).not.toHaveBeenCalled()
  })
  it.each([-1, Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY])('invalid monotonic clock %s cannot admit facts or selection', now => {
    vi.spyOn(performance, 'now').mockReturnValue(now)
    const current = input(), select = vi.fn(), source = ownerSpotsSource(current, () => current, select)
    expect(externalSpotsCurrent(source, now)).toBe(false)
    expect(selectExternalSpot(source, source.rows[0], now)).toBe(false)
    source.select(source.rows[0])
    expect(select).not.toHaveBeenCalled()
  })
})
