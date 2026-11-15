// @vitest-environment jsdom
// Source-only draft. Real original controls; synthetic data, no native station.
import { afterEach, describe, expect, it, vi } from 'vitest'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { useRef } from 'react'
import * as native from '../api'
import { APP_SNAPSHOT } from '../appCockpits.testkit'
import type { AppSnapshot, CallStats } from '../types'
import { useWheelTune } from '../useWheelTune'
import { useCallBearings } from '../features/callBearing'
import { useEntityCentroids } from '../features/entityCentroids'
import { CallStatsPane } from './CallStatsPane'
import { OperateServices, bindOperateServices, useOperateServices, type OperateServiceAdapter } from './operateServices'
import { TopBar } from './TopBar'
import { TuningStrip } from './TuningStrip'
import { RotorStrip } from './RotorStrip'
import { LiveLevelMeter } from './LiveMeters'

vi.mock('../api', async importOriginal => {
  const actual = await importOriginal<typeof import('../api')>()
  return {
    ...Object.fromEntries(Object.entries(actual).map(([key, value]) => [key,
      typeof value === 'function' ? vi.fn(async () => null) : value])),
    appVersion: vi.fn(async () => 'test-upstream'),
    getMeters: vi.fn(async () => ({ rxLevel: 0.5, smeterDb: -12, cwToneHz: null })),
  }
})
const snap = APP_SNAPSHOT as unknown as AppSnapshot
const denied = (): OperateServiceAdapter => ({ handlers: {}, actions: new Set(), onDeniedAction: vi.fn() })
afterEach(() => { cleanup(); vi.useRealTimers(); vi.clearAllMocks() })

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>(done => { resolve = done })
  return { promise, resolve }
}
function Wheel({ onSnap }: { onSnap: (value: AppSnapshot) => void }) {
  const ref = useRef<HTMLDivElement>(null)
  const tune = useWheelTune(ref, { dialMhz: 14.1, sideband: 'USB', enabled: true, stepHz: 100, onSnap })
  return <div ref={ref}><button onClick={() => tune(100)}>Wheel target</button></div>
}
function Header() {
  const noop = () => {}
  return <TopBar mycall={snap.mycall} mygrid={snap.mygrid} radio={snap.radio} link={snap.link}
    bandPlan={[]} tier="FT8" onOpenGuide={noop} onSetFrequency={noop} onSetTxEnabled={noop}
    onSetTune={noop} onHaltTx={noop} onSetTxEven={noop} onSetTxCycleAuto={noop}
    onSetHoldTxFreq={noop} onTierChange={noop} />
}

describe('original descendant services and visible capabilities', () => {
  it('keeps an inherited explicit adapter through a nested provider with no override', async () => {
    const injected = vi.fn(async () => 'owner-adapter')
    function Version() {
      const { api } = useOperateServices()
      return <button onClick={() => void api.appVersion()}>Read version</button>
    }
    render(<OperateServices adapter={{ ...denied(), handlers: { appVersion: injected } }}>
      <OperateServices><Version /></OperateServices>
    </OperateServices>)
    fireEvent.click(screen.getByText('Read version'))
    await waitFor(() => expect(injected).toHaveBeenCalledTimes(1))
    expect(native.appVersion).not.toHaveBeenCalled()
  })

  it('preserves the original top bar while disabling unsupported transmit and radio controls', () => {
    const command = vi.fn()
    const { container } = render(<OperateServices adapter={denied()}><TopBar
      mycall={snap.mycall} mygrid={snap.mygrid} radio={snap.radio} link={snap.link}
      bandPlan={[]} tier="FT8" onSetFrequency={command} onSetTxEnabled={command}
      onSetTune={command} onHaltTx={command} onSetTxEven={command}
      onSetTxCycleAuto={command} onSetHoldTxFreq={command}
      onTierChange={command} onOpenGuide={command} /></OperateServices>)
    expect(container.querySelector('.topbar')).not.toBeNull()
    for (const label of ['TX Off', 'Tune', 'Stop TX']) {
      const button = screen.getByRole('button', { name: label })
      expect((button as HTMLButtonElement).disabled).toBe(true)
      fireEvent.click(button)
    }
    expect(command).not.toHaveBeenCalled()
    expect(native.appVersion).not.toHaveBeenCalled()
    expect(native.getMeters).not.toHaveBeenCalled()
  })

  it('does not equate an allowed dial write with VFO/RIT/XIT support', async () => {
    const setFrequency = vi.fn(async () => snap)
    const { container } = render(<OperateServices adapter={{ ...denied(),
      actions: new Set(['radio']), handlers: { setFrequency } }}><TuningStrip snap={snap} /></OperateServices>)
    const nudge = container.querySelector<HTMLButtonElement>('.tuning-nudge')!
    expect(nudge.disabled).toBe(false)
    fireEvent.click(nudge)
    await waitFor(() => expect(setFrequency).toHaveBeenCalledTimes(1))
    for (const button of container.querySelectorAll('.tuning-vfo button, .tuning-clar button')) expect((button as HTMLButtonElement).disabled).toBe(true)
    expect(native.setFrequency).not.toHaveBeenCalled()
    expect(native.setVfo).not.toHaveBeenCalled()
  })

  it('keeps owner rotor Stop available without an azimuth or satellite service', async () => {
    const stopRotator = vi.fn(async () => {})
    render(<OperateServices adapter={{ ...denied(), actions: new Set(['stopRotator']), handlers: { stopRotator } }}>
      <RotorStrip />
    </OperateServices>)
    const stop = screen.getByRole('button', { name: '■' })
    expect((stop as HTMLButtonElement).disabled).toBe(false)
    fireEvent.click(stop)
    await waitFor(() => expect(stopRotator).toHaveBeenCalledTimes(1))
    expect(native.stopRotator).not.toHaveBeenCalled()
    expect(native.stopSatTrack).not.toHaveBeenCalled()
    expect(native.readRotator).not.toHaveBeenCalled()
  })

  it('dispatches owner rotor Stop without waiting for a stalled satellite stop', async () => {
    const stopRotator = vi.fn(async () => {})
    const stopSatTrack = vi.fn(() => new Promise<void>(() => {}))
    const adapter: OperateServiceAdapter = { ...denied(), actions: new Set(['stopRotator']), handlers: {
      readRotator: async () => 123, stopRotator, stopSatTrack,
    } }
    render(<OperateServices adapter={adapter}><RotorStrip /></OperateServices>)
    await waitFor(() => expect(screen.getByText('123°T')).toBeTruthy())
    fireEvent.click(screen.getByRole('button', { name: '■' }))
    expect(stopRotator).toHaveBeenCalledOnce()
    expect(stopSatTrack).toHaveBeenCalledOnce()
  })

  it('shows absent owner meter data as unavailable and never subscribes to the native meter bus', () => {
    render(<OperateServices adapter={denied()}><LiveLevelMeter label="Owner RX" /></OperateServices>)
    expect(screen.getByLabelText('Owner RX').textContent).toContain('—')
    expect(native.getMeters).not.toHaveBeenCalled()
  })

  it('offers only explicitly supported decoder modes, independently of method presence', () => {
    const services = bindOperateServices({ ...denied(), actions: new Set(['decoder']), tiers: new Set(['FT8', 'FT4']) })
    expect(services.canTier('FT8')).toBe(true)
    expect(services.canTier('FT4')).toBe(true)
    expect(services.canTier('MSK144')).toBe(false)
    expect(services.can('redecode')).toBe(false)
  })

  it('retires a queued wheel target when the station service changes', async () => {
    vi.useFakeTimers()
    const a = vi.fn(async () => snap), b = vi.fn(async () => snap), onSnap = vi.fn()
    const ownerA: OperateServiceAdapter = { ...denied(), actions: new Set(['radio']), handlers: { setFrequency: a } }
    const ownerB: OperateServiceAdapter = { ...denied(), actions: new Set(['radio']), handlers: { setFrequency: b } }
    const view = render(<OperateServices adapter={ownerA}><Wheel onSnap={onSnap} /></OperateServices>)
    fireEvent.click(screen.getByText('Wheel target'))
    view.rerender(<OperateServices adapter={ownerB}><Wheel onSnap={onSnap} /></OperateServices>)
    await act(() => vi.advanceTimersByTimeAsync(120))
    expect(a).not.toHaveBeenCalled()
    expect(b).not.toHaveBeenCalled()
    fireEvent.click(screen.getByText('Wheel target'))
    await act(() => vi.advanceTimersByTimeAsync(120))
    expect(b).toHaveBeenCalledExactlyOnceWith(14.1001, '20m', 'USB')
    expect(onSnap).toHaveBeenCalledOnce()
  })

  it('does not deliver an old station wheel response after a service switch', async () => {
    vi.useFakeTimers()
    const response = deferred<AppSnapshot>(), onSnap = vi.fn()
    const ownerA: OperateServiceAdapter = { ...denied(), actions: new Set(['radio']), handlers: { setFrequency: () => response.promise } }
    const view = render(<OperateServices adapter={ownerA}><Wheel onSnap={onSnap} /></OperateServices>)
    fireEvent.click(screen.getByText('Wheel target'))
    await act(() => vi.advanceTimersByTimeAsync(120))
    view.rerender(<OperateServices adapter={denied()}><Wheel onSnap={onSnap} /></OperateServices>)
    await act(async () => response.resolve(snap))
    expect(onSnap).not.toHaveBeenCalled()
  })

  it('clears the previous rotor immediately and ignores retired settings and declination responses', async () => {
    const settings = deferred<Awaited<ReturnType<typeof native.getSettings>>>()
    const declination = deferred<Awaited<ReturnType<typeof native.getDeclination>>>()
    const ownerA: OperateServiceAdapter = { ...denied(), handlers: {
      readRotator: async () => 123, getSettings: () => settings.promise, getDeclination: () => declination.promise,
    } }
    const view = render(<OperateServices adapter={ownerA}><RotorStrip /></OperateServices>)
    await waitFor(() => expect(view.container.textContent).toContain('123°T'))
    view.rerender(<OperateServices adapter={denied()}><RotorStrip /></OperateServices>)
    expect(view.container.textContent).not.toContain('123°T')
    await act(async () => {
      settings.resolve({ rotatorModel: 1, rotatorHost: 'synthetic' } as Awaited<ReturnType<typeof native.getSettings>>)
      declination.resolve(12)
    })
    expect(view.container.innerHTML).toBe('')
  })

  it('does not replace the new source version with a late old source response', async () => {
    const old = deferred<string>()
    const view = render(<OperateServices adapter={{ ...denied(), handlers: { appVersion: () => old.promise } }}><Header /></OperateServices>)
    view.rerender(<OperateServices adapter={{ ...denied(), handlers: { appVersion: async () => 'owner-B' } }}><Header /></OperateServices>)
    await waitFor(() => expect(view.container.querySelector('.app-version')?.textContent).toBe('vowner-B'))
    await act(async () => old.resolve('owner-A'))
    expect(view.container.querySelector('.app-version')?.textContent).toBe('vowner-B')
  })

  it('retires same-call award facts immediately while the next source is pending', async () => {
    const qsl = { card: false, lotw: false, eqsl: false, eqslAg: false, qrz: false }
    const stats: CallStats = { call: 'K1ABC', bands: ['20m'], modes: ['Digital'], newOne: true, newBand: true,
      newMode: true, dxcc: { label: 'Owner A entity', code: 291, rows: [['worked']] }, wasFamily: false,
      was: null, waz: null, countryQsl: qsl, callQsl: qsl }
    const next = deferred<CallStats>()
    const child = <CallStatsPane call="K1ABC" band="20m" mode="FT8" />
    const view = render(<OperateServices adapter={{ ...denied(), handlers: { getCallStats: async () => stats } }}>{child}</OperateServices>)
    await waitFor(() => expect(view.container.textContent).toContain('Owner A entity'))
    view.rerender(<OperateServices adapter={{ ...denied(), handlers: { getCallStats: () => next.promise } }}>{child}</OperateServices>)
    expect(view.container.textContent).not.toContain('Owner A entity')
    expect(view.container.querySelector('.cstats-flags')?.getAttribute('data-idle')).toBe('true')
    await act(async () => next.resolve({ ...stats, dxcc: { label: 'Owner B entity', code: 1, rows: [['confirmed']] } }))
    expect(view.container.textContent).toContain('Owner B entity')
  })

  it('retires previous bearing and entity facts while the new source is pending', async () => {
    function Facts() {
      const aims = useCallBearings(['K1ABC'])
      const places = useEntityCentroids()
      return <span>{aims.size} bearings; {places?.size ?? 0} centroids</span>
    }
    const ownerA: OperateServiceAdapter = { ...denied(), handlers: {
      rotatorBearingToCall: async () => ({ pointed: { bearing: 123 }, km: 10 } as Awaited<ReturnType<typeof native.rotatorBearingToCall>>),
      getDxccEntityLocations: async () => [['Synthetic', 1, 2]],
    } }
    const ownerB: OperateServiceAdapter = { ...denied(), handlers: {
      rotatorBearingToCall: () => new Promise(() => {}), getDxccEntityLocations: () => new Promise(() => {}),
    } }
    const view = render(<OperateServices adapter={ownerA}><Facts /></OperateServices>)
    await waitFor(() => expect(view.container.textContent).toContain('1 bearings; 1 centroids'))
    view.rerender(<OperateServices adapter={ownerB}><Facts /></OperateServices>)
    expect(view.container.textContent).toContain('0 bearings; 0 centroids')
  })
})
