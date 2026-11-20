// @vitest-environment jsdom
// OM0RX-xNexus: the autopilot control, the Settings ▸ Automation section and the need feed.
import { describe, expect, it, vi } from 'vitest'
import { fireEvent, render, screen } from '@testing-library/react'
import { AutopilotControl, PRESENCE_PROMPT_SECS } from './AutopilotControl'
import { SettingsAutomation, automationOf, clampInt, DEFAULT_AUTOMATION } from './SettingsAutomation'
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
