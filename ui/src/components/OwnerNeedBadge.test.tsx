// @vitest-environment jsdom
import { render, screen, cleanup } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { OwnerNeedBadge, StatusCell } from './OwnerNeedBadge'
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

describe("CS and DS from the operator's own log (full app)", () => {
  it("draws each status in xLog's short code and colour", () => {
    render(<><StatusCell kind="CS" status="New Band+Mode" /><StatusCell kind="DS" status="C/New Mode" /></>)
    const cs = screen.getByLabelText('CS New Band+Mode')
    expect(cs.textContent).toBe('NB+M')
    expect(cs.style.color).toBeTruthy()
    const ds = screen.getByLabelText('DS C/New Mode')
    expect(ds.textContent).toBe('NM/C')
    expect(ds.getAttribute('data-xlog-status')).toBe('C/New Mode')
  })
  it('leaves an absent status empty, never "?" or unworked', () => {
    const { container } = render(<><StatusCell kind="CS" status={null} /><StatusCell kind="DS" /></>)
    const cells = container.querySelectorAll('.decode-xlog')
    expect(cells.length).toBe(2)
    cells.forEach(c => expect(c.textContent).toBe(''))
    expect(screen.queryByLabelText(/^CS /)).toBeNull()
  })
})
