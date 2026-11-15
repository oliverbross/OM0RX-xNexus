// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { SpotsPanel } from './SpotsPanel'
import type { ExternalSpotRow, ExternalSpotsSource } from './spotsExternal'
import type { SpotRow } from '../types'
import * as api from '../api'

vi.mock('../api', async original => ({
  ...await original<typeof import('../api')>(),
  setPounceSpotterFilter: vi.fn(async () => {}),
  getDxccEntityLocations: vi.fn(async () => [['United States', 39.8, -98.6]]),
  postSpot: vi.fn(async () => {}),
  openQrzPage: vi.fn(async () => {}),
}))

const row = (over: Partial<ExternalSpotRow> = {}): ExternalSpotRow => ({
  id: 'owner-1', callsign: 'K1ABC', frequencyHz: 14_074_123, mode: 'FT8',
  observedAtUnixMs: 10_000, source: 'Owner cluster', country: 'United States', band: '20m',
  assessment: { status: 'needed', score: 71, reasons: ['New Call'] }, ...over,
})
function source(rows = [row()]): ExternalSpotsSource {
  return { kind: 'xlog', key: 'scope-1', state: 'current', deadline: 1000, generatedAtUnixMs: 10_000,
    rows, current: () => true, select: vi.fn() }
}
const rowsOf = (container: HTMLElement) => [...container.querySelectorAll<HTMLElement>('.sp-row')]
beforeEach(() => { vi.clearAllMocks(); localStorage.clear(); sessionStorage.clear(); vi.spyOn(performance, 'now').mockReturnValue(100) })
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.useRealTimers() })

describe('original Spots table with external observations', () => {
  it('uses original table/columns with truthful source/assessment and exact source ID/Hz selection', () => {
    const data = source([row(), row({ id: 'owner-2', frequencyHz: 14_074_456 })])
    const { container } = render(<SpotsPanel external={data} selectedCall={null} />)
    expect(container.querySelector('main.layout.single.spots-panel .np-grid.sp-grid')).toBeTruthy()
    expect(screen.getByText('Source')).toBeTruthy()
    expect(screen.getByText('xLog assessment')).toBeTruthy()
    const rows = rowsOf(container)
    expect(rows).toHaveLength(2)
    expect(rows.map(element => element.dataset.ownerSpotId).sort()).toEqual(['owner-1', 'owner-2'])
    const second = rows.find(element => element.dataset.ownerSpotId === 'owner-2')!
    expect(second.title).toContain('14074456 Hz')
    fireEvent.click(second)
    expect(data.select).toHaveBeenCalledExactlyOnceWith(data.rows[1])
    expect(api.setPounceSpotterFilter).not.toHaveBeenCalled()
    expect(api.getDxccEntityLocations).not.toHaveBeenCalled()
    expect(api.postSpot).not.toHaveBeenCalled()
  })
  it('keeps unsupported native filters/actions out, including mount and right-click paths', () => {
    // Existing independent settings must not hide the owner observations.
    localStorage.setItem('nexus.spots.hiddenModes', JSON.stringify(['FT8']))
    localStorage.setItem('nexus.spots.bands', JSON.stringify(['80m']))
    const data = source(), { container } = render(<SpotsPanel external={data} selectedCall={null} />)
    expect(rowsOf(container)).toHaveLength(1)
    fireEvent.click(screen.getByTitle('Filter spots by band, mode, state, or privileges'))
    expect(screen.getByText(/Reporter, licence, state and worked-date filters are unavailable/)).toBeTruthy()
    expect(screen.queryByRole('checkbox')).toBeNull()
    expect(container.querySelector('.sp-spot-btn')).toBeNull()
    expect(container.querySelector('.np-popout')).toBeNull()
    expect(container.querySelector('.np-az')).toBeNull()
    const call = screen.getByRole('button', { name: 'K1ABC' })
    fireEvent.contextMenu(call)
    fireEvent.keyDown(call, { key: 'ContextMenu' })
    expect(screen.queryByRole('menu')).toBeNull()
    fireEvent.click(call)
    expect(data.select).toHaveBeenCalledExactlyOnceWith(data.rows[0])
    expect(api.openQrzPage).not.toHaveBeenCalled()
    expect(api.getDxccEntityLocations).not.toHaveBeenCalled()
    expect(api.setPounceSpotterFilter).not.toHaveBeenCalled()
    expect(api.postSpot).not.toHaveBeenCalled()
  })
  it('unknown has no score/award colouring; missing fields stay unknown', () => {
    const { container } = render(<SpotsPanel external={source([row({ country: null, band: null, observedAtUnixMs: null,
      assessment: { status: 'unknown', explanation: 'Reclassifying' } })])} selectedCall={null} />)
    expect(screen.getByText('Unknown · Reclassifying')).toBeTruthy()
    expect(screen.queryByText(/xLog score/)).toBeNull()
    expect(container.querySelector('.need-chip,.sp-worked,.np-az')).toBeNull()
    const rendered = rowsOf(container)[0]
    for (const selector of ['.np-age', '.np-name', '.np-band', '.sp-state']) expect(rendered.querySelector(selector)?.textContent).toBe('—')
  })
  it('future receipt and negative wall elapsed display unknown age without withdrawing current rows', () => {
    vi.spyOn(Date, 'now').mockReturnValue(9_500)
    const { container } = render(<SpotsPanel external={source([
      row({ id: 'future-snapshot', observedAtUnixMs: 10_001 }),
      row({ id: 'future-wall', observedAtUnixMs: 10_000 }),
      row({ id: 'past', observedAtUnixMs: 8_000 }),
    ])} selectedCall={null} />)
    expect(rowsOf(container)).toHaveLength(3)
    expect(container.querySelector('[data-owner-spot-id="future-snapshot"] .np-age')?.textContent).toBe('—')
    expect(container.querySelector('[data-owner-spot-id="future-wall"] .np-age')?.textContent).toBe('—')
    expect(container.querySelector('[data-owner-spot-id="past"] .np-age')?.textContent).toBe('1s')
  })
  it('retires at original deadline without requiring another poll and ignores a queued old click', () => {
    vi.useFakeTimers(); let now = 100
    vi.spyOn(performance, 'now').mockImplementation(() => now)
    const data = source(), { container } = render(<SpotsPanel external={data} selectedCall={null} />)
    const oldCall = screen.getByRole('button', { name: 'K1ABC' })
    now = 1000
    fireEvent.click(oldCall) // deadline has passed before the timer repaints.
    expect(data.select).not.toHaveBeenCalled()
    act(() => vi.advanceTimersByTime(100))
    expect(rowsOf(container)).toHaveLength(0)
    expect(screen.getAllByText('Current xLog observations are unavailable.').length).toBeGreaterThan(0)
  })
  it('separates current empty from unavailable, and owner search uses real source/frequency fields', () => {
    const { rerender } = render(<SpotsPanel external={source([])} selectedCall={null} />)
    expect(screen.getByText('No observations supplied in this xLog snapshot.')).toBeTruthy()
    rerender(<SpotsPanel external={{ ...source([]), state: 'unavailable' }} selectedCall={null} />)
    expect(screen.queryByText('No observations supplied in this xLog snapshot.')).toBeNull()
    rerender(<SpotsPanel external={source([row(), row({ id: 'other', callsign: 'W9XYZ', source: 'Second feed', frequencyHz: 7_074_001, band: '40m' })])} selectedCall={null} />)
    fireEvent.change(screen.getByLabelText('Search spots'), { target: { value: 'Second 7074001' } })
    expect(screen.queryByRole('button', { name: 'K1ABC' })).toBeNull()
    expect(screen.getByRole('button', { name: 'W9XYZ' })).toBeTruthy()
  })
  it('does not select a retained row after source revocation even before repaint', () => {
    let live = true
    const data = { ...source(), current: () => live }
    render(<SpotsPanel external={data} selectedCall={null} />)
    const call = screen.getByRole('button', { name: 'K1ABC' })
    live = false
    fireEvent.click(call)
    expect(data.select).not.toHaveBeenCalled()
  })
  it('native positive control keeps original mount services, row Work and Spot dialog', async () => {
    const native: SpotRow = { call: 'N1NATIVE', entity: 'United States', zone: 5, band: '20m',
      freqMhz: 14.025, mode: 'CW', spotter: 'W3LPL', corroborators: [], ageSecs: 10, comment: 'native report', licensed: true }
    const select = vi.fn(), work = vi.fn()
    const { container } = render(<SpotsPanel spots={[native]} bandPlan={[]} selectedCall={null} onSelect={select} onWork={work} />)
    await act(async () => {})
    expect(api.setPounceSpotterFilter).toHaveBeenCalledExactlyOnceWith([], [])
    expect(api.getDxccEntityLocations).toHaveBeenCalledOnce()
    expect(screen.getByText('Spotter')).toBeTruthy()
    expect(screen.getByText('Comment')).toBeTruthy()
    expect(screen.getByText('W3LPL')).toBeTruthy()
    const rendered = rowsOf(container)[0]
    fireEvent.click(rendered)
    expect(select).toHaveBeenCalledExactlyOnceWith(native.call)
    expect(work).toHaveBeenCalledExactlyOnceWith(native)
    fireEvent.click(within(rendered).getByRole('button', { name: /Spot N1NATIVE/i }))
    expect(screen.getByRole('dialog')).toBeTruthy()
    expect(api.postSpot).not.toHaveBeenCalled() // Opening the review is not posting.
  })
})
