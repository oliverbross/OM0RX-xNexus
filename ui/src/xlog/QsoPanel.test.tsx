// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { QsoPanel } from './QsoPanel'
import { bridge, type Bridge, type OutboxEntry } from './api'

const qso = { contact_id: '10000000-0000-4000-8000-000000000001', callsign: 'OM0RX', band: '20m', mode: 'SSB', started_at_unix_ms: 1_700_000_000_000, frequency_hz: 14_200_000, rst_sent: '59', rst_received: '59', grid: '', comment: '' }
const entry = (state: OutboxEntry['state'] = 'Pending'): OutboxEntry => ({ queued: { version: 1, request_id: '20000000-0000-4000-8000-000000000001', workspace_id: 'workspace-a', mutation: { kind: 'logbook.create', qso } }, state })
function api(entries: OutboxEntry[] = []): Bridge {
  return { ...bridge, outbox: vi.fn(async () => entries), enqueue: vi.fn(async () => [entry()]), deliver: vi.fn(async () => ({ request_id: entry().queued.request_id, server_instance: 'server-a', workspace_id: 'workspace-a', status: 'applied' as const, message: null })) }
}
beforeEach(() => window.localStorage.clear())
describe('durable QSO queue', () => {
  it('queues a draft without dispatch and only clears it after durable acknowledgement', async () => {
    const native = api()
    render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    fireEvent.change(screen.getByLabelText('Callsign'), { target: { value: 'om0rx' } })
    fireEvent.click(screen.getByRole('button', { name: 'Queue QSO' }))
    await screen.findByRole('button', { name: 'Submit to xLog' })
    expect(native.enqueue).toHaveBeenCalledWith(expect.objectContaining({ kind: 'logbook.create', qso: expect.objectContaining({ callsign: 'OM0RX' }) }), expect.any(String), 'workspace-a')
    expect(native.deliver).not.toHaveBeenCalled()
    expect(screen.getByLabelText('Callsign')).toHaveProperty('value', '')
  })
  it('preserves draft on storage failure', async () => {
    const native = api(); vi.mocked(native.enqueue).mockRejectedValue(new Error('Cannot persist queue'))
    render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    fireEvent.change(screen.getByLabelText('Callsign'), { target: { value: 'OM0RX' } })
    fireEvent.click(screen.getByRole('button', { name: 'Queue QSO' }))
    await screen.findByText('Cannot persist queue')
    expect(screen.getByLabelText('Callsign')).toHaveProperty('value', 'OM0RX')
    expect(native.deliver).not.toHaveBeenCalled()
  })
  it('starts a new QSO when its first callsign is entered after an idle period', async () => {
    const clock = vi.spyOn(Date, 'now').mockReturnValue(1_700_000_000_000)
    try {
      const native = api()
      render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
      clock.mockReturnValue(1_700_001_800_000)
      fireEvent.change(screen.getByLabelText('Callsign'), { target: { value: 'OM0RX' } })
      clock.mockReturnValue(1_700_002_100_000)
      fireEvent.change(screen.getByLabelText('RST sent'), { target: { value: '59' } })
      fireEvent.click(screen.getByRole('button', { name: 'Queue QSO' }))
      await waitFor(() => expect(native.enqueue).toHaveBeenCalledWith(expect.objectContaining({ qso: expect.objectContaining({ started_at_unix_ms: 1_700_001_800_000 }) }), expect.any(String), 'workspace-a'))
    } finally { clock.mockRestore() }
  })
  it('preserves a manually chosen UTC start even before the callsign is entered', async () => {
    const native = api()
    render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    fireEvent.change(screen.getByLabelText('Started UTC'), { target: { value: '2023-11-14T21:00' } })
    fireEvent.change(screen.getByLabelText('Callsign'), { target: { value: 'OM0RX' } })
    fireEvent.click(screen.getByRole('button', { name: 'Queue QSO' }))
    await waitFor(() => expect(native.enqueue).toHaveBeenCalledWith(expect.objectContaining({ qso: expect.objectContaining({ started_at_unix_ms: Date.parse('2023-11-14T21:00Z') }) }), expect.any(String), 'workspace-a'))
  })
  it('never sends another workspace entry or auto-retries an uncertain result', async () => {
    const native = api([entry('NeedsReceipt')])
    const { rerender } = render(<QsoPanel api={native} enabled connected workspace="workspace-b" />)
    expect(await screen.findByRole('button', { name: 'Check receipt' })).toHaveProperty('disabled', true)
    expect(native.deliver).not.toHaveBeenCalled()
    rerender(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    fireEvent.click(screen.getByRole('button', { name: 'Check receipt' }))
    await waitFor(() => expect(native.deliver).toHaveBeenCalledWith(entry().queued.request_id, 'reconcile'))
    expect(native.deliver).toHaveBeenCalledTimes(1)
  })
  it('requires an explicit separate retry preserving the operation ID', async () => {
    const native = api([entry('NeedsReceipt')])
    render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    fireEvent.click(await screen.findByRole('button', { name: 'Retry only if receipt missing' }))
    await waitFor(() => expect(native.deliver).toHaveBeenCalledWith(entry().queued.request_id, 'retry_missing'))
  })
})
