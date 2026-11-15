// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { disconnected, type Bridge, type Receipt, type View } from './api'
import { useOwnerConnection } from './useOwnerConnection'
import type { ControlScope } from './controlObservation'
const requestId = '11111111-1111-4111-8111-111111111111'
const point = { kind: 'rotor.point', rotor_id: 'exact-route', azimuth_deg: 90 } as const
const expected = (connection: ReturnType<typeof useOwnerConnection>): ControlScope => ({
  connectionToken: connection.view.connection_token!, sessionGeneration: connection.session,
  serverInstance: connection.view.hello!.server_instance, workspaceId: connection.view.hello!.workspace_id,
})
function pending<T>() { let resolve!: (v: T) => void; const promise = new Promise<T>(r => { resolve = r }); return { promise, resolve } }
function view(revision = 1): View {
  return { ...disconnected, paired: true, connected: true, connection_token: 'local-token',
    hello: { application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'owner', workspace_id: 'workspace', capabilities: ['rotor.point','rotor.stop'] },
    snapshot: { protocol_version: 1, server_instance: 'owner', workspace_id: 'workspace', revision, generated_at_unix_ms: Date.now(), max_age_ms: 3000,
      station: { connected: false, radio_id: null, frequency_hz: null, mode: null, transmitting: null },
      rotor: { connected: true, rotor_id: 'exact-route', azimuth_deg: 90, moving: null }, spots: [] } }
}
const receipt = (): Receipt => ({ request_id: requestId, server_instance: 'owner', workspace_id: 'workspace', status: 'accepted', message: null })
function api(initial = view()): Bridge {
  return { status: vi.fn(async () => initial), poll: vi.fn(async () => view(2)), importPairing: vi.fn(), connect: vi.fn(),
    disconnect: vi.fn(), command: vi.fn(async () => receipt()), outbox: vi.fn(), enqueue: vi.fn(), deliver: vi.fn(), history: vi.fn() }
}
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.useRealTimers() })
describe('actual hook poll provenance and observed dispatch', () => {
  it('does not count initial status or connect replies as a Point baseline', async () => {
    const owner = api(); vi.mocked(owner.connect).mockResolvedValue(view(2))
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    expect(result.current.lease).not.toBeNull(); expect(result.current.poll).toBeNull()
    await act(async () => { await result.current.observedPoint(point, expected(result.current), requestId) })
    expect(result.current.pointEvidence).toMatchObject({ submitted: false, settlement: { kind: 'locally_refused' } })
    await act(async () => { await result.current.connection('connect') })
    expect(result.current.lease).not.toBeNull(); expect(result.current.poll).toBeNull()
    expect(owner.command).not.toHaveBeenCalled()
  })
  it('publishes the actual request start and requires a post-completion-started poll for later matching', async () => {
    vi.useFakeTimers(); let mono = 100; vi.spyOn(performance, 'now').mockImplementation(() => mono)
    const baseline = pending<View>(), before = pending<View>(), after = pending<View>(), sent = pending<Receipt>(), owner = api()
    vi.mocked(owner.poll).mockReturnValueOnce(baseline.promise).mockReturnValueOnce(before.promise).mockReturnValueOnce(after.promise)
    vi.mocked(owner.command).mockReturnValue(sent.promise)
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    mono = 200; await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    mono = 260; await act(async () => { baseline.resolve(view(2)) })
    expect(result.current.poll?.requestStartedMono).toBe(200)
    expect(result.current.poll?.deadline).toBe(result.current.lease?.deadline)
    mono = 300; await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    let outcome!: ReturnType<typeof result.current.observedPoint>
    mono = 400; act(() => { outcome = result.current.observedPoint(point, expected(result.current), requestId) })
    expect(owner.command).toHaveBeenCalledExactlyOnceWith(point, requestId)
    mono = 500; await act(async () => { before.resolve(view(3)) })
    expect(result.current.poll).toBeNull()
    mono = 600; await act(async () => { sent.resolve(receipt()); await outcome })
    expect(result.current.pointEvidence).toMatchObject({ submitted: true, settlement: { kind: 'receipt' }, observation: { kind: 'unknown' } })
    mono = 700; await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    mono = 800; await act(async () => { after.resolve(view(4)) })
    expect(result.current.poll?.requestStartedMono).toBe(700)
    expect(result.current.pointEvidence).toMatchObject({ settlement: { kind: 'receipt', receipt: { status: 'accepted' } },
      observation: { kind: 'snapshot', matchesTarget: true, moving: null } })
  })
  it('fences a pre-Stop poll without waiting for Stop or dispatching Point from that old baseline', async () => {
    vi.useFakeTimers(); let mono = 100; vi.spyOn(performance, 'now').mockImplementation(() => mono)
    const old = pending<View>(), stopped = pending<Receipt>(), owner = api()
    vi.mocked(owner.poll).mockResolvedValueOnce(view(2)).mockReturnValueOnce(old.promise)
    vi.mocked(owner.command).mockReturnValue(stopped.promise)
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    mono = 200; await act(async () => { await vi.advanceTimersByTimeAsync(500) }); expect(result.current.poll).not.toBeNull()
    mono = 300; await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    let stop!: Promise<void>
    act(() => { stop = result.current.command({ kind: 'rotor.stop', rotor_id: 'exact-route' }) })
    expect(owner.command).toHaveBeenCalledTimes(1)
    expect(owner.command).toHaveBeenCalledWith({ kind: 'rotor.stop', rotor_id: 'exact-route' })
    await act(async () => { old.resolve(view(3)) }); expect(result.current.poll).toBeNull()
    await act(async () => { await result.current.observedPoint(point, expected(result.current), requestId) })
    expect(owner.command).toHaveBeenCalledTimes(1)
    expect(result.current.pointEvidence?.settlement.kind).toBe('locally_refused')
    await act(async () => { stopped.resolve(receipt()); await stop })
    expect(result.current.poll).toBeNull()
  })
  it('preserves historical settlement after source retirement but never publishes it as the current receipt', async () => {
    vi.useFakeTimers(); const sent = pending<Receipt>(), owner = api()
    vi.mocked(owner.command).mockReturnValue(sent.promise)
    vi.mocked(owner.connect).mockResolvedValue({ ...view(4), connection_token: 'replacement-token' })
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    let outcome!: ReturnType<typeof result.current.observedPoint>
    act(() => { outcome = result.current.observedPoint(point, expected(result.current), requestId) })
    await act(async () => { await result.current.connection('connect') })
    await act(async () => { sent.resolve(receipt()); await outcome })
    expect(result.current.pointEvidence).toMatchObject({ retired: true, settlement: { kind: 'receipt', receipt: receipt() }, observation: { kind: 'unknown' } })
    expect(result.current.receipt).toBeNull(); expect(owner.command).toHaveBeenCalledTimes(1)
  })
  it('keeps a lost submitted outcome uncertain with no automatic retry', async () => {
    vi.useFakeTimers(); const owner = api(); vi.mocked(owner.command).mockRejectedValue(new Error('lost reply'))
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    await act(async () => { await result.current.observedPoint(point, expected(result.current), requestId) })
    expect(result.current.pointEvidence).toMatchObject({ submitted: true, settlement: { kind: 'uncertain' } })
    await act(async () => { await vi.advanceTimersByTimeAsync(1500) })
    expect(owner.command).toHaveBeenCalledTimes(1)
  })
  it('rejects absent capability locally even with a fresh successful poll', async () => {
    vi.useFakeTimers(); const noCap = view(2); noCap.hello!.capabilities = ['rotor.stop']; const owner = api(noCap)
    vi.mocked(owner.poll).mockResolvedValue(noCap)
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    await act(async () => { await result.current.observedPoint(point, expected(result.current), requestId) })
    expect(result.current.pointEvidence).toMatchObject({ submitted: false, settlement: { kind: 'locally_refused' } })
    expect(owner.command).not.toHaveBeenCalled()
  })
  it('allows immediate Stop during pending Point and preserves each distinct receipt without retry', async () => {
    vi.useFakeTimers(); const sent = pending<Receipt>(), owner = api()
    const stopReceipt = { ...receipt(), request_id: '22222222-2222-4222-8222-222222222222' }
    vi.mocked(owner.command).mockImplementation(value => value.kind === 'rotor.stop' ? Promise.resolve(stopReceipt) : sent.promise)
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    let outcome!: ReturnType<typeof result.current.observedPoint>
    act(() => { outcome = result.current.observedPoint(point, expected(result.current), requestId) })
    await act(async () => { await result.current.command({ kind: 'rotor.stop', rotor_id: 'exact-route' }) })
    expect(owner.command).toHaveBeenNthCalledWith(2, { kind: 'rotor.stop', rotor_id: 'exact-route' })
    expect(result.current.receipt).toEqual(stopReceipt)
    await act(async () => { sent.resolve(receipt()); await outcome })
    expect(result.current.pointEvidence).toMatchObject({ retired: true, settlement: { kind: 'receipt', receipt: receipt() } })
    expect(result.current.receipt).toEqual(stopReceipt); expect(owner.command).toHaveBeenCalledTimes(2)
  })
  it('does not retarget a stale caller scope after reconnect even when the rotor ID is unchanged', async () => {
    vi.useFakeTimers(); const owner = api()
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    const oldCall = result.current.observedPoint, oldScope = expected(result.current)
    const replacement = { ...view(4), connection_token: 'new-token' }
    vi.mocked(owner.connect).mockResolvedValue(replacement); vi.mocked(owner.poll).mockResolvedValue(replacement)
    await act(async () => { await result.current.connection('connect') })
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    expect(result.current.poll?.scope.connectionToken).toBe('new-token')
    await act(async () => { await oldCall(point, oldScope, requestId) })
    expect(owner.command).not.toHaveBeenCalled()
    expect(result.current.pointEvidence).toMatchObject({ submitted: false, settlement: { kind: 'locally_refused' } })
  })
  it.each(['rollback', 'throw'] as const)('makes no IPC for a pre-admission %s clock fault and still allows Stop', async failure => {
    vi.useFakeTimers(); const owner = api()
    const clock = vi.spyOn(performance, 'now').mockReturnValue(100)
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    // Construction sees 100, dispatch's ordinary lease check sees 100, then the
    // contained observer admission sees rollback/throw before any Bridge call.
    clock.mockReturnValueOnce(100).mockReturnValueOnce(100).mockImplementationOnce(() => {
      if (failure === 'throw') throw Error('clock unavailable')
      return 99
    })
    await act(async () => { await result.current.observedPoint(point, expected(result.current), requestId) })
    expect(result.current.pointEvidence).toMatchObject({ submitted: false, retired: true, settlement: { kind: 'locally_refused' } })
    expect(owner.command).not.toHaveBeenCalled()
    await act(async () => { await result.current.command({ kind: 'rotor.stop', rotor_id: 'exact-route' }) })
    expect(owner.command).toHaveBeenCalledExactlyOnceWith({ kind: 'rotor.stop', rotor_id: 'exact-route' })
  })
  it('preserves successful Point receipt after observation-clock failure and allows immediate independent Stop', async () => {
    vi.useFakeTimers(); const owner = api(), sent = pending<Receipt>()
    const clock = vi.spyOn(performance, 'now').mockReturnValue(100)
    const stopReceipt = { ...receipt(), request_id: '22222222-2222-4222-8222-222222222222' }
    vi.mocked(owner.command).mockImplementation(value => value.kind === 'rotor.stop' ? Promise.resolve(stopReceipt) : sent.promise)
    const { result } = renderHook(() => useOwnerConnection(owner)); await act(async () => {})
    await act(async () => { await vi.advanceTimersByTimeAsync(500) })
    let outcome!: ReturnType<typeof result.current.observedPoint>
    act(() => { outcome = result.current.observedPoint(point, expected(result.current), requestId) })
    await act(async () => {
      clock.mockImplementationOnce(() => { throw Error('observation clock unavailable') })
      sent.resolve(receipt()); await outcome
    })
    expect(result.current.pointEvidence).toMatchObject({ submitted: true, retired: true,
      settlement: { kind: 'receipt', receipt: receipt() }, observation: { kind: 'unknown' } })
    expect(result.current.receipt).toEqual(receipt())
    await act(async () => { await result.current.command({ kind: 'rotor.stop', rotor_id: 'exact-route' }) })
    expect(owner.command).toHaveBeenCalledTimes(2)
    expect(result.current.receipt).toEqual(stopReceipt)
    expect(result.current.pointEvidence?.settlement).toEqual({ kind: 'receipt', receipt: receipt() })
  })
})
