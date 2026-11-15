// @vitest-environment jsdom
import { useLayoutEffect } from 'react'
import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { disconnected, type Bridge, type Receipt, type View } from './api'
import { useOwnerConnection } from './useOwnerConnection'

function pending<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done }); return { promise, resolve } }
function api(status: Promise<View>): Bridge {
  return { status: vi.fn(() => status), importPairing: vi.fn(), connect: vi.fn(),
    disconnect: vi.fn(), poll: vi.fn(), command: vi.fn(), outbox: vi.fn(),
    enqueue: vi.fn(), deliver: vi.fn(), history: vi.fn() }
}
function paired(id: string): View {
  return { ...disconnected, paired: true, connected: false, hello: {
    application: 'OM0RX-xLog', protocol_version: 1, server_instance: id,
    workspace_id: id, capabilities: [],
  } }
}
function currentView(): View {
  return { ...paired('A'), connected: true, snapshot: { protocol_version: 1, server_instance: 'A', workspace_id: 'A',
    revision: 1, generated_at_unix_ms: Date.now(), max_age_ms: 10000,
    station: { connected: true, radio_id: 'radio1', frequency_hz: 14074000, mode: 'DIGU', transmitting: false },
    rotor: { connected: true, rotor_id: 'rotor1', azimuth_deg: 90, moving: null }, spots: [] } }
}
const receipt = (id: string): Receipt => ({ request_id: id, server_instance: 'A', workspace_id: 'A', status: 'accepted', message: null })
const tune = { kind: 'station.tune', radio_id: 'radio1', frequency_hz: 14074000, mode: 'DIGU' } as const
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.useRealTimers() })
describe('original host connection ownership', () => {
  it('mount reads status only and never starts a station or pairs automatically', async () => {
    const owner = api(Promise.resolve(paired('A')))
    const { result } = renderHook(() => useOwnerConnection(owner))
    await act(async () => {})
    expect(result.current.view.hello?.server_instance).toBe('A')
    expect(owner.status).toHaveBeenCalledOnce()
    for (const command of [owner.connect, owner.importPairing, owner.command, owner.enqueue]) expect(command).not.toHaveBeenCalled()
  })
  it('withholds old source facts during the first replacement render and rejects late old status', async () => {
    const late = pending<View>(), next = pending<View>()
    const a = api(late.promise), b = api(next.promise)
    const seen: Array<{ api: Bridge; view: View }> = []
    const { result, rerender } = renderHook(({ source }) => {
      const connection = useOwnerConnection(source)
      useLayoutEffect(() => { seen.push({ api: source, view: connection.view }) })
      return connection
    }, { initialProps: { source: a } })
    await act(async () => { late.resolve(paired('A')) })
    rerender({ source: b })
    expect(seen.filter(value => value.api === b).every(value => value.view.hello === null)).toBe(true)
    await act(async () => { next.resolve(paired('B')) })
    expect(result.current.view.hello?.server_instance).toBe('B')
  })
  it('never publishes a status response from a replaced source', async () => {
    const old = pending<View>(), next = pending<View>()
    const a = api(old.promise), b = api(next.promise)
    const { result, rerender } = renderHook(({ source }) => useOwnerConnection(source), { initialProps: { source: a } })
    rerender({ source: b })
    await act(async () => { old.resolve(paired('A')) })
    expect(result.current.view.hello).toBeNull()
    await act(async () => { next.resolve(paired('B')) })
    expect(result.current.view.hello?.server_instance).toBe('B')
  })
  it('restores control observations only from a poll started after ordinary command completion', async () => {
    vi.useFakeTimers(); vi.spyOn(performance, 'now').mockReturnValue(100)
    const before = pending<View>(), during = pending<View>(), after = pending<View>(), command = pending<Receipt>()
    const owner = api(Promise.resolve(currentView()))
    vi.mocked(owner.poll).mockReturnValueOnce(before.promise).mockReturnValueOnce(during.promise).mockReturnValueOnce(after.promise)
    vi.mocked(owner.command).mockReturnValue(command.promise)
    const { result } = renderHook(() => useOwnerConnection(owner))
    await act(async () => {})
    expect(result.current.lease).not.toBeNull()
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    let dispatched!: Promise<void>
    act(() => { dispatched = result.current.command(tune) })
    await act(async () => { before.resolve(currentView()) })
    expect(result.current.lease).toBeNull()
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    await act(async () => { command.resolve(receipt('tune')); await dispatched })
    await act(async () => { during.resolve(currentView()) })
    expect(result.current.lease).toBeNull()
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    await act(async () => { after.resolve(currentView()) })
    expect(result.current.lease).not.toBeNull()
    expect(owner.command).toHaveBeenCalledOnce()
  })
  it('refuses a second ordinary dispatch synchronously but allows independent Stop and preserves its receipt', async () => {
    const command = pending<Receipt>(), owner = api(Promise.resolve(currentView()))
    vi.mocked(owner.command).mockImplementation(value => value.kind === 'rotor.stop' ? Promise.resolve(receipt('stop')) : command.promise)
    const { result } = renderHook(() => useOwnerConnection(owner))
    await act(async () => {})
    let first!: Promise<void>, duplicate!: Promise<unknown>
    await act(async () => {
      first = result.current.command(tune)
      duplicate = result.current.command(tune).catch(error => error)
      await result.current.command({ kind: 'rotor.stop', rotor_id: 'rotor1' })
    })
    expect(await duplicate).toBeInstanceOf(Error)
    expect(vi.mocked(owner.command).mock.calls.map(call => call[0].kind)).toEqual(['station.tune', 'rotor.stop'])
    await act(async () => { command.resolve(receipt('tune')); await first })
    expect(result.current.receipt?.request_id).toBe('stop')
    expect(result.current.lease).toBeNull()
  })
})
