// @vitest-environment jsdom
import { render, screen, cleanup } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { OwnerNeedBadge } from './OwnerNeedBadge'
import { XLOG_STATUS_COLORS } from '../xlog/xlogStatus'
afterEach(cleanup)
const needColor = (el: HTMLElement) => el.style.getPropertyValue('--need-color')
describe('owner call and entity facts', () => {
  it('shows new-band call history independently of an unresolved country', () => {
    render(<OwnerNeedBadge facts={{ periodStartMs: 0, status: 'unknown', score: null,
      reasons: [], reason: 'country_unknown', callStatus: 'New Band', dxccStatus: 'Unknown', band: '30m', classificationMode: 'DIGI' }} />)
    const cs = screen.getByLabelText('CS New Band')
    expect(cs.textContent).toBe('NB')
    expect(cs.title).toContain('30m DIGI')
    expect(cs.title).toContain('DS Unknown')
    expect(screen.getByLabelText('DS Unknown').textContent).toBe('?')
  })
  it('shows an ATNO separately from a callsign already worked', () => {
    render(<OwnerNeedBadge facts={{ periodStartMs: 0, status: 'needed', score: 100,
      reasons: [], reason: null, callStatus: 'Worked', dxccStatus: 'ATNO' }} />)
    expect(screen.getByLabelText('CS Worked').textContent).toBe('W')
    const ds = screen.getByLabelText('DS ATNO')
    expect(ds.textContent).toBe('ATNO')
    expect(ds.title).toContain('score 100')
  })
  it('keeps missing facts unknown instead of implying an unworked station', () => {
    render(<OwnerNeedBadge facts={{ periodStartMs: 0, status: 'unknown', score: null, reasons: [], reason: 'native_stale' }} />)
    expect(screen.getByText('Unknown')).toBeTruthy()
    expect(screen.queryByText('NC')).toBeNull()
    expect(screen.queryByLabelText(/^CS /)).toBeNull()
  })
  it("uses xLog's cluster codes and colours, telling a confirmed entity from a worked one", () => {
    render(<OwnerNeedBadge facts={{ periodStartMs: 0, status: 'needed', score: 40,
      reasons: [], reason: null, callStatus: 'New Call', dxccStatus: 'C/New Band' }} />)
    const cs = screen.getByLabelText('CS New Call')
    expect(cs.textContent).toBe('NC')
    expect(needColor(cs)).toBe(XLOG_STATUS_COLORS['New Call'])
    const ds = screen.getByLabelText('DS C/New Band')
    expect(ds.textContent).toBe('NB/C')
    expect(needColor(ds)).toBe(XLOG_STATUS_COLORS['C/New Band'])
    cleanup()
    render(<OwnerNeedBadge facts={{ periodStartMs: 0, status: 'needed', score: 40,
      reasons: [], reason: null, callStatus: 'New Call', dxccStatus: 'W/New Band' }} />)
    const worked = screen.getByLabelText('DS W/New Band')
    expect(worked.textContent).toBe('NB/W')
    expect(needColor(worked)).toBe(XLOG_STATUS_COLORS['W/New Band'])
    expect(needColor(worked)).not.toBe(XLOG_STATUS_COLORS['C/New Band'])
  })
})
