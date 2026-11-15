import { describe, expect, it } from 'vitest'
import type { History, RecentContacts } from './api'
import { ownerRecallRead, ownerRecentRead } from './ownerHistoryPresentation'
import type { OwnerReadState } from './ownerHistoryReader'

function payload(): History {
  return { protocol_version: 1, server_instance: 'owner', workspace_id: 'workspace', callsign: 'K1ABC',
    generated_at_unix_ms: 10000, contacts: [
      { contact_id: 'complete', contact_revision: 5, callsign: 'K1ABC', band: '20m', mode: 'MFSK',
        exact_mode: 'FT4', submode: 'FT4', details_complete: true, started_at_unix_ms: 9000,
        frequency_hz: null, rst_sent: '-12', rst_received: '-13', grid: '', comment: 'Real comment' },
      { contact_id: 'partial', contact_revision: null, callsign: 'K1ABC', band: '30m', mode: 'FT8',
        exact_mode: 'FT8', submode: '', details_complete: false, started_at_unix_ms: 8000,
        frequency_hz: null, rst_sent: '-08', rst_received: '-11', grid: '', comment: 'Partial comment' },
    ], matrix: [], history_summary: { schema_version: 1, scope: 'normal_log_base_callsign', complete: true,
      total: 40, worked_before: true, confirmed_count: 3, confirmation_policy: 'xlog_cluster_paper_or_lotw_v1',
      confirmation_sources: ['paper_qsl', 'lotw'], last_started_at_unix_ms: 9000,
      bands: ['20m', '30m'], modes: ['FT4', 'FT8'], bands_complete: true, modes_complete: true,
      requested_band: '20m', requested_exact_mode: 'FT4', requested_band_worked: true, requested_band_exact_mode_worked: false } }
}
const current = <T,>(payload: T): OwnerReadState<T> => ({ state: 'current', payload, deadline: 500, contextKey: 'scope', queryKey: 'query' })
describe('owner history projection into original panes', () => {
  it('uses full summary counts and distinguishes band from exact-mode matches', () => {
    const read = current(payload()), band = ownerRecallRead(read, false), exact = ownerRecallRead(read, true)
    expect(band.state === 'current' && band.history.dupeThisBand).toBe(true)
    expect(exact.state === 'current' && exact.history.dupeThisBand).toBe(false)
    if (exact.state !== 'current') throw new Error('Expected summary')
    expect(exact.history.count).toBe(40)
    expect(exact.history.confirmedCount).toBe(3)
    expect(exact.confirmationTitle).toContain('paper QSL or LoTW')
    expect(exact.history.qsos.map(q => q.canOpen)).toEqual([true, false])
    expect(exact.history.qsos[0].mode).toBe('FT4')
    expect(exact.history.qsos[0]).not.toHaveProperty('freqMhz')
    expect(exact.history.qsos[0]).not.toHaveProperty('confirmed')
    expect(exact.history.qsos[0]).not.toHaveProperty('notes')
  })
  it('preserves unknown dupe state and refuses a missing full summary', () => {
    const data = payload(); data.history_summary!.requested_band_exact_mode_worked = null
    const read = ownerRecallRead(current(data), true)
    expect(read.state === 'current' && read.history.dupeThisBand).toBeNull()
    delete data.history_summary
    expect(ownerRecallRead(current(data), false).state).toBe('unavailable')
  })
  it('preserves unavailable/loading states without empty-log replacement', () => {
    const missing = { state: 'unavailable' as const, reason: 'Expired', contextKey: null, queryKey: '' }
    expect(ownerRecallRead(missing, false)).toEqual({ state: 'unavailable', message: 'Expired' })
    expect(ownerRecentRead(missing)).toEqual({ state: 'unavailable', message: 'Expired' })
    expect(ownerRecallRead({ state: 'loading', contextKey: 'scope', queryKey: '' }, false).state).toBe('loading')
  })
  it('maps recent rows without inventing frequency or editable revisions', () => {
    const data: RecentContacts = { protocol_version: 1, server_instance: 'owner', workspace_id: 'workspace',
      generation: 'workspace', logbook_revision: 5, generated_at_unix_ms: 10000, max_age_ms: 5000,
      schema_version: 1, total: 40, limit: 5, complete: false, ordering: 'qso_start_desc_contact_id_asc', contacts: payload().contacts }
    const read = ownerRecentRead(current(data))
    if (read.state !== 'current') throw new Error('Expected rows')
    expect(read.total).toBe(40)
    expect(read.rows.map(q => q.canOpen)).toEqual([true, false])
    expect(read.rows[0]).toMatchObject({ id: 'complete', whenUnix: 9, mode: 'FT4' })
  })
})
