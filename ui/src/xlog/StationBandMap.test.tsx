// @vitest-environment jsdom
import { fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { StationBandMap } from './StationBandMap'
import type { Spot } from './api'

const spot: Spot = { id: 'spot-a', callsign: 'OM0RX', band: '20m', country: 'Slovak Republic', frequency_hz: 14_025_000, mode: 'CW', source: 'owner-cluster', observed_at_unix_ms: Date.now(), need: { status: 'unknown', score: 0, reasons: [] } }
afterEach(() => { delete window.__TAURI__ })
describe('Nexus band map with xLog observations', () => {
  it('uses existing renderer for owner spots without invoking native Nexus actions', () => {
    const invoke = vi.fn(); window.__TAURI__ = { core: { invoke } }
    const select = vi.fn()
    const { container } = render(<StationBandMap spots={[spot]} dialHz={14_074_000} select={select} />)
    const button = screen.getByRole('button', { name: /OM0RX/ })
    fireEvent.click(button)
    expect(select).toHaveBeenCalledWith(spot)
    fireEvent.contextMenu(button)
    fireEvent.keyDown(button, { key: 'F10', shiftKey: true })
    const track = container.querySelector('.bandmap-track')!
    fireEvent.wheel(track, { deltaY: 120 })
    fireEvent.click(track, { clientY: 30 })
    expect(invoke).not.toHaveBeenCalled()
    expect(container.querySelector('.bandmap-spot.worked')).toBeNull()
    expect(screen.getByTitle('Need status unknown in xLog')).toBeTruthy()
  })
  it('plots digital observations without relabelling them as phone', () => {
    render(<StationBandMap spots={[{ ...spot, mode: 'FT8', frequency_hz: 14_074_000 }]} dialHz={14_074_000} select={vi.fn()} />)
    expect(screen.queryByRole('button', { name: /OM0RX/ })).toBeNull()
    fireEvent.change(screen.getByLabelText('Band map mode'), { target: { value: 'Digital' } })
    expect(screen.getByRole('button', { name: /OM0RX/ })).toBeTruthy()
  })
  it('normalizes band presentation and never marks mixed owner evidence as worked', () => {
    const { container } = render(<StationBandMap spots={[{ ...spot, band: '20M', mode: 'cw', need: { status: 'worked', score: 0, reasons: [] } }, { ...spot, id: 'other', band: '20M', frequency_hz: 14_030_000, need: { status: 'needed', score: 100, reasons: ['Needed at this frequency'] } }]} dialHz={14_025_000} select={vi.fn()} />)
    expect(container.querySelectorAll('.bandmap-spot')).toHaveLength(2)
    expect(container.querySelector('.bandmap-spot.worked')).toBeNull()
  })
})
