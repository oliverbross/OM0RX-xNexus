// SPDX-License-Identifier: GPL-3.0-only
import { describe, expect, it } from 'vitest'
import type { NeedAlert } from '../types'
import { autopilotFeedFromNeeds } from './autopilotFeed'
import { parseStates } from '../components/SettingsAutomation'

const alert = (over: Partial<NeedAlert>): NeedAlert =>
  ({ call: 'W1VT', entity: 'United States', band: '20m', zone: 5, tags: [], priority: 0, headline: '', mode: 'Digital', freqMhz: null, ...over }) as NeedAlert

describe('autopilot feed', () => {
  it('carries a needed state and the state it is in', () => {
    const [e] = autopilotFeedFromNeeds([alert({ tags: ['NewState'], state: 'VT' })])
    expect(e.newState).toBe(true)
    expect(e.state).toBe('VT')
  })

  it('carries a DXpedition with its own frequency', () => {
    const [e] = autopilotFeedFromNeeds([
      alert({ call: '3y0k', band: '40m', tags: ['Dxped', 'NewBand'], exactMode: 'FT8', freqMhz: 7.056 }),
    ])
    expect(e).toMatchObject({ call: '3Y0K', dxpedition: true, newSlot: true, freqMhz: 7.056, mode: 'FT8' })
  })

  it('keeps every mode a station is spotted in on a band', () => {
    const rows = autopilotFeedFromNeeds([
      alert({ call: 'VK9XY', band: '15m', tags: ['Dxped', 'NewBand'], exactMode: 'FT8', freqMhz: 21.074 }),
      alert({ call: 'VK9XY', band: '15m', tags: ['Dxped', 'NewBand'], exactMode: 'FT4', freqMhz: 21.14 }),
      alert({ call: 'VK9XY', band: '15m', tags: ['Dxped'], exactMode: 'FT4', freqMhz: 21.14 }),
    ])
    expect(rows.map((r) => r.mode)).toEqual(['FT8', 'FT4'])
  })

  it('counts a new mode alone as no slot, except for a DXpedition', () => {
    const slot = (tags: NeedAlert['tags']) =>
      autopilotFeedFromNeeds([alert({ call: 'OM8AKP', tags, exactMode: 'FT4' })])[0].newSlot
    expect(slot(['NewMode'])).toBe(false)
    expect(slot(['NewBand'])).toBe(true)
    expect(slot(['Dxped', 'NewMode'])).toBe(true)
  })
})

describe('wanted states', () => {
  it('parses a typed list into clean codes', () => {
    expect(parseStates(' vt, ND  me;vt ')).toEqual(['VT', 'ND', 'ME'])
    expect(parseStates('')).toEqual([])
  })
})
