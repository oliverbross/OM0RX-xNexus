// @vitest-environment jsdom
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { HistoryPanel } from './HistoryPanel'
import { QsoPanel } from './QsoPanel'
import { bridge, type History, type Spot } from './api'

const contact = { contact_id: '10000000-0000-4000-8000-000000000001', callsign: 'OM0RX', band: '20m', mode: 'CW', started_at_unix_ms: 1_700_000_000_123, frequency_hz: 14_025_000, rst_sent: '599', rst_received: '599', grid: 'JN98', comment: 'Original\ncomment', contact_revision: 12 }
const history = (): History => ({ protocol_version: 1, server_instance: 'server-a', workspace_id: 'workspace-a', callsign: 'OM0RX', generated_at_unix_ms: Date.now(), contacts: [contact], matrix: [{ band: '20m', mode: 'CW', worked: true, confirmed: true }] })
const selected: Spot = { id: 'spot-a', callsign: 'OM0RX', frequency_hz: 14_025_000, mode: 'CW', source: 'xLog', observed_at_unix_ms: Date.now(), need: { status: 'worked', score: 0, reasons: [] } }
beforeEach(() => window.localStorage.clear())
describe('owner logbook history', () => {
  it('loads on demand and requires a specific revision to offer editing', async () => {
    const value = history(); value.contacts[0] = { ...contact, contact_revision: null }
    const api = { ...bridge, history: vi.fn(async () => value) }
    render(<HistoryPanel api={api} callsign="OM0RX" workspace="workspace-a" enabled canEdit onEdit={vi.fn()} />)
    expect(api.history).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Load history from xLog' }))
    expect(await screen.findByRole('button', { name: 'Edit QSO' })).toHaveProperty('disabled', true)
    expect(screen.getByText('20m / CW: Confirmed')).toBeTruthy()
  })
  it('discards a history response after the selected station changes', async () => {
    let resolve!: (value: History) => void
    const api = { ...bridge, history: vi.fn(() => new Promise<History>(r => { resolve = r })) }
    const { rerender } = render(<HistoryPanel api={api} callsign="OM0RX" workspace="workspace-a" enabled canEdit onEdit={vi.fn()} />)
    fireEvent.click(screen.getByRole('button', { name: 'Load history from xLog' }))
    rerender(<HistoryPanel api={api} callsign="K1ABC" workspace="workspace-a" enabled canEdit onEdit={vi.fn()} />)
    await act(async () => resolve(history()))
    expect(screen.queryByRole('button', { name: 'Edit QSO' })).toBeNull()
  })
  it('queues a revision-bound correction preserving original UTC precision and fields', async () => {
    const api = { ...bridge, history: vi.fn(async () => history()), outbox: vi.fn(async () => []), enqueue: vi.fn(async () => []) }
    render(<QsoPanel api={api} enabled connected readEnabled workspace="workspace-a" selected={selected} />)
    fireEvent.click(screen.getByRole('button', { name: 'Load history from xLog' }))
    fireEvent.click(await screen.findByRole('button', { name: 'Edit QSO' }))
    for (const label of ['Callsign', 'Band', 'Mode']) expect(screen.getByLabelText(label)).toHaveProperty('readOnly', true)
    expect(screen.getByLabelText('Comment')).toHaveProperty('value', 'Original\ncomment')
    fireEvent.change(screen.getByLabelText('RST received'), { target: { value: '579' } })
    fireEvent.click(screen.getByRole('button', { name: 'Queue QSO correction' }))
    const { contact_revision: _, ...qso } = contact
    await waitFor(() => expect(api.enqueue).toHaveBeenCalledWith({ kind: 'logbook.update', qso: { ...qso, rst_received: '579' }, expected_contact_revision: 12 }, expect.any(String), 'workspace-a'))
  })
  it('does not replace an unfinished draft whose callsign is still blank', async () => {
    const api = { ...bridge, history: vi.fn(async () => history()), outbox: vi.fn(async () => []), enqueue: vi.fn(async () => []) }
    render(<QsoPanel api={api} enabled connected readEnabled workspace="workspace-a" selected={selected} />)
    fireEvent.change(screen.getByLabelText('Comment'), { target: { value: 'Unfinished contact notes' } })
    fireEvent.click(screen.getByRole('button', { name: 'Load history from xLog' }))
    const edit = await screen.findByRole('button', { name: 'Edit QSO' })
    expect(edit).toHaveProperty('disabled', true)
    fireEvent.click(edit)
    expect(screen.getByLabelText('Comment')).toHaveProperty('value', 'Unfinished contact notes')
    fireEvent.click(screen.getByRole('button', { name: 'Clear draft' }))
    expect(screen.getByRole('button', { name: 'Edit QSO' })).toHaveProperty('disabled', false)
  })
})
