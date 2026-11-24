// @vitest-environment jsdom
// OM0RX-xNexus: the autopilot control, the Settings ▸ Autopilot section and the need feed.
import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import { AutopilotControl, PRESENCE_PROMPT_SECS } from './AutopilotControl'
import {
  SettingsAutomation,
  automationOf,
  clampInt,
  DEFAULT_AUTOMATION,
  DEFAULT_PRIORITY,
  moveRule,
  ruleOrder,
} from './SettingsAutomation'
import { autopilotFeedFromNeeds } from '../features/autopilotFeed'
import type { AutomationState, NeedAlert, Settings } from '../types'

const state = (over: Partial<AutomationState> = {}): AutomationState => ({
  answerCallers: false,
  callers: [],
  chain: 0,
  autopilotEnabled: true,
  autopilotOn: false,
  status: 'off',
  reason: '',
  presenceRemainingSecs: 0,
  presenceMinutes: 10,
  qsosLastHour: 0,
  log: [],
  ...over,
})

describe('AutopilotControl', () => {
  it('draws nothing while Settings does not allow the autopilot', () => {
    const { container } = render(
      <AutopilotControl automation={state({ autopilotEnabled: false })} onSetAutopilot={vi.fn()} onConfirmPresence={vi.fn()} />,
    )
    expect(container.firstChild).toBeNull()
  })

  it('turns the autopilot on and off', () => {
    const set = vi.fn()
    const { rerender } = render(<AutopilotControl automation={state()} onSetAutopilot={set} onConfirmPresence={vi.fn()} />)
    fireEvent.click(screen.getByTestId('autopilot-toggle'))
    expect(set).toHaveBeenCalledWith(true)
    rerender(
      <AutopilotControl
        automation={state({ autopilotOn: true, status: 'listening', presenceRemainingSecs: 500 })}
        onSetAutopilot={set}
        onConfirmPresence={vi.fn()}
      />,
    )
    fireEvent.click(screen.getByTestId('autopilot-toggle'))
    expect(set).toHaveBeenLastCalledWith(false)
  })

  it('opens the presence prompt in the last minute and confirms', () => {
    const confirm = vi.fn()
    const { rerender } = render(
      <AutopilotControl
        automation={state({ autopilotOn: true, status: 'listening', presenceRemainingSecs: PRESENCE_PROMPT_SECS + 1 })}
        onSetAutopilot={vi.fn()}
        onConfirmPresence={confirm}
      />,
    )
    expect(screen.queryByTestId('autopilot-prompt-confirm')).toBeNull()
    rerender(
      <AutopilotControl
        automation={state({ autopilotOn: true, status: 'listening', presenceRemainingSecs: 30 })}
        onSetAutopilot={vi.fn()}
        onConfirmPresence={confirm}
      />,
    )
    fireEvent.click(screen.getByTestId('autopilot-prompt-confirm'))
    expect(confirm).toHaveBeenCalledTimes(1)
  })

  it('shows stations calling you while Answer callers is on', () => {
    render(
      <AutopilotControl
        automation={state({ autopilotEnabled: false, answerCallers: true, callers: ['K1ABC', 'W1AW'] })}
        onSetAutopilot={vi.fn()}
        onConfirmPresence={vi.fn()}
      />,
    )
    expect(screen.getByText(/K1ABC W1AW/)).toBeTruthy()
  })
})

describe('SettingsAutomation', () => {
  it('fills missing fields from the defaults', () => {
    expect(automationOf({} as Settings)).toEqual(DEFAULT_AUTOMATION)
    const a = automationOf({ automation: { presenceCheckMin: 30 } } as unknown as Settings)
    expect(a.presenceCheckMin).toBe(30)
    expect(a.tiers.atno).toBe(true)
  })

  it('holds the presence interval to 2–999 minutes, default 10', () => {
    expect(DEFAULT_AUTOMATION.presenceCheckMin).toBe(10)
    expect(clampInt('1', 2, 999, 10)).toBe(2)
    expect(clampInt('5000', 2, 999, 10)).toBe(999)
    expect(clampInt('', 2, 999, 10)).toBe(10)
    expect(clampInt('abc', 2, 999, 10)).toBe(10)
    expect(clampInt('45', 2, 999, 10)).toBe(45)
  })

  it('makes a stored rule order whole, as the engine does', () => {
    expect(ruleOrder(undefined)).toEqual(DEFAULT_PRIORITY)
    // An older file without Most Wanted and Grid: both come in at their default places, and the
    // operator's own order of the rest is kept.
    expect(
      ruleOrder(['dxpedition', 'atno', 'dxccUnconfirmed', 'dxccNewSlot', 'usState', 'reference', 'newCall']),
    ).toEqual([
      'dxpedition', 'atno', 'mostWanted', 'dxccUnconfirmed', 'dxccNewSlot', 'usState', 'reference', 'grid', 'newCall',
    ])
    // Unknown names and duplicates go; a station calling you is never in the list.
    const order = ruleOrder(['grid', 'grid', 'callingMe', 'fromTheFuture', 'atno'])
    expect(order.slice(0, 2)).toEqual(['grid', 'atno'])
    expect(order).toHaveLength(DEFAULT_PRIORITY.length)
    expect(order).not.toContain('callingMe')
  })

  it('moves a rule one place, and not past either end', () => {
    const o = DEFAULT_PRIORITY
    expect(moveRule(o, 1, -1).slice(0, 2)).toEqual(['mostWanted', 'atno'])
    expect(moveRule(o, 0, -1)).toBe(o)
    expect(moveRule(o, o.length - 1, 1)).toBe(o)
  })

  it('reorders and switches rules from the list', () => {
    const change = vi.fn()
    render(<SettingsAutomation form={{} as Settings} onChange={change} />)
    const rows = () =>
      [...screen.getByTestId('automation-rules').querySelectorAll('li')].map((li) =>
        li.getAttribute('data-testid'),
      )
    expect(rows()[0]).toBe('automation-rule-atno')
    expect((screen.getByTestId('automation-rule-atno-up') as HTMLButtonElement).disabled).toBe(true)
    fireEvent.click(screen.getByTestId('automation-rule-grid-up'))
    const moved = change.mock.lastCall![0].priority as string[]
    expect(moved.indexOf('grid')).toBe(DEFAULT_PRIORITY.indexOf('grid') - 1)
    fireEvent.click(screen.getByTestId('automation-rule-newCall-on'))
    expect(change).toHaveBeenLastCalledWith(
      expect.objectContaining({ tiers: expect.objectContaining({ newCall: false }) }),
    )
    fireEvent.click(screen.getByTestId('automation-outside-eu'))
    expect(change).toHaveBeenLastCalledWith(expect.objectContaining({ preferOutsideEu: false }))
    fireEvent.click(screen.getByTestId('automation-clear-tx'))
    expect(change).toHaveBeenLastCalledWith(expect.objectContaining({ clearTxOffset: false }))
  })

  it('switches a continent off and on, and sets the call limit', () => {
    const change = vi.fn()
    const { rerender } = render(<SettingsAutomation form={{} as Settings} onChange={change} />)
    expect((screen.getByTestId('automation-continent-EU') as HTMLInputElement).checked).toBe(true)
    fireEvent.click(screen.getByTestId('automation-continent-EU'))
    expect(change).toHaveBeenLastCalledWith(expect.objectContaining({ skipContinents: ['EU'] }))
    rerender(
      <SettingsAutomation
        form={{ automation: { skipContinents: ['EU', 'AF'] } } as unknown as Settings}
        onChange={change}
      />,
    )
    expect((screen.getByTestId('automation-continent-EU') as HTMLInputElement).checked).toBe(false)
    fireEvent.click(screen.getByTestId('automation-continent-EU'))
    expect(change).toHaveBeenLastCalledWith(expect.objectContaining({ skipContinents: ['AF'] }))
    fireEvent.change(screen.getByTestId('automation-max-calls'), { target: { value: '6' } })
    expect(change).toHaveBeenLastCalledWith(expect.objectContaining({ targetMaxCalls: 6 }))
  })

  it('shows a saved order as saved', () => {
    const priority = ['grid', ...DEFAULT_PRIORITY.filter((r) => r !== 'grid')]
    render(<SettingsAutomation form={{ automation: { priority } } as unknown as Settings} onChange={vi.fn()} />)
    const first = screen.getByTestId('automation-rules').querySelector('li')
    expect(first?.getAttribute('data-testid')).toBe('automation-rule-grid')
  })

  it('edits the autopilot switch and the presence interval', () => {
    const change = vi.fn()
    render(<SettingsAutomation form={{} as Settings} onChange={change} />)
    fireEvent.click(screen.getByTestId('automation-autopilot-enabled'))
    expect(change).toHaveBeenLastCalledWith(expect.objectContaining({ autopilotEnabled: true }))
    fireEvent.change(screen.getByTestId('automation-presence-min'), { target: { value: '25' } })
    expect(change).toHaveBeenLastCalledWith(expect.objectContaining({ presenceCheckMin: 25 }))
    fireEvent.change(screen.getByTestId('automation-presence-min'), { target: { value: '1000' } })
    expect(change).toHaveBeenLastCalledWith(expect.objectContaining({ presenceCheckMin: 999 }))
  })
})

describe('autopilotFeedFromNeeds', () => {
  const alert = (over: Partial<NeedAlert>): NeedAlert =>
    ({ call: 'k1abc', entity: 'X', band: '20m', zone: 5, tags: [], priority: 1, headline: '', mode: 'Digital', freqMhz: null, ...over }) as NeedAlert

  it('maps the tags the autopilot ranks by', () => {
    const [e] = autopilotFeedFromNeeds([alert({ tags: ['NewEntity', 'Dxped'], exactMode: 'FT8', freqMhz: 14.074 })])
    expect(e).toMatchObject({ call: 'K1ABC', band: '20m', mode: 'FT8', newEntity: true, dxpedition: true, newSlot: false })
  })

  it('keeps the row that says the most and drops unlicensed bands', () => {
    const out = autopilotFeedFromNeeds([
      alert({ tags: ['Pota'] }),
      alert({ tags: ['NewBand'] }),
      alert({ call: 'W1AW', tags: ['NewEntity'], licensed: false }),
    ])
    expect(out).toHaveLength(1)
    expect(out[0].newSlot).toBe(true)
  })
})
