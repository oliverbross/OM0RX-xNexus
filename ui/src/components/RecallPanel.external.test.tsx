// @vitest-environment jsdom
// SPDX-License-Identifier: GPL-3.0-only
import { afterEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/react'
import { RecallPanel, type ExternalRecallRead } from './RecallPanel'
import { OperateServices } from './operateServices'
import { callHistory } from '../features/callHistory'
import type { LoggedQso } from '../types'

afterEach(cleanup)
const native = callHistory([{ id: 'native-only', call: 'K1ABC', grid: null, band: '80m', freqMhz: 3.573,
  mode: 'FT8', rstSent: '-10', rstRcvd: '-11', whenUnix: 10000, comment: 'Native-only comment' } as LoggedQso], 'K1ABC', '80m')
const adapter = { handlers: {}, actions: new Set<never>(), onDeniedAction: vi.fn() }
function external(): Extract<ExternalRecallRead, { state: 'current' }> {
  return { state: 'current', callsign: 'K1ABC', band: '20m',
    notice: 'Showing 2 of 40 xLog contacts. 3 of 40 confirmed by paper QSL or LoTW in xLog.',
    confirmationTitle: '3 of 40 confirmed by paper QSL or LoTW in xLog.', unavailableMessage: 'Selected xLog history unavailable.',
    history: { count: 40, confirmedCount: 3, dupeThisBand: true, qsos: [
      { id: 'owner-complete', whenUnix: 20000, band: '20m', mode: 'FT4', rstSent: '-08', rstRcvd: '-12', comment: 'Owner complete comment', canOpen: true },
      { id: 'owner-view-only', whenUnix: 19000, band: '30m', mode: 'FT8', rstSent: '-09', rstRcvd: '-13', comment: 'Owner partial comment', canOpen: false },
    ] } }
}
function pane(read: ExternalRecallRead, onOpenLog = vi.fn(), call = 'K1ABC', band = '20m') {
  return <OperateServices adapter={adapter}><RecallPanel call={call} band={band} hist={native}
    externalHistory={read} onOpenLog={onOpenLog} latestNote="stale native private note" /></OperateServices>
}

describe('original recall pane with xLog history', () => {
  it('renders real narrow rows and full-count policy without native history substitution', () => {
    const { container } = render(pane(external()))
    expect(container.querySelectorAll('.recall-log-row')).toHaveLength(2)
    expect(container.querySelector('.recall-log-count')!.textContent).toBe('40')
    expect(container.querySelector('.recall-badge.ok')!.getAttribute('title')).toBe(external().confirmationTitle)
    expect(container.textContent).toContain('paper QSL or LoTW')
    expect(container.textContent).toContain('FT4')
    expect(container.textContent).not.toContain('Native-only comment')
    expect(container.textContent).not.toContain('stale native private note')
  })
  it('removes rows and badges immediately on unavailable/loading states', () => {
    const shown = render(pane(external()))
    for (const state of ['unavailable', 'loading'] as const) {
      shown.rerender(pane({ state, message: `${state} owner history` }))
      expect(shown.container.querySelectorAll('.recall-log-row')).toHaveLength(0)
      expect(shown.container.querySelector('.recall-badge.ok')).toBeNull()
      expect(shown.container.querySelector('.recall-badge.dupe')).toBeNull()
      expect(shown.container.textContent).toContain(`${state} owner history`)
      expect(shown.container.textContent).not.toContain('Native-only comment')
      expect(shown.container.textContent).not.toContain('stale native private note')
    }
  })
  it('does not display the previous selected callsign history', () => {
    const shown = render(pane(external(), vi.fn(), 'G1XYZ'))
    expect(shown.container.querySelectorAll('.recall-log-row')).toHaveLength(0)
    expect(shown.container.textContent).toContain('Selected xLog history unavailable.')
  })
  it('does not carry a prior-band dupe badge to a new band', () => {
    const shown = render(pane(external()))
    expect(shown.container.querySelector('.recall-badge.dupe')).not.toBeNull()
    shown.rerender(pane(external(), vi.fn(), 'K1ABC', '40m'))
    expect(shown.container.querySelector('.recall-badge.dupe')).toBeNull()
    expect(shown.container.querySelectorAll('.recall-log-row')).toHaveLength(2)
  })
  it('opens only complete editable owner contacts and preserves their IDs', () => {
    const open = vi.fn(), { container } = render(pane(external(), open))
    const rows = [...container.querySelectorAll<HTMLElement>('.recall-log-row')]
    fireEvent.click(rows[1])
    expect(open).not.toHaveBeenCalled()
    expect(rows[1].hasAttribute('tabindex')).toBe(false)
    expect(rows[1].classList.contains('clickable')).toBe(false)
    fireEvent.click(rows[0].querySelector('.recall-log-bm')!)
    expect(open.mock.calls).toEqual([['K1ABC', 'owner-complete']])
  })
  it('keeps keyboard navigation on editable contacts when a read-only row is newest', () => {
    const read = external(); read.history.qsos[1].whenUnix = 21000
    const open = vi.fn(), { container } = render(pane(read, open))
    const rows = [...container.querySelectorAll<HTMLElement>('.recall-log-row')]
    expect(rows[0].hasAttribute('tabindex')).toBe(false)
    expect(rows[1].tabIndex).toBe(0)
    fireEvent.focus(rows[1]); fireEvent.keyDown(rows[1], { key: 'Enter' })
    expect(open.mock.calls).toEqual([['K1ABC', 'owner-complete']])
  })
  it('keeps the original comment toggle usable on a read-only owner row', () => {
    const open = vi.fn(), { container } = render(pane(external(), open))
    const row = container.querySelectorAll('.recall-log-row')[1]
    fireEvent.click(row.querySelector('button')!)
    expect(row.querySelector('button')!.getAttribute('aria-expanded')).toBe('true')
    expect(open).not.toHaveBeenCalled()
  })
})
