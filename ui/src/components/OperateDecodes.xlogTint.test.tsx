// @vitest-environment jsdom
// A companion row takes the colour xLog's band map gives the station, so the stations worth
// working stand out as they do in xLog's cluster; a station xLog has already worked does not.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, render } from '@testing-library/react'
import { OperateDecodes } from './OperateDecodes'
import type { ExternalDecodeEntry } from '../decodeHistory'
import { XLOG_STATUS_COLORS } from '../xlog/xlogStatus'
vi.mock('../api', () => ({ openQrzPage: vi.fn() }))
beforeEach(() => localStorage.clear())
afterEach(cleanup)
function row(id: string, callStatus: string, dxccStatus: string, extra: Partial<ExternalDecodeEntry> = {}): ExternalDecodeEntry {
  return { id, slot: 4, at: 60_000, from: 'K1ABC', snr: -10, dtSec: .1, freqHz: 1200,
    message: 'CQ K1ABC FN42', isCq: true, directedToMe: false, worked: null, tier: 'FT8', rv: null,
    external: { periodStartMs: 60_000, status: 'needed', score: 50, reasons: [], reason: null, callStatus, dxccStatus },
    ...extra }
}
function rendered(entries: ExternalDecodeEntry[]) {
  const { container } = render(<OperateDecodes decodes={[]} slot={4} rxOffsetHz={1500} band="20m" tier="FT8"
    harqRescues={null} external={{ contextKey: 'owner-receiver', entries }} onCall={vi.fn()} />)
  return (id: string) => container.querySelector<HTMLElement>(`.decode-row[data-pin-key="${id}"]`)!
}
describe("xLog's classification on a companion row", () => {
  it('tints a needed entity in its DS colour, ahead of a needed call', () => {
    const get = rendered([row('atno', 'New Call', 'ATNO'), row('nb-w', 'New Call', 'W/New Band')])
    expect(get('atno').classList.contains('xlog-need')).toBe(true)
    expect(get('atno').style.getPropertyValue('--xlog-need-color')).toBe(XLOG_STATUS_COLORS.ATNO)
    expect(get('nb-w').style.getPropertyValue('--xlog-need-color')).toBe(XLOG_STATUS_COLORS['W/New Band'])
  })
  it('tints a needed call when its entity is not needed', () => {
    const get = rendered([row('nc', 'New Call', 'Worked')])
    expect(get('nc').classList.contains('xlog-need')).toBe(true)
    expect(get('nc').style.getPropertyValue('--xlog-need-color')).toBe(XLOG_STATUS_COLORS['New Call'])
  })
  it('leaves a worked or confirmed station, and an unclassified one, with its ordinary look', () => {
    const get = rendered([row('w', 'Worked', 'Confirmed'), row('u', 'Unknown', 'Unknown')])
    for (const id of ['w', 'u']) {
      expect(get(id).classList.contains('xlog-need')).toBe(false)
      expect(get(id).classList.contains('cq')).toBe(true)
    }
  })
  it('never outranks a message directed to me', () => {
    const get = rendered([row('to-me', 'New Call', 'ATNO', { isCq: false, directedToMe: true })])
    expect(get('to-me').classList.contains('directed')).toBe(true)
    expect(get('to-me').classList.contains('xlog-need')).toBe(false)
  })
})
