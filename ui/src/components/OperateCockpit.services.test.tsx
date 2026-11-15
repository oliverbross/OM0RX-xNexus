// @vitest-environment jsdom
// Source-prepared only: these regressions must run before this seam is committed.
// Synthetic snapshots exercise the existing cockpit, never a production owner adapter.
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import * as native from '../api'
import { APP_SNAPSHOT } from '../appCockpits.testkit'
import type { AppSnapshot } from '../types'
import { OPERATE_PANELS, panelStateIn, type OperatePanelId, type PanelLayoutApi } from '../features/panelState'
import { OperateCockpit } from './OperateCockpit'
import { bindOperateServices, guardOperateAction, type OperateServiceAdapter } from './operateServices'

vi.mock('../api', async importOriginal => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...Object.fromEntries(Object.entries(actual).map(([key, value]) => [key,
      typeof value === 'function' ? vi.fn(async () => null) : value])),
    getSettings: vi.fn(async () => ({})),
    askLog: vi.fn(async () => { throw new Error('No log in this fixture') }),
  }
})
// Avoid a canvas in jsdom while exercising the REAL cockpit's callback routing.
vi.mock('./Waterfall', () => ({ Waterfall: ({ onTune }: { onTune: (hz: number, target: 'rx' | 'tx') => void }) => <div>
  <button onClick={() => onTune(1200, 'rx')}>Fixture RX marker</button>
  <button onClick={() => onTune(1500, 'tx')}>Fixture TX marker</button>
</div> }))

const snapshot = APP_SNAPSHOT as unknown as AppSnapshot
function setup(services?: OperateServiceAdapter, layoutMode: 'classic' | 'roster' = 'classic') {
  const noop = vi.fn()
  const onHaltTx = vi.fn()
  const onTune = vi.fn()
  const panels: PanelLayoutApi<OperatePanelId> = {
    layout: { v: 1, state: {}, share: {} },
    stateOf: id => panelStateIn(OPERATE_PANELS, { v: 1, state: {}, share: {} }, id),
    setPanelState: noop, shareOf: () => 1, setShare: noop, setShares: noop,
    undo: noop, canUndo: false, undoRemoves: [], reset: noop,
  }
  const view = render(<OperateCockpit snap={snapshot} theme="dark" tier="FT8" services={services}
    onTierChange={noop} bandPlan={[]} onSetFrequency={noop} onSourceChange={noop}
    onTune={onTune} onCall={noop} onSetTxLevel={noop} onSetMode={noop}
    onSetTxEven={noop} onSetTxCycleAuto={noop} onResend={noop} onFreetext={noop}
    onLog={noop} onOverrideTx={noop} onHaltTx={onHaltTx}
    roster={<div>Fixture roster</div>} needByCall={new Map()} selectedCall={null}
    onSelect={noop} layoutMode={layoutMode} onLayoutMode={noop} panels={panels} active />)
  return { ...view, onHaltTx, onTune }
}
const key = (key: string, code = key, altKey = false) => fireEvent.keyDown(window, { key, code, altKey })
afterEach(() => { cleanup(); vi.clearAllMocks() })

describe('original cockpit service boundary', () => {
  it.each(['classic', 'roster'] as const)('preserves upstream API and shortcuts by default in %s', async layout => {
    const { onHaltTx, container } = setup(undefined, layout)
    expect(container.querySelector('main.operate-cockpit')).not.toBeNull()
    key('F6')
    key('6', 'Digit6', true)
    key('Escape')
    await waitFor(() => expect(native.getSettings).toHaveBeenCalled())
    expect(native.redecode).toHaveBeenCalledTimes(1)
    expect(native.startCq).toHaveBeenCalledTimes(1)
    expect(onHaltTx).toHaveBeenCalledTimes(1)
  })

  it('refuses denied shortcut handlers even when the adapter supplies implementations', async () => {
    const redecode = vi.fn(async () => snapshot)
    const startCq = vi.fn(async () => snapshot)
    const onDeniedAction = vi.fn()
    const { onHaltTx, onTune } = setup({ handlers: { redecode, startCq }, actions: new Set(), onDeniedAction })
    key('F6'); key('6', 'Digit6', true); key('Escape')
    fireEvent.click(screen.getByText('Fixture RX marker'))
    fireEvent.click(screen.getByText('Fixture TX marker'))
    await waitFor(() => expect(onDeniedAction).toHaveBeenCalledWith('decoder'))
    expect(onDeniedAction).toHaveBeenCalledWith('transmit')
    expect(onDeniedAction).toHaveBeenCalledWith('stopTransmit')
    expect(onDeniedAction).toHaveBeenCalledWith('radio')
    expect(redecode).not.toHaveBeenCalled()
    expect(startCq).not.toHaveBeenCalled()
    expect(onHaltTx).not.toHaveBeenCalled()
    expect(onTune).not.toHaveBeenCalled()
    expect(native.redecode).not.toHaveBeenCalled()
    expect(native.startCq).not.toHaveBeenCalled()
    expect(native.getSettings).not.toHaveBeenCalled()
  })

  it('routes allowed receive work and emergency stop independently from denied transmit', async () => {
    const redecode = vi.fn(async () => snapshot)
    const onDeniedAction = vi.fn()
    const { onHaltTx, onTune } = setup({ handlers: { redecode },
      actions: new Set(['decoder', 'radio', 'stopTransmit']), onDeniedAction })
    key('F6'); key('6', 'Digit6', true); key('Escape')
    fireEvent.click(screen.getByText('Fixture RX marker'))
    fireEvent.click(screen.getByText('Fixture TX marker'))
    await waitFor(() => expect(redecode).toHaveBeenCalledTimes(1))
    expect(onHaltTx).toHaveBeenCalledTimes(1)
    expect(onTune).toHaveBeenCalledExactlyOnceWith(1200, 'rx')
    expect(onDeniedAction).toHaveBeenCalledWith('transmit')
    expect(native.redecode).not.toHaveBeenCalled()
    expect(native.startCq).not.toHaveBeenCalled()
  })

  it('does not turn a missing service into a native fallback or fabricated success', async () => {
    const services = bindOperateServices({ handlers: {}, actions: new Set(['decoder']), onDeniedAction: vi.fn() })
    expect(services.can('getSpectrumRow')).toBe(false)
    expect(services.can('redecode')).toBe(false)
    await expect(services.api.getSpectrumRow(false)).rejects.toThrow('does not provide getSpectrumRow')
    await expect(services.api.redecode()).rejects.toThrow('does not provide redecode')
    expect(native.getSpectrumRow).not.toHaveBeenCalled()
    expect(native.redecode).not.toHaveBeenCalled()
  })

  it('preserves exact handler arguments and result without synthesizing station fields', async () => {
    const redecode = vi.fn(async () => snapshot)
    const services = bindOperateServices({ handlers: { redecode }, actions: new Set(['decoder']), onDeniedAction: vi.fn() })
    expect(services.can('redecode')).toBe(true)
    expect(await services.api.redecode()).toBe(snapshot)
    expect(redecode).toHaveBeenCalledExactlyOnceWith()
    const callback = vi.fn()
    expect(guardOperateAction(undefined, 'transmit', callback)).toBe(callback)
  })
})
