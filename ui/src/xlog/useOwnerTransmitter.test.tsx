// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useOwnerTransmitter, type FtScope } from './useOwnerTransmitter'
import { ftOff, type FtApi, type FtView } from './ftApi'
const scope: FtScope = { session: 1, connectionToken: 'one', capable: true, connected: true }
const view = (token = 'one'): FtView => ({ ...ftOff, connection_token: token, available: true, mode: 'FT8', remaining_valid_ms: 1000 })
const flush = async () => { await act(async () => { await Promise.resolve(); await Promise.resolve() }) }
function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>(done => { resolve = done }); return { promise, resolve } }
function api(): FtApi { return { status: vi.fn(async token => view(token)), action: vi.fn(async token => view(token)), log: vi.fn(async () => []) } }
let clock = 100
beforeEach(() => { vi.useFakeTimers(); clock = 100; vi.spyOn(performance, 'now').mockImplementation(() => clock) })
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.useRealTimers() })
describe('native owner FT controls', () => {
  it('observes an unarmed station without enabling or calling it', async () => {
    const transport = api(); const { result } = renderHook(() => useOwnerTransmitter(scope, transport)); await flush()
    expect(result.current.fresh).toBe(true); expect(result.current.view.armed).toBe(false)
    await act(async () => { clock += 250; await vi.advanceTimersByTimeAsync(250) })
    expect(transport.action).not.toHaveBeenCalled(); expect(transport.log).not.toHaveBeenCalled()
  })
  it('does not call a native TX endpoint without the advertised capability', async () => {
    const transport = api(); const { result } = renderHook(() => useOwnerTransmitter({ ...scope, capable: false }, transport)); await flush()
    await act(async () => { await result.current.action({ kind: 'cq' }) })
    expect(transport.status).not.toHaveBeenCalled(); expect(transport.action).not.toHaveBeenCalled()
  })
  it('rejects mismatched connection evidence instead of authorizing a start', async () => {
    const transport = api(); vi.mocked(transport.status).mockResolvedValue(view('foreign'))
    const { result } = renderHook(() => useOwnerTransmitter(scope, transport)); await flush()
    await act(async () => { await result.current.action({ kind: 'cq' }) })
    expect(result.current.fresh).toBe(false); expect(transport.action).not.toHaveBeenCalled()
  })
  it('keeps exact Stop callable after evidence expiry and while an ordinary action is pending', async () => {
    const transport = api(), pending = deferred<FtView>()
    vi.mocked(transport.action).mockImplementation((token, action) => action.kind === 'cq' ? pending.promise : Promise.resolve(view(token)))
    const { result } = renderHook(() => useOwnerTransmitter(scope, transport)); await flush()
    act(() => { void result.current.action({ kind: 'cq' }) }); await flush(); expect(result.current.busy).toBe(true)
    clock += 1200
    await act(async () => { await result.current.action({ kind: 'stop' }) })
    expect(transport.action).toHaveBeenLastCalledWith('one', { kind: 'stop' })
    await act(async () => { pending.resolve({ ...view(), armed: true }); await pending.promise })
    expect(result.current.view.armed).toBe(false)
  })
  it('retains the release obligation on failed polls and refuses a fresh start', async () => {
    const transport = api(); vi.mocked(transport.status).mockResolvedValueOnce({ ...view(), operation_id: 'retained', release_pending: true }).mockRejectedValue(new Error('offline'))
    const { result } = renderHook(() => useOwnerTransmitter(scope, transport)); await flush()
    await act(async () => { clock += 300; await vi.advanceTimersByTimeAsync(250) })
    expect(result.current.fresh).toBe(false); expect(result.current.view.operation_id).toBe('retained')
    await act(async () => { await result.current.action({ kind: 'cq' }) }); expect(transport.action).not.toHaveBeenCalled()
    await act(async () => { await result.current.action({ kind: 'stop' }) }); expect(transport.action).toHaveBeenCalledOnce()
  })
  it('cannot restore an old action after the connection changes', async () => {
    const transport = api(), pending = deferred<FtView>(); vi.mocked(transport.action).mockReturnValue(pending.promise)
    const { result, rerender } = renderHook((binding: FtScope) => useOwnerTransmitter(binding, transport), { initialProps: scope }); await flush()
    act(() => { void result.current.action({ kind: 'cq' }) }); await flush()
    rerender({ ...scope, session: 2, connectionToken: 'two' }); await flush()
    await act(async () => { pending.resolve({ ...view(), armed: true }); await pending.promise })
    expect(result.current.view.connection_token).toBe('two'); expect(result.current.view.armed).toBe(false)
  })
  it('submits Log once and never retries an uncertain submission automatically', async () => {
    const transport = api(); vi.mocked(transport.log).mockRejectedValue(new Error('uncertain'))
    const { result } = renderHook(() => useOwnerTransmitter(scope, transport)); await flush()
    await act(async () => { await result.current.log() }); expect(transport.log).toHaveBeenCalledExactlyOnceWith('one')
    await act(async () => { clock += 500; await vi.advanceTimersByTimeAsync(500) }); expect(transport.log).toHaveBeenCalledOnce()
  })
})
