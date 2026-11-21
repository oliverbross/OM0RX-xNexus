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
})

describe('wanted states', () => {
  it('parses a typed list into clean codes', () => {
    expect(parseStates(' vt, ND  me;vt ')).toEqual(['VT', 'ND', 'ME'])
    expect(parseStates('')).toEqual([])
  })
})
