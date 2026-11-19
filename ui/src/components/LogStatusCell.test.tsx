// @vitest-environment jsdom
import { describe, expect, it, afterEach } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'
import { StatusCell, logStatusCode, logStatusColor, LOG_STATUS_COLORS } from './LogStatusCell'

afterEach(cleanup)

describe("CS and DS from the operator's own log", () => {
  it("draws each status in xLog's short code and colour", () => {
    render(<><StatusCell kind="CS" status="New Band+Mode" /><StatusCell kind="DS" status="C/New Mode" /></>)
    const cs = screen.getByLabelText('CS New Band+Mode')
    expect(cs.textContent).toBe('NB+M')
    expect(cs.style.color).toBeTruthy()
    const ds = screen.getByLabelText('DS C/New Mode')
    expect(ds.textContent).toBe('NM/C')
    expect(ds.getAttribute('data-log-status')).toBe('C/New Mode')
  })

  it('leaves an absent status empty, never "?" or unworked', () => {
    const { container } = render(<><StatusCell kind="CS" status={null} /><StatusCell kind="DS" /></>)
    const cells = container.querySelectorAll('.log-status')
    expect(cells.length).toBe(2)
    cells.forEach(c => expect(c.textContent).toBe(''))
    expect(screen.queryByLabelText(/^CS /)).toBeNull()
  })

  it('maps every status to its code and an unknown one to the Unknown colour', () => {
    expect(logStatusCode('ATNO')).toBe('ATNO')
    expect(logStatusCode('W/New Band+Mode')).toBe('NB+NM/W')
    expect(logStatusCode('Confirmed')).toBe('C')
    expect(logStatusCode(undefined)).toBe('?')
    expect(logStatusColor('Something else')).toBe(LOG_STATUS_COLORS.Unknown)
  })
})
