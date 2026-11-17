import { describe, expect, it } from 'vitest'
import {
  XLOG_STATUS_COLORS, isXlogNeed, xlogOpportunityStatus, xlogStatusCode, xlogStatusColor,
} from './xlogStatus'

// Every status xLog's companion needs endpoint may send (xLog companion_needs.rs validation and
// SpotLogic.swift), CS first, then the DS-only ones.
const CALL = ['New Call', 'Confirmed', 'New Band+Mode', 'New Band', 'New Mode', 'Worked', 'Unknown']
const DXCC = ['ATNO', 'Confirmed', 'C/New Band+Mode', 'C/New Band', 'C/New Mode',
  'W/New Band+Mode', 'W/New Band', 'W/New Mode', 'Worked', 'Unknown']

describe("xLog's cluster status vocabulary", () => {
  it('gives every status xLog sends its own code and a colour', () => {
    for (const status of new Set([...CALL, ...DXCC])) {
      // A known status maps to a short code (ATNO is its own code), never echoed back raw.
      if (status !== 'ATNO') expect(xlogStatusCode(status)).not.toBe(status)
      expect(XLOG_STATUS_COLORS[status]).toMatch(/^#[0-9A-F]{6}$/)
    }
    expect(xlogStatusCode('New Band+Mode')).toBe('NB+M')
    expect(xlogStatusCode('C/New Band+Mode')).toBe('NB+NM/C')
    expect(xlogStatusCode('W/New Mode')).toBe('NM/W')
    expect(xlogStatusCode(null)).toBe('?')
  })
  it('falls back to the Unknown colour for anything unrecognised', () => {
    expect(xlogStatusColor('Something else')).toBe(XLOG_STATUS_COLORS.Unknown)
    expect(xlogStatusColor(undefined)).toBe(XLOG_STATUS_COLORS.Unknown)
  })
  it("follows the band map's order: entity need, then call need, then Confirmed, then Worked", () => {
    expect(xlogOpportunityStatus('New Call', 'ATNO')).toBe('ATNO')
    expect(xlogOpportunityStatus('New Call', 'W/New Mode')).toBe('W/New Mode')
    expect(xlogOpportunityStatus('New Band', 'Worked')).toBe('New Band')
    expect(xlogOpportunityStatus('Worked', 'Confirmed')).toBe('Confirmed')
    expect(xlogOpportunityStatus('Worked', 'Worked')).toBe('Worked')
    expect(xlogOpportunityStatus(null, null)).toBe('Unknown')
  })
  it('counts exactly the "New …" and ATNO statuses as needed', () => {
    for (const s of ['New Call', 'New Band', 'New Mode', 'New Band+Mode', 'ATNO',
      'C/New Band', 'C/New Mode', 'C/New Band+Mode', 'W/New Band', 'W/New Mode', 'W/New Band+Mode']) {
      expect(isXlogNeed(s)).toBe(true)
    }
    for (const s of ['Worked', 'Confirmed', 'Unknown', '', null]) expect(isXlogNeed(s)).toBe(false)
  })
})
