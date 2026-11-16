// @vitest-environment jsdom
import { render, screen, cleanup } from '@testing-library/react'
import { afterEach, describe, expect, it } from 'vitest'
import { OwnerNeedBadge } from './OwnerNeedBadge'
afterEach(cleanup)
describe('owner call and entity facts', () => {
  it('shows new-band call history independently of an unresolved country', () => {
    render(<OwnerNeedBadge facts={{ periodStartMs: 0, status: 'unknown', score: null,
      reasons: [], reason: 'country_unknown', callStatus: 'New Band', dxccStatus: 'Unknown', band: '30m', classificationMode: 'DIGI' }} />)
    expect(screen.getByText('New Band').title).toContain('30m DIGI')
    expect(screen.getByText('New Band').title).toContain('DXCC Unknown')
  })
  it('shows an ATNO separately from a callsign already worked', () => {
    render(<OwnerNeedBadge facts={{ periodStartMs: 0, status: 'needed', score: 100,
      reasons: [], reason: null, callStatus: 'Worked', dxccStatus: 'ATNO' }} />)
    expect(screen.getByText('Worked · 100')).toBeTruthy()
    expect(screen.getByText('DXCC ATNO')).toBeTruthy()
  })
  it('keeps missing facts unknown instead of implying an unworked station', () => {
    render(<OwnerNeedBadge facts={{ periodStartMs: 0, status: 'unknown', score: null, reasons: [], reason: 'native_stale' }} />)
    expect(screen.getByText('Unknown')).toBeTruthy()
    expect(screen.queryByText('New Call')).toBeNull()
  })
})
