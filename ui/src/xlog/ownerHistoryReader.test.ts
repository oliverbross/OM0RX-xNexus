// SPDX-License-Identifier: GPL-3.0-only
import { describe, expect, it, vi } from 'vitest'
import type { History, HistoryContact, HistoryReadView, RecentContacts } from './api'
import { createOwnerHistoryReader, type OwnerReadBinding } from './ownerHistoryReader'

const binding = (): OwnerReadBinding => ({ sessionGeneration: 1, connectionToken: 'connection-A', owner: {
  paired: true, connected: true, hello: { application: 'OM0RX-xLog', protocol_version: 1,
    server_instance: 'server-A', workspace_id: 'workspace-A', capabilities: ['logbook.read'] },
} })
const contact = (i: number): HistoryContact => ({ contact_id: `contact-${i}`, contact_revision: null,
  callsign: 'K1ABC', band: '20m', mode: 'FT8', started_at_unix_ms: 9000 - i,
  frequency_hz: null, rst_sent: '-10', rst_received: '-12', grid: '', comment: '',
  submode: '', exact_mode: 'FT8', details_complete: false })
function history(total = 1): History {
  return { protocol_version: 1, server_instance: 'server-A', workspace_id: 'workspace-A', callsign: 'K1ABC',
    generation: 'workspace-A', logbook_revision: 4, generated_at_unix_ms: 10000, max_age_ms: 5000,
    contacts: Array.from({ length: Math.min(total, 25) }, (_, i) => contact(i)), matrix: [],
    contacts_limit: 25, contacts_complete: total <= 25, matrix_complete: false,
    history_summary: { schema_version: 1, scope: 'normal_log_base_callsign', complete: true,
      total, worked_before: total > 0, confirmed_count: total > 0 ? 1 : 0,
      confirmation_policy: 'xlog_cluster_paper_or_lotw_v1', confirmation_sources: ['paper_qsl', 'lotw'],
      last_started_at_unix_ms: total > 0 ? 9000 : null, bands: total > 0 ? ['20m'] : [], bands_complete: true,
      modes: total > 0 ? ['FT8'] : [], modes_complete: true,
      requested_band: null, requested_exact_mode: null, requested_band_worked: null, requested_band_exact_mode_worked: null } }
}
function recent(total = 1): RecentContacts {
  return { protocol_version: 1, server_instance: 'server-A', workspace_id: 'workspace-A',
    schema_version: 1, generation: 'workspace-A', logbook_revision: 4, generated_at_unix_ms: 10001,
    max_age_ms: 5000, limit: 5, total, complete: total <= 5, ordering: 'qso_start_desc_contact_id_asc',
    contacts: Array.from({ length: Math.min(total, 5) }, (_, i) => contact(i)) }
}
const wire = <T,>(payload: T, remaining = 500): HistoryReadView<T> => ({ payload, connection_token: 'connection-A', remaining_valid_ms: remaining })
function setup() {
  let time = 100
  const transport = { history: vi.fn(async () => wire(history())), recent: vi.fn(async () => wire(recent())) }
  const reader = createOwnerHistoryReader({ transport, now: () => time })
  return { reader, transport, binding: binding(), advance: (next: number) => { time = next } }
}
const q = { callsign: 'K1ABC' }

describe('original interface owner history reader', () => {
  it('preserves the full owner count and incomplete page without synthesizing QSO facts', async () => {
    const t = setup(), payload = history(40)
    t.transport.history.mockResolvedValue(wire(payload))
    const result = await t.reader.readHistory(t.binding, q)
    expect(result.state).toBe('current')
    if (result.state !== 'current') throw new Error('Expected current result')
    expect(result.payload).toBe(payload)
    expect(result.payload.history_summary!.total).toBe(40)
    expect(result.payload.contacts).toHaveLength(25)
    expect(result.payload.contacts[0].frequency_hz).toBeNull()
    expect(result.payload.contacts[0].details_complete).toBe(false)
    expect(result.payload.contacts[0].contact_revision).toBeNull()
  })
  it('accepts the exact newest five with a larger authoritative total', async () => {
    const t = setup(), payload = recent(12)
    t.transport.recent.mockResolvedValue(wire(payload))
    const result = await t.reader.readRecent(t.binding)
    expect(result.state).toBe('current')
    if (result.state === 'current') expect(result.payload).toBe(payload)
  })
  it('distinguishes an authoritative empty result from an unavailable read', async () => {
    const t = setup()
    t.transport.history.mockResolvedValue(wire(history(0)))
    const empty = await t.reader.readHistory(t.binding, q)
    expect(empty.state).toBe('current')
    t.transport.history.mockRejectedValue(new Error('unsupported'))
    expect((await t.reader.readHistory(t.binding, q)).state).toBe('unavailable')
    expect(t.reader.inspectHistory(t.binding, q).state).toBe('unavailable')
  })
  it('expires at request-start plus remaining time without extending on inspection', async () => {
    const t = setup(), result = await t.reader.readRecent(t.binding)
    expect(result.state === 'current' && result.deadline).toBe(600)
    t.advance(599); expect(t.reader.isCurrent(result, t.binding)).toBe(true)
    t.advance(600); expect(t.reader.inspectRecent(t.binding).state).toBe('unavailable')
    expect(t.reader.isCurrent(result, t.binding)).toBe(false)
    expect(t.transport.recent).toHaveBeenCalledTimes(1)
  })
  it('refuses a response that expired during IPC', async () => {
    const t = setup()
    t.transport.history.mockImplementation(async () => { t.advance(601); return wire(history()) })
    expect((await t.reader.readHistory(t.binding, q)).state).toBe('unavailable')
  })
  it('never renews repeated producer evidence, including after the first display expired', async () => {
    const t = setup()
    await t.reader.readHistory(t.binding, q)
    t.advance(400)
    const repeated = await t.reader.readHistory(t.binding, q)
    expect(repeated.state === 'current' && repeated.deadline).toBe(600)
    t.advance(700)
    expect((await t.reader.readHistory(t.binding, q)).state).toBe('unavailable')
  })
  it('drops other held facts when the shared log revision advances', async () => {
    const t = setup(), before = await t.reader.readHistory(t.binding, q)
    const updated = recent(); updated.logbook_revision = 5
    t.transport.recent.mockResolvedValue(wire(updated))
    expect((await t.reader.readRecent(t.binding)).state).toBe('current')
    expect(t.reader.isCurrent(before, t.binding)).toBe(false)
    expect(t.reader.inspectHistory(t.binding, q).state).toBe('unavailable')
  })
  it.each(['revision', 'timestamp'] as const)('refuses older %s evidence', async field => {
    const t = setup(); await t.reader.readRecent(t.binding)
    const older = history(); older.generated_at_unix_ms = 10002
    if (field === 'revision') older.logbook_revision = 3
    else older.generated_at_unix_ms = 9999
    t.transport.history.mockResolvedValue(wire(older))
    expect((await t.reader.readHistory(t.binding, q)).state).toBe('unavailable')
    expect(t.reader.inspectRecent(t.binding).state).toBe('unavailable')
  })
  it.each(['connection', 'server', 'workspace'] as const)('refuses mismatched %s response identity', async field => {
    const t = setup(), response = wire(history())
    if (field === 'connection') response.connection_token = 'connection-B'
    if (field === 'server') response.payload.server_instance = 'server-B'
    if (field === 'workspace') response.payload.workspace_id = 'workspace-B'
    t.transport.history.mockResolvedValue(response)
    expect((await t.reader.readHistory(t.binding, q)).state).toBe('unavailable')
  })
  it.each(['paired', 'connected', 'capability', 'token'] as const)('makes missing %s authority unavailable without IPC', async field => {
    const t = setup()
    if (field === 'paired') t.binding.owner.paired = false
    if (field === 'connected') t.binding.owner.connected = false
    if (field === 'capability') t.binding.owner.hello!.capabilities = []
    if (field === 'token') t.binding.connectionToken = null
    expect((await t.reader.readRecent(t.binding)).state).toBe('unavailable')
    expect(t.transport.recent).not.toHaveBeenCalled()
  })
  it('canonicalizes exact owner aliases and requires their query echo', async () => {
    const t = setup(), payload = history()
    Object.assign(payload.history_summary!, { requested_band: '20m', requested_exact_mode: 'SSB', requested_band_worked: true, requested_band_exact_mode_worked: false })
    t.transport.history.mockResolvedValue(wire(payload))
    expect((await t.reader.readHistory(t.binding, { callsign: ' k1abc ', band: '20M', exactMode: 'LSB' })).state).toBe('current')
    expect(t.transport.history).toHaveBeenCalledWith('connection-A', 'K1ABC', '20m', 'SSB')
  })
  it('never treats FT8 history as an exact FT4 answer', async () => {
    const t = setup(), payload = history()
    Object.assign(payload.history_summary!, { requested_band: '20m', requested_exact_mode: 'FT8', requested_band_worked: true, requested_band_exact_mode_worked: true })
    t.transport.history.mockResolvedValue(wire(payload))
    expect((await t.reader.readHistory(t.binding, { callsign: 'K1ABC', band: '20m', exactMode: 'FT4' })).state).toBe('unavailable')
  })
  it('cannot land a delayed result after a same-owner reconnect', async () => {
    const t = setup(); let finish!: (v: HistoryReadView<History>) => void
    t.transport.history.mockImplementation(() => new Promise(resolve => { finish = resolve }))
    const pending = t.reader.readHistory(t.binding, q)
    expect(t.reader.inspectHistory(t.binding, q).state).toBe('loading')
    t.binding = { ...t.binding, sessionGeneration: 2 }
    t.reader.synchronize(t.binding); finish(wire(history()))
    expect((await pending).state).toBe('unavailable')
    expect(t.reader.inspectHistory(t.binding, q).state).toBe('unavailable')
  })
  it('drops a superseded callsign reply and other facts its native revision may have retired', async () => {
    const t = setup(); await t.reader.readRecent(t.binding)
    let finish!: (v: HistoryReadView<History>) => void
    t.transport.history.mockImplementation(() => new Promise(resolve => { finish = resolve }))
    const pending = t.reader.readHistory(t.binding, q)
    t.reader.inspectHistory(t.binding, { callsign: 'G1XYZ' })
    finish(wire(history())); expect((await pending).state).toBe('unavailable')
    expect(t.reader.inspectRecent(t.binding).state).toBe('unavailable')
  })
  it('does not queue a second read behind a stalled native request', async () => {
    const t = setup(); let finish!: (v: HistoryReadView<History>) => void
    t.transport.history.mockImplementation(() => new Promise(resolve => { finish = resolve }))
    const pending = t.reader.readHistory(t.binding, q)
    expect((await t.reader.readRecent(t.binding)).state).toBe('unavailable')
    expect(t.transport.recent).not.toHaveBeenCalled()
    finish(wire(history())); await pending
  })
  it('does not revive an old request after the selection changes A to B to A', async () => {
    const t = setup(); let finish!: (v: HistoryReadView<History>) => void
    t.transport.history.mockImplementation(() => new Promise(resolve => { finish = resolve }))
    const pending = t.reader.readHistory(t.binding, q)
    t.reader.inspectHistory(t.binding, { callsign: 'G1XYZ' })
    t.reader.inspectHistory(t.binding, q)
    finish(wire(history()))
    expect((await pending).state).toBe('unavailable')
    expect(t.reader.inspectHistory(t.binding, q).state).toBe('unavailable')
  })
  it.each(['legacy', 'incomplete'] as const)('does not present %s history as complete', async kind => {
    const t = setup(), payload = history()
    if (kind === 'legacy') delete payload.history_summary
    else payload.history_summary!.complete = false
    t.transport.history.mockResolvedValue(wire(payload))
    expect((await t.reader.readHistory(t.binding, q)).state).toBe('unavailable')
  })
  it('revokes both read caches on a native error without retry', async () => {
    const t = setup(); await t.reader.readRecent(t.binding)
    t.transport.history.mockRejectedValue(new Error('401'))
    await t.reader.readHistory(t.binding, q)
    expect(t.reader.inspectRecent(t.binding).state).toBe('unavailable')
    expect(t.transport.history).toHaveBeenCalledTimes(1)
  })
  it.each([99, Number.NaN, Number.POSITIVE_INFINITY])('refuses invalid monotonic clock %s', async value => {
    const t = setup(), result = await t.reader.readRecent(t.binding)
    t.advance(value); expect(t.reader.isCurrent(result, t.binding)).toBe(false)
    expect((await t.reader.readRecent(t.binding)).state).toBe('unavailable')
    expect(t.transport.recent).toHaveBeenCalledTimes(1)
  })
  it('refuses a mode-only query without dispatch', async () => {
    const t = setup()
    expect((await t.reader.readHistory(t.binding, { callsign: 'K1ABC', exactMode: 'FT8' })).state).toBe('unavailable')
    expect(t.transport.history).not.toHaveBeenCalled()
  })
  it.each(['ß1ABC', 'ı1ABC'])('never rewrites Unicode callsign %s into another station', async callsign => {
    const t = setup()
    expect((await t.reader.readHistory(t.binding, { callsign })).state).toBe('unavailable')
    expect(t.transport.history).not.toHaveBeenCalled()
  })
  it.each(['ı', 'ß'])('never rewrites Unicode mode %s into an ASCII query', async exactMode => {
    const t = setup()
    expect((await t.reader.readHistory(t.binding, { callsign: 'K1ABC', band: '20m', exactMode })).state).toBe('unavailable')
    expect(t.transport.history).not.toHaveBeenCalled()
  })
  it('preserves owner-supported hyphens and canonicalizes ASCII lowercase', async () => {
    const t = setup(), payload = history()
    payload.callsign = 'K1ABC-1'
    t.transport.history.mockResolvedValue(wire(payload))
    expect((await t.reader.readHistory(t.binding, { callsign: 'k1abc-1' })).state).toBe('current')
    expect(t.transport.history).toHaveBeenCalledWith('connection-A', 'K1ABC-1', undefined, undefined)
  })
  it('invalidates retained facts synchronously before reconnect or explicit refresh retirement', async () => {
    const t = setup(), result = await t.reader.readRecent(t.binding)
    t.reader.invalidate()
    expect(t.reader.isCurrent(result, t.binding)).toBe(false)
    expect(t.reader.inspectRecent(t.binding).state).toBe('unavailable')
  })
})
