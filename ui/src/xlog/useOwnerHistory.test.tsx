// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { disconnected, type History, type HistoryReadView, type RecentContacts, type View } from './api'
import { useOwnerHistory } from './useOwnerHistory'

// Synthetic owner API boundary, not a real SQLite or RF acceptance fixture.
const view: View = { ...disconnected, paired: true, connected: true, connection_token: 'connection-A',
  hello: { application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'server-A',
    workspace_id: 'workspace-A', capabilities: ['logbook.read'] } }
function payload(callsign: string, generated: number): History {
  return { protocol_version: 1, server_instance: 'server-A', workspace_id: 'workspace-A', callsign,
    generation: 'workspace-A', logbook_revision: 1, generated_at_unix_ms: generated, max_age_ms: 5000,
    contacts: [], matrix: [], contacts_limit: 25, contacts_complete: true, matrix_complete: true,
    history_summary: { schema_version: 1, scope: 'normal_log_base_callsign', complete: true,
      total: 0, worked_before: false, confirmed_count: 0, confirmation_policy: 'xlog_cluster_paper_or_lotw_v1',
      confirmation_sources: ['paper_qsl', 'lotw'], last_started_at_unix_ms: null, bands: [], bands_complete: true,
      modes: [], modes_complete: true, requested_band: null, requested_exact_mode: null,
      requested_band_worked: null, requested_band_exact_mode_worked: null } }
}
function wire<T>(value: T): HistoryReadView<T> { return { payload: value, connection_token: 'connection-A', remaining_valid_ms: 5000 } }
function setup() {
  vi.useFakeTimers(); vi.spyOn(document, 'hidden', 'get').mockReturnValue(false); let now = 100, generated = 10000
  vi.spyOn(performance, 'now').mockImplementation(() => now)
  const transport = {
    history: vi.fn(async (_token: string, call: string) => wire(payload(call, ++generated))),
    recent: vi.fn(async () => wire<RecentContacts>({ protocol_version: 1, server_instance: 'server-A', workspace_id: 'workspace-A',
      schema_version: 1, generation: 'workspace-A', logbook_revision: 1, generated_at_unix_ms: ++generated,
      max_age_ms: 5000, contacts: [], limit: 5, total: 0, complete: true, ordering: 'qso_start_desc_contact_id_asc' })),
  }
  return { transport, advance: async (ms: number) => { now += ms; await act(async () => { await vi.advanceTimersByTimeAsync(ms) }) } }
}
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done }); return { promise, resolve } }
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.useRealTimers() })
describe('original Recall and Recent live reads', () => {
  it('opens exact compact row IDs using the queried Recall call and current lease, never a positional fallback', async () => {
    const s = setup(), id = '00000000-0000-4000-8000-000000000999'
    const data = payload('K1AAA', 10001)
    data.contacts = [{ contact_id: id, contact_revision: null, callsign: 'K1AAA/P', band: '20m', mode: 'MFSK',
      exact_mode: 'FT8', submode: 'FT8', details_complete: false, started_at_unix_ms: 9000,
      frequency_hz: null, rst_sent: '-10', rst_received: '-11', grid: '', comment: '' }]
    data.history_summary = { ...data.history_summary!, total: 1, worked_before: true, last_started_at_unix_ms: 9000,
      bands: ['20m'], modes: ['FT8'] }
    s.transport.history.mockResolvedValueOnce(wire(data))
    const capable = { ...view, hello: { ...view.hello!, capabilities: ['logbook.read', 'logbook.contact.read'] } }
    const { result } = renderHook(() => useOwnerHistory(capable, 1, { callsign: 'K1AAA' }, s.transport))
    await act(async () => {})
    const open = result.current.canOpenContact
    expect(open('K1AAA', id)).toBe(true)
    expect(open('K1AAA/P', id)).toBe(false)
    expect(open('K1AAA', null)).toBe(false)
    expect(open('K1AAA', '00000000-0000-4000-8000-000000000998')).toBe(false)
    act(() => { vi.spyOn(document, 'hidden', 'get').mockReturnValue(true); document.dispatchEvent(new Event('visibilitychange')) })
    await s.advance(4999); expect(open('K1AAA', id)).toBe(true)
    await s.advance(1); expect(open('K1AAA', id)).toBe(false)
  })
  it('rejects duplicate exact IDs even when the compact rows have complete business metadata', async () => {
    const s = setup(), id = '00000000-0000-4000-8000-000000000999'
    const data = payload('K1AAA', 10001)
    const row = { contact_id: id, contact_revision: 1, callsign: 'K1AAA', band: '20m', mode: 'FT8',
      details_complete: true, started_at_unix_ms: 9000, frequency_hz: null, rst_sent: '', rst_received: '', grid: '', comment: '' }
    data.contacts = [row, { ...row }]
    data.history_summary = { ...data.history_summary!, total: 2, worked_before: true }
    s.transport.history.mockResolvedValueOnce(wire(data))
    const capable = { ...view, hello: { ...view.hello!, capabilities: ['logbook.read', 'logbook.contact.read'] } }
    const { result } = renderHook(() => useOwnerHistory(capable, 1, { callsign: 'K1AAA' }, s.transport))
    await act(async () => {})
    expect(result.current.canOpenContact('K1AAA', id)).toBe(false)
  })
  it('coalesces a pending A → B → A selection into one fresh A request after retirement', async () => {
    const s = setup(), first = deferred<HistoryReadView<History>>()
    s.transport.history.mockReturnValueOnce(first.promise)
    const { result, rerender } = renderHook(({ call }) => useOwnerHistory(view, 1, { callsign: call }, s.transport), { initialProps: { call: 'K1AAA' } })
    rerender({ call: 'K1BBB' }); rerender({ call: 'K1AAA' })
    expect(s.transport.history).toHaveBeenCalledTimes(1)
    expect(result.current.recall.state).not.toBe('current')
    await act(async () => { first.resolve(wire(payload('K1AAA', 9999))) })
    expect(s.transport.history.mock.calls.map(call => call[1])).toEqual(['K1AAA', 'K1AAA'])
    expect(s.transport.recent).toHaveBeenCalledTimes(1)
    expect(result.current.recall.state).toBe('current')
    if (result.current.recall.state === 'current') expect(result.current.recall.callsign).toBe('K1AAA')
  })
  it('refreshes successful reads nonoverlapping and keeps Recall plus Recent current beyond one lease', async () => {
    const s = setup()
    const { result } = renderHook(() => useOwnerHistory(view, 1, { callsign: 'K1AAA' }, s.transport))
    await act(async () => {})
    expect(result.current.recall.state).toBe('current')
    for (let i = 0; i < 3; i++) await s.advance(2000)
    expect(s.transport.history).toHaveBeenCalledTimes(4)
    expect(s.transport.recent).toHaveBeenCalledTimes(4)
    expect(result.current.recall.state).toBe('current')
    expect(result.current.recent.state).toBe('current')
  })
  it('pauses after failure and restarts only on explicit refresh or a new desired query', async () => {
    const s = setup()
    const { result } = renderHook(() => useOwnerHistory(view, 1, { callsign: 'K1AAA' }, s.transport))
    await act(async () => {})
    s.transport.history.mockRejectedValueOnce(new Error('Owner unavailable'))
    await s.advance(2000)
    expect(result.current.recall.state).toBe('unavailable')
    await s.advance(10000)
    expect(s.transport.history).toHaveBeenCalledTimes(2)
    await act(async () => {
      vi.spyOn(document, 'hidden', 'get').mockReturnValue(true)
      document.dispatchEvent(new Event('visibilitychange'))
      vi.spyOn(document, 'hidden', 'get').mockReturnValue(false)
      document.dispatchEvent(new Event('visibilitychange'))
    })
    expect(s.transport.history).toHaveBeenCalledTimes(2)
    await act(async () => result.current.refresh())
    expect(s.transport.history).toHaveBeenCalledTimes(3)
    expect(result.current.recall.state).toBe('current')
  })
  it('does not overlap a slow refresh, and its old Recent lease expires while waiting', async () => {
    const s = setup(), slow = deferred<HistoryReadView<History>>()
    const { result } = renderHook(() => useOwnerHistory(view, 1, { callsign: 'K1AAA' }, s.transport))
    await act(async () => {})
    s.transport.history.mockReturnValueOnce(slow.promise)
    await s.advance(2000)
    await s.advance(4000)
    expect(s.transport.history).toHaveBeenCalledTimes(2)
    expect(s.transport.recent).toHaveBeenCalledTimes(1)
    expect(result.current.recent.state).toBe('unavailable')
    expect(result.current.recall.state).not.toBe('current')
  })
  it('never requests a local log or owner transport while disconnected', async () => {
    const s = setup()
    const { result } = renderHook(() => useOwnerHistory(disconnected, 1, { callsign: 'K1AAA' }, s.transport))
    await act(async () => {})
    await s.advance(6000)
    expect(s.transport.history).not.toHaveBeenCalled()
    expect(s.transport.recent).not.toHaveBeenCalled()
    expect(result.current.recall.state).toBe('unavailable')
  })
  it('pauses while hidden, expires displayed facts and resumes only the latest selection', async () => {
    const s = setup()
    const { result, rerender } = renderHook(({ call }) => useOwnerHistory(view, 1, { callsign: call }, s.transport), { initialProps: { call: 'K1AAA' } })
    await act(async () => {})
    act(() => {
      vi.spyOn(document, 'hidden', 'get').mockReturnValue(true)
      document.dispatchEvent(new Event('visibilitychange'))
    })
    rerender({ call: 'K1BBB' }); rerender({ call: 'K1CCC' })
    await s.advance(6000)
    expect(s.transport.history.mock.calls.map(call => call[1])).toEqual(['K1AAA'])
    expect(result.current.recall.state).not.toBe('current')
    expect(result.current.recent.state).toBe('unavailable')
    await act(async () => {
      vi.spyOn(document, 'hidden', 'get').mockReturnValue(false)
      document.dispatchEvent(new Event('visibilitychange'))
    })
    expect(s.transport.history.mock.calls.map(call => call[1])).toEqual(['K1AAA', 'K1CCC'])
    expect(result.current.recall.state).toBe('current')
  })
  it('does not dispatch Recent after a pending History completes in a hidden document', async () => {
    const s = setup(), first = deferred<HistoryReadView<History>>()
    s.transport.history.mockReturnValueOnce(first.promise)
    const { result } = renderHook(() => useOwnerHistory(view, 1, { callsign: 'K1AAA' }, s.transport))
    act(() => {
      vi.spyOn(document, 'hidden', 'get').mockReturnValue(true)
      document.dispatchEvent(new Event('visibilitychange'))
    })
    await act(async () => { first.resolve(wire(payload('K1AAA', 9999))) })
    expect(s.transport.history).toHaveBeenCalledTimes(1)
    expect(s.transport.recent).not.toHaveBeenCalled()
    await act(async () => {
      vi.spyOn(document, 'hidden', 'get').mockReturnValue(false)
      document.dispatchEvent(new Event('visibilitychange'))
    })
    expect(s.transport.history).toHaveBeenCalledTimes(2)
    expect(s.transport.recent).toHaveBeenCalledTimes(1)
    expect(result.current.recent.state).toBe('current')
  })
})
