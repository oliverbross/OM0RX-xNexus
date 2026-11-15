// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { RotorStrip } from './RotorStrip'
import type { ExternalRotorPresentation } from './rotorExternal'
const native = vi.hoisted(() => ({ getSettings: vi.fn(), readRotator: vi.fn(), getDeclination: vi.fn(),
  getSatTrackStatus: vi.fn(), getSatTransponder: vi.fn(), stopRotator: vi.fn(), stopSatTrack: vi.fn() }))
vi.mock('../api', () => native)
vi.mock('../toast', () => ({ pushToast: vi.fn() }))
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.clearAllMocks(); vi.useRealTimers() })
const source = (): ExternalRotorPresentation => ({ kind: 'xlog', targetKey: 'exact-route',
  observation: { azimuthDeg: 171, moving: null, deadline: 500 }, observationCurrent: () => true, canStop: () => true, stop: vi.fn(async () => {}) })
describe('original RotorStrip owner presentation', () => {
  it('shows real azimuth then unknown at the original deadline while keeping Stop with no native readers', async () => {
    vi.useFakeTimers(); let now = 100; vi.spyOn(performance, 'now').mockImplementation(() => now)
    const external = source(); render(<RotorStrip external={external} />)
    expect(screen.getByText('171°T')).toBeTruthy()
    now = 500; await act(async () => { await vi.advanceTimersByTimeAsync(100) })
    expect(screen.queryByText('171°T')).toBeNull(); expect(screen.getByText('—')).toBeTruthy()
    fireEvent.click(screen.getByRole('button', { name: 'Stop rotation NOW' }))
    expect(external.stop).toHaveBeenCalledOnce()
    for (const method of Object.values(native)) expect(method).not.toHaveBeenCalled()
  })
  it('does not invent an angle for connected-but-null readback and keeps Stop', () => {
    vi.spyOn(performance, 'now').mockReturnValue(100); const external = source()
    render(<RotorStrip external={{ ...external, observation: { ...external.observation!, azimuthDeg: null } }} />)
    expect(screen.getByText('—')).toBeTruthy(); expect((screen.getByRole('button') as HTMLButtonElement).disabled).toBe(false)
    expect(document.querySelector('svg')).toBeNull()
  })
  it('hides the old observation and action when their connection retires', () => {
    vi.spyOn(performance, 'now').mockReturnValue(100); const external = source()
    const { rerender } = render(<RotorStrip external={external} />)
    rerender(<RotorStrip external={{ ...external, observationCurrent: () => false, canStop: () => false }} />)
    expect(screen.queryByRole('group')).toBeNull()
  })
})
