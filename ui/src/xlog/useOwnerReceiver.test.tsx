// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useOwnerReceiver } from './useOwnerReceiver'
import { rxOff, type RxApi, type RxView } from './rxApi'
import { currentNeeds } from './rxNeeds'
import serializedRows from './fixtures/decoded-needs.json'
import serializedContext from './fixtures/rx-context-view.json'

function pending<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done }); return { promise, resolve } }
function received(): RxView { return { ...structuredClone(serializedContext), rows: structuredClone(serializedRows.rows) } as RxView }
function scope() {
  const context = serializedContext.current_context
  return { session: 1, owner: { server_instance: context.server_instance, workspace_id: context.workspace_id },
    connected: true, fresh: true, audioCapable: true, needsCapable: true }
}
function source(): RxApi { return { status: vi.fn(async () => rxOff), start: vi.fn(async () => received()), stop: vi.fn(async () => rxOff) } }
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.useRealTimers() })

describe('original receiver lifecycle', () => {
  it('observes an existing worker without automatically starting or restarting it', async () => {
    vi.useFakeTimers()
    const api = source()
    const { result } = renderHook(() => useOwnerReceiver(scope(), api))
    await act(async () => {})
    await act(async () => { await vi.advanceTimersByTimeAsync(1500) })
    expect(api.status).toHaveBeenCalledTimes(4)
    expect(api.start).not.toHaveBeenCalled()
    expect(api.stop).not.toHaveBeenCalled()
    expect(result.current.view.enabled).toBe(false)
  })

  it('allows Stop while Start is pending and ignores the late Start result', async () => {
    const start = pending<RxView>(), api = source()
    vi.mocked(api.start).mockReturnValue(start.promise)
    const { result } = renderHook(() => useOwnerReceiver(scope(), api))
    await act(async () => {})
    let starting!: Promise<void>
    act(() => { starting = result.current.receiving(true) })
    expect(result.current.pending).toBe(true)
    await act(async () => { await result.current.receiving(false) })
    expect(api.stop).toHaveBeenCalledOnce()
    expect(result.current.view.enabled).toBe(false)
    await act(async () => { start.resolve(received()); await starting })
    expect(result.current.view.enabled).toBe(false)
    expect(result.current.view.rows).toEqual([])
    expect(result.current.leases.size).toBe(0)
    expect(result.current.pending).toBe(false)
  })

  it('retires pending Start on a new connection session without starting the replacement', async () => {
    const start = pending<RxView>(), api = source(), initial = scope()
    vi.mocked(api.start).mockReturnValue(start.promise)
    const { result, rerender } = renderHook(value => useOwnerReceiver(value, api), { initialProps: initial })
    await act(async () => {})
    let starting!: Promise<void>
    act(() => { starting = result.current.receiving(true) })
    rerender({ ...initial, session: 2 })
    await act(async () => {})
    await act(async () => { start.resolve(received()); await starting })
    expect(result.current.view.enabled).toBe(false)
    expect(result.current.view.rows).toEqual([])
    expect(result.current.pending).toBe(false)
    expect(api.start).toHaveBeenCalledOnce()
  })

  it('recovers valid facts after a short read failure without extending their original deadline', async () => {
    vi.useFakeTimers(); let at = 100
    vi.spyOn(performance, 'now').mockImplementation(() => at)
    const api = source(), ownerScope = scope(), view = received()
    vi.mocked(api.status).mockResolvedValueOnce(view).mockRejectedValueOnce(Error('Synthetic RX status unavailable')).mockResolvedValue(view)
    const { result } = renderHook(() => useOwnerReceiver(ownerScope, api))
    await act(async () => {})
    const facts = () => currentNeeds(result.current.view.rows[0], result.current.view, result.current.leases, at, ownerScope.owner)
    expect(facts()?.status).toBe('needed')
    const deadline = 100 + view.rows[0].needs.remaining_valid_ms
    at = 350; await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    expect(result.current.pollError).toBe('Synthetic RX status unavailable')
    expect(result.current.view.rows[0].decode_id).toBe(view.rows[0].decode_id)
    expect(facts()?.status).toBe('needed')
    at = 600; await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    expect(result.current.pollError).toBe('')
    expect(facts()?.status).toBe('needed')
    expect(result.current.leases.get(view.rows[0].decode_id)?.deadline).toBe(deadline)
    at = deadline + 1
    expect(facts()).toBeNull()
    expect(api.start).not.toHaveBeenCalled()
  })

  it('withholds a previous receiver source and rejects its late status result', async () => {
    const old = pending<RxView>(), a = source(), b = source()
    vi.mocked(a.status).mockReturnValue(old.promise)
    const { result, rerender } = renderHook(api => useOwnerReceiver(scope(), api), { initialProps: a })
    rerender(b)
    await act(async () => { old.resolve(received()) })
    expect(result.current.view.enabled).toBe(false)
    expect(result.current.view.rows).toEqual([])
    expect(result.current.leases.size).toBe(0)
    expect(b.start).not.toHaveBeenCalled()
  })
})
describe('temporary owner observation gaps', () => {
  it('preserves passive facts across observation gaps without renewing live authority', async () => {
    vi.useFakeTimers(); let at = 100
    vi.spyOn(performance, 'now').mockImplementation(() => at)
    const api = source(), initial = scope(), view = received()
    view.rows[0].needs.display_valid = true
    view.rows[0].needs.remaining_valid_ms = 0
    vi.mocked(api.status).mockResolvedValue(view)
    const { result, rerender } = renderHook(value => useOwnerReceiver(value, api), {initialProps:initial})
    await act(async () => {})
    const original = result.current.leases.get(view.rows[0].decode_id)?.deadline
    rerender({...initial, fresh:false})
    at=5000; await act(async () => {await vi.advanceTimersByTimeAsync(500)})
    expect(currentNeeds(result.current.view.rows[0], result.current.view, result.current.leases, at, initial.owner)?.status).toBe('needed')
    rerender(initial); await act(async () => {})
    expect(result.current.leases.get(view.rows[0].decode_id)?.deadline).toBe(original)
    expect(currentNeeds(result.current.view.rows[0], result.current.view, result.current.leases, at, initial.owner)?.status).toBe('needed')
    const invalidated = received()
    invalidated.rows[0].needs = { ...invalidated.rows[0].needs, display_valid:false,
      remaining_valid_ms:0, owner:null, authority:null }
    vi.mocked(api.status).mockResolvedValue(invalidated)
    await act(async () => {await vi.advanceTimersByTimeAsync(500)})
    expect(currentNeeds(result.current.view.rows[0], result.current.view, result.current.leases, at, initial.owner)).toBeNull()
    expect(result.current.view.rows).toHaveLength(view.rows.length)
  })

  it('retries a not-ready auto-start with remembered mode then respects ordinary Stop', async () => {
    vi.useFakeTimers()
    localStorage.setItem('xlog.rx.mode', 'FT4')
    const api = source(), initial = { ...scope(), autoStart:true }
    vi.mocked(api.start).mockRejectedValueOnce(Error('Owner capture not ready')).mockResolvedValue(received())
    const { result } = renderHook(() => useOwnerReceiver(initial, api))
    await act(async () => {})
    expect(api.start).toHaveBeenCalledExactlyOnceWith('FT4')
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    expect(api.start).toHaveBeenCalledTimes(2)
    expect(api.start).toHaveBeenLastCalledWith('FT4')
    await act(async () => { await result.current.receiving(false) })
    await act(async () => { await vi.advanceTimersByTimeAsync(1500) })
    expect(api.start).toHaveBeenCalledTimes(2)
    expect(api.stop).toHaveBeenCalledTimes(1)
    localStorage.removeItem('xlog.rx.mode')
  })
})
