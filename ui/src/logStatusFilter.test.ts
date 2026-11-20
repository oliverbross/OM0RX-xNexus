// OM0RX-xNexus: the CS / DS status filter's rules.
import { describe, expect, it } from 'vitest'
import {
  LOG_STATUS_PRESETS,
  SHOW_ALL,
  hiddenCount,
  matchingPreset,
  parseLogStatusFilter,
  passesLogStatus,
  toggleStatus,
} from './logStatusFilter'

describe('passesLogStatus', () => {
  it('shows everything when nothing is hidden', () => {
    expect(passesLogStatus(SHOW_ALL, 'Worked', 'Confirmed')).toBe(true)
    expect(passesLogStatus(SHOW_ALL, null, undefined)).toBe(true)
  })

  it('hides a row whose call status is unticked, and keeps the rest', () => {
    const f = LOG_STATUS_PRESETS.newCalls
    expect(passesLogStatus(f, 'Worked', 'C/New Band')).toBe(false)
    expect(passesLogStatus(f, 'Confirmed', 'ATNO')).toBe(false)
    expect(passesLogStatus(f, 'New Call', 'Confirmed')).toBe(true)
    expect(passesLogStatus(f, 'New Band', 'Worked')).toBe(true)
  })

  it('ANDs the two boxes', () => {
    const f = { hideCs: ['Worked'], hideDs: ['Confirmed'] }
    expect(passesLogStatus(f, 'New Call', 'Confirmed')).toBe(false)
    expect(passesLogStatus(f, 'Worked', 'ATNO')).toBe(false)
    expect(passesLogStatus(f, 'New Call', 'ATNO')).toBe(true)
  })

  it('treats an absent status as Unknown', () => {
    const f = LOG_STATUS_PRESETS.newDxcc
    expect(passesLogStatus(f, 'New Call', null)).toBe(false)
    expect(passesLogStatus({ hideCs: ['Unknown'], hideDs: [] }, undefined, 'ATNO')).toBe(false)
    expect(passesLogStatus({ hideCs: ['Unknown'], hideDs: [] }, 'Worked', 'ATNO')).toBe(true)
  })
})

describe('parseLogStatusFilter', () => {
  it('drops anything unreadable or unknown, which can only show more', () => {
    expect(parseLogStatusFilter(null)).toEqual(SHOW_ALL)
    expect(parseLogStatusFilter('not json')).toEqual(SHOW_ALL)
    expect(parseLogStatusFilter('null')).toEqual(SHOW_ALL)
    expect(parseLogStatusFilter('{"hideCs":["Worked","Bogus",3,"Worked"],"hideDs":"ATNO"}')).toEqual({
      hideCs: ['Worked'],
      hideDs: [],
    })
  })

  it('round-trips a stored value', () => {
    const f = { hideCs: ['Worked', 'Confirmed'], hideDs: ['Unknown'] }
    expect(parseLogStatusFilter(JSON.stringify(f))).toEqual(f)
  })
})

describe('toggleStatus / presets', () => {
  it('flips one status in and out', () => {
    const a = toggleStatus(SHOW_ALL, 'cs', 'Worked')
    expect(a.hideCs).toEqual(['Worked'])
    expect(hiddenCount(a)).toBe(1)
    expect(toggleStatus(a, 'cs', 'Worked').hideCs).toEqual([])
    expect(toggleStatus(a, 'ds', 'ATNO')).toEqual({ hideCs: ['Worked'], hideDs: ['ATNO'] })
  })

  it('recognises a preset whatever the order', () => {
    expect(matchingPreset(SHOW_ALL)).toBe('all')
    expect(matchingPreset({ hideCs: ['Confirmed', 'Worked'], hideDs: [] })).toBe('newCalls')
    expect(matchingPreset({ hideCs: ['Worked'], hideDs: [] })).toBeNull()
  })
})
