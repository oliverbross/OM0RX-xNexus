// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { disconnected, type View } from './api'
import { rxApi, type RxView } from './rxApi'
import { useOwnerSpectrum } from './useOwnerSpectrum'
import frame from './fixtures/spectrum-frame.json'
import serializedView from './fixtures/rx-context-view.json'

const receive = () => structuredClone(serializedView) as RxView
function owner(): View {
  return { ...disconnected, paired: true, connected: true, hello: {
    application: 'OM0RX-xLog', protocol_version: 1, server_instance: frame.context.server_instance,
    workspace_id: frame.context.workspace_id, capabilities: ['audio.rx'],
  } }
}
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.useRealTimers() })
describe('original waterfall host lifecycle', () => {
  it('returns actual bins once, then clears the reset key at the original lease deadline', async () => {
    vi.useFakeTimers(); let now = 100; vi.spyOn(performance, 'now').mockImplementation(() => now)
    const read = vi.fn().mockResolvedValue(frame)
    const input = { session: 1, owner: owner(), receiver: rxApi, view: receive(), pending: false, observation: 1 }
    const { result } = renderHook(() => useOwnerSpectrum(input, read))
    expect(await result.current.getSpectrumRow()).toEqual(frame.spectrum)
    const key = result.current.resetKey
    expect(await result.current.getSpectrumRow()).toBeNull()
    now += frame.remaining_valid_ms
    act(() => { vi.advanceTimersByTime(100) })
    expect(result.current.resetKey).not.toBe(key)
    expect(await result.current.getSpectrumRow()).toBeNull()
  })
  it('suspends synchronously and rejects an in-flight frame until a post-action observation', async () => {
    vi.spyOn(performance, 'now').mockReturnValue(100)
    let finish!: (value: unknown) => void
    const pending = new Promise<unknown>(resolve => { finish = resolve })
    const read = vi.fn().mockReturnValueOnce(pending).mockResolvedValue(frame)
    const input = { session: 1, owner: owner(), receiver: rxApi, view: receive(), pending: false, observation: 1 }
    const { result, rerender } = renderHook(props => useOwnerSpectrum(props, read), { initialProps: input })
    const old = result.current.getSpectrumRow()
    act(() => result.current.suspend())
    finish(frame)
    expect(await old).toBeNull()
    expect(await result.current.getSpectrumRow()).toBeNull()
    rerender({ ...input, pending: true, observation: 2 })
    expect(await result.current.getSpectrumRow()).toBeNull()
    rerender({ ...input, observation: 2 })
    expect(await result.current.getSpectrumRow()).toEqual(frame.spectrum)
  })
  it('retires same-route worker epochs before reading or publishing another frame', async () => {
    vi.spyOn(performance, 'now').mockReturnValue(100)
    const read = vi.fn().mockResolvedValue(frame)
    const input = { session: 1, owner: owner(), receiver: rxApi, view: receive(), pending: false, observation: 1 }
    const { result, rerender } = renderHook(props => useOwnerSpectrum(props, read), { initialProps: input })
    expect(await result.current.getSpectrumRow()).toEqual(frame.spectrum)
    const key = result.current.resetKey
    const next = receive(); next.current_context!.worker_epoch++
    rerender({ ...input, view: next, observation: 2 })
    expect(result.current.resetKey).not.toBe(key)
    expect(await result.current.getSpectrumRow()).toBeNull()
  })
})
