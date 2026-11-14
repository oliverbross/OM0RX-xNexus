// @vitest-environment jsdom
import { act, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { App } from './App'
import { bridge, disconnected, type Bridge, type View } from './api'

function connected(capabilities = ['snapshot.read', 'station.tune', 'rotor.point', 'rotor.stop']): View {
  return { paired: true, connected: true, endpoint: 'http://127.0.0.1:49123',
    hello: { application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'fixture-server', workspace_id: 'fixture-workspace', capabilities },
    snapshot: { protocol_version: 1, server_instance: 'fixture-server', workspace_id: 'fixture-workspace', revision: 1, generated_at_unix_ms: Date.now(), max_age_ms: 10_000,
      station: { connected: true, radio_id: 'fixture-radio', frequency_hz: 14_074_000, mode: 'USB', transmitting: false },
      rotor: { connected: true, rotor_id: 'fixture-rotor', azimuth_deg: 123, moving: false },
      spots: [{ id: 'needed', callsign: 'W1AW', frequency_hz: 14_025_000, mode: 'CW', observed_at_unix_ms: Date.now(), source: 'synthetic-cluster', need: { status: 'needed', score: 100, reasons: ['New band from xLog'] } },
        { id: 'unknown', callsign: 'K1ABC', frequency_hz: 14_030_000, mode: 'CW', observed_at_unix_ms: Date.now(), source: 'synthetic-cluster', need: { status: 'unknown', score: 999, reasons: [] } }],
    } }
}
function api(view: View = disconnected): Bridge {
  const receipt = { request_id: 'synthetic-request', server_instance: 'fixture-server', workspace_id: 'fixture-workspace', status: 'observed' as const, message: null }
  return { status: vi.fn(async () => view), importPairing: vi.fn(async () => connected()), connect: vi.fn(async () => connected()), poll: vi.fn(async () => connected()), disconnect: vi.fn(async forget => ({ ...disconnected, paired: !forget })), command: vi.fn(async () => receipt), outbox: vi.fn(async () => []), enqueue: vi.fn(async () => []), deliver: vi.fn(async () => receipt), history: vi.fn(async callsign => ({ protocol_version: 1, server_instance: 'fixture-server', workspace_id: 'fixture-workspace', generated_at_unix_ms: Date.now(), callsign, contacts: [], matrix: [] })) }
}
afterEach(() => { vi.useRealTimers(); delete (window as unknown as { __TAURI__?: unknown }).__TAURI__ })

describe('xLog-owned desktop', () => {
  it('starts disconnected and requires native pairing without opening a station', async () => {
    const native = api()
    render(<App api={native} />)
    await screen.findByText('Your station, one source of truth')
    expect(native.connect).not.toHaveBeenCalled()
    expect(native.poll).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Stop rotator' })).toHaveProperty('disabled', true)
    fireEvent.click(screen.getByRole('button', { name: 'Open xLog pairing file' }))
    await screen.findByText('Connected to xLog')
    expect(native.importPairing).toHaveBeenCalledOnce()
  })
  it('renders xLog evidence, excludes unknown needs and uses the observed radio for tune', async () => {
    const native = api(connected())
    render(<App api={native} />)
    fireEvent.click(await screen.findByRole('button', { name: 'Inspect W1AW' }))
    expect(screen.queryByRole('button', { name: 'Inspect K1ABC' })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Tune through xLog' }))
    await waitFor(() => expect(native.command).toHaveBeenCalledWith({ kind: 'station.tune', radio_id: 'fixture-radio', frequency_hz: 14_025_000, mode: 'CW' }))
    fireEvent.click(screen.getByRole('button', { name: 'All spots' }))
    expect(screen.getByRole('button', { name: 'Inspect K1ABC' })).toBeTruthy()
  })
  it('does not infer command capability from a connected radio or rotor', async () => {
    render(<App api={api(connected(['snapshot.read']))} />)
    fireEvent.click(await screen.findByRole('button', { name: 'Inspect W1AW' }))
    expect(screen.getByRole('button', { name: 'Tune through xLog' })).toHaveProperty('disabled', true)
    expect(screen.getByRole('button', { name: 'Point through xLog' })).toHaveProperty('disabled', true)
    expect(screen.getByRole('button', { name: 'Stop rotator' })).toHaveProperty('disabled', true)
  })
  it.each([true, null])('holds tuning when transmit state is %s and labels unknown rotor motion', async transmitting => {
    const value = connected(); value.snapshot!.station.transmitting = transmitting; value.snapshot!.rotor.moving = null
    const native = api(value)
    render(<App api={native} />)
    fireEvent.click(await screen.findByRole('button', { name: 'Inspect W1AW' }))
    expect(screen.getByRole('button', { name: 'Tune through xLog' })).toHaveProperty('disabled', true)
    expect(screen.getByText('Motion unknown')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Tune through xLog' }))
    expect(native.command).not.toHaveBeenCalled()
  })
  it('invalidates stale observations while keeping the separate stop route visible', async () => {
    vi.useFakeTimers()
    const v = connected(); v.snapshot!.max_age_ms = 500
    const native = api(v)
    render(<App api={native} />)
    await act(async () => { await Promise.resolve() })
    fireEvent.click(screen.getByRole('button', { name: 'Inspect W1AW' }))
    await act(async () => { vi.advanceTimersByTime(750) })
    expect(screen.getByRole('button', { name: 'Tune through xLog' })).toHaveProperty('disabled', true)
    expect(screen.getByRole('button', { name: 'Stop rotator' })).toHaveProperty('disabled', false)
    fireEvent.click(screen.getByRole('button', { name: 'Stop rotator' }))
    expect(native.command).toHaveBeenCalledWith({ kind: 'rotor.stop', rotor_id: 'fixture-rotor' })
  })
  it('clears observations after poll failure and never automatically reconnects', async () => {
    vi.useFakeTimers()
    const native = api(connected()); vi.mocked(native.poll).mockRejectedValue(new Error('Fixture link lost'))
    render(<App api={native} />)
    await act(async () => { await Promise.resolve() })
    await act(async () => { vi.advanceTimersByTime(1100) })
    expect(screen.getByRole('alert').textContent).toContain('Fixture link lost')
    expect(screen.queryByRole('button', { name: 'Inspect W1AW' })).toBeNull()
    expect(native.connect).not.toHaveBeenCalled()
    expect(screen.getByRole('button', { name: 'Stop rotator' })).toHaveProperty('disabled', false)
  })
  it('does not restore a disconnected session from a late poll', async () => {
    vi.useFakeTimers()
    let resolve!: (view: View) => void
    const native = api(connected()); vi.mocked(native.poll).mockReturnValue(new Promise(r => { resolve = r }))
    render(<App api={native} />)
    await act(async () => { await Promise.resolve() })
    await act(async () => { vi.advanceTimersByTime(1000) })
    fireEvent.click(screen.getByRole('button', { name: 'Disconnect' }))
    await act(async () => { await Promise.resolve(); resolve(connected()) })
    expect(screen.getByText('xLog disconnected')).toBeTruthy()
    expect(screen.queryByRole('button', { name: 'Inspect W1AW' })).toBeNull()
  })
  it('forgets pairing and disables retained stop authority', async () => {
    const native = api(connected())
    render(<App api={native} />)
    fireEvent.click(await screen.findByRole('button', { name: 'Forget pairing' }))
    await screen.findByText('Your station, one source of truth')
    expect(native.disconnect).toHaveBeenCalledWith(true)
    expect(screen.getByRole('button', { name: 'Stop rotator' })).toHaveProperty('disabled', true)
  })
  it('drops the prior station stop route while pairing a replacement', async () => {
    const native = api(connected()); vi.mocked(native.importPairing).mockReturnValue(new Promise(() => {}))
    render(<App api={native} />)
    await screen.findByText('Connected to xLog')
    fireEvent.click(screen.getByRole('button', { name: 'Open xLog pairing file' }))
    expect(screen.getByRole('button', { name: 'Stop rotator' })).toHaveProperty('disabled', true)
    expect(screen.queryByRole('button', { name: 'Inspect W1AW' })).toBeNull()
  })
  it('does not show an old Stop result under a replacement pairing', async () => {
    let resolve!: (receipt: Awaited<ReturnType<Bridge['command']>>) => void
    const native = api(connected()); vi.mocked(native.command).mockReturnValue(new Promise(r => { resolve = r }))
    render(<App api={native} />)
    await screen.findByText('Connected to xLog')
    fireEvent.click(screen.getByRole('button', { name: 'Stop rotator' }))
    fireEvent.click(screen.getByRole('button', { name: 'Open xLog pairing file' }))
    await act(async () => { await Promise.resolve(); resolve({ request_id: 'old-stop', server_instance: 'old-server', workspace_id: 'old-workspace', status: 'observed', message: 'Old station stopped' }) })
    expect(screen.queryByText(/Old station stopped/)).toBeNull()
  })
  it('refuses browser HTTP fallback when the native bridge is absent', async () => {
    const fetch = vi.spyOn(window, 'fetch')
    await expect(bridge.status()).rejects.toThrow('Open the OM0RX-xNexus desktop')
    expect(fetch).not.toHaveBeenCalled()
    fetch.mockRestore()
  })
})
