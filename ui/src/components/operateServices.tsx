// Component-scoped integration seam. No global IPC replacement and no station
// construction. Omitted provider keeps upstream behavior; an explicit adapter
// must implement each allowed operation and never falls back to the native API.
import { createContext, useContext, useMemo, type ReactNode } from 'react'
import * as native from '../api'
import type { Tier } from '../types'

const names = [
  'atuBypass', 'atuTune', 'closePanelWindow', 'openPanelWindow', 'getSettings',
  'notifyErase', 'setSettings', 'setMsk144Period', 'clearDxCall', 'pointRotatorAtCall',
  'redecode', 'startCq', 'startRunning', 'startSearchAndPounce', 'startQsoRecording',
  'stopQsoRecording', 'setDecodeDepth', 'setSkipTx1', 'qrzLookup', 'resolveEntity',
  'getDecodeRefill', 'getSpectrumRow', 'getDeclination',
  'getSatTrackStatus', 'getSatTransponder', 'readRotator', 'setSatTransponder',
  'stopRotator', 'stopSatTrack', 'setSplit', 'setFrequency', 'setRit', 'setXit',
  'setVfo', 'swapVfo', 'getCallStats', 'getFastPower', 'requestWindowAttention',
  'openExternalUrl', 'openQrzPage', 'postSpot', 'dismissTxAlarm', 'getMeters',
  'getDxccEntityLocations', 'getAudioScopeFrame', 'getRfFrame', 'getScopeFrame',
  'appVersion', 'repairClock', 'getPrivilegeSpans', 'rotatorBearingToCall',
] as const satisfies readonly (keyof typeof native)[]
export type OperateServiceName = typeof names[number]
export type OperateApi = Omit<Pick<typeof native, OperateServiceName>, 'setFrequency' | 'getSpectrumRow'> & {
  // An owner command completes with its own receipt and refreshes observations
  // independently; it must never forge an AppSnapshot to satisfy a callback.
  setFrequency: (...args: Parameters<typeof native.setFrequency>) => Promise<Awaited<ReturnType<typeof native.setFrequency>> | void>
  // No new/fresh spectrum is distinct from a real quiet row of samples.
  getSpectrumRow: (...args: Parameters<typeof native.getSpectrumRow>) => Promise<Awaited<ReturnType<typeof native.getSpectrumRow>> | null>
}
export const OPERATE_UNAVAILABLE = 'Unavailable through this station connection.'
export type OperateAction = 'transmit' | 'stopTransmit' | 'rxMarker' | 'radio' | 'rotator' | 'stopRotator' | 'source' | 'decoder' | 'log' | 'settings' | 'panels' | 'recording' | 'cluster' | 'external' | 'clock' | 'propagation'
const actionFor: Partial<Record<OperateServiceName, OperateAction>> = {
  atuTune: 'transmit', atuBypass: 'radio', startCq: 'transmit', startRunning: 'transmit',
  startSearchAndPounce: 'transmit', clearDxCall: 'stopTransmit', setSkipTx1: 'transmit',
  pointRotatorAtCall: 'rotator', setSettings: 'settings', redecode: 'decoder',
  setMsk144Period: 'decoder', setDecodeDepth: 'decoder', notifyErase: 'decoder',
  openPanelWindow: 'panels', closePanelWindow: 'panels',
  startQsoRecording: 'recording', stopQsoRecording: 'recording',
  stopRotator: 'stopRotator', stopSatTrack: 'stopRotator', setSatTransponder: 'radio',
  setSplit: 'radio', setFrequency: 'radio', setRit: 'radio', setXit: 'transmit',
  setVfo: 'radio', swapVfo: 'radio', postSpot: 'cluster',
  openExternalUrl: 'external', openQrzPage: 'external', repairClock: 'clock',
  requestWindowAttention: 'panels', dismissTxAlarm: 'settings',
}
export interface OperateServiceAdapter {
  readonly handlers: Partial<OperateApi>
  readonly actions: ReadonlySet<OperateAction>
  /** An explicit adapter offers only the decoder modes it can actually run. */
  readonly tiers?: ReadonlySet<Tier>
  /** Operator-visible refusal supplied by the host; never a fabricated receipt. */
  readonly onDeniedAction: (action: OperateAction) => void
}
interface Services {
  api: OperateApi
  adapter?: OperateServiceAdapter
  can: (name: OperateServiceName) => boolean
  canAction: (action: OperateAction) => boolean
  canTier: (tier: Tier) => boolean
}
// Resolve the native export only when called, as the original imported functions
// did. This also preserves upstream tests that mock only the exercised exports.
const upstream: Services = {
  api: Object.fromEntries(names.map(name => [name, (...args: unknown[]) =>
    Reflect.apply(native[name], undefined, args)])) as OperateApi,
  can: () => true,
  canAction: () => true,
  canTier: () => true,
}
const Context = createContext<Services>(upstream)

export function bindOperateServices(adapter: OperateServiceAdapter): Services {
  // This finite mapped surface preserves each public method's argument/result
  // type. Unsupported calls reject, including calls made by shortcuts/effects.
  const api = Object.fromEntries(names.map(name => [name, (...args: unknown[]) => {
    const action = actionFor[name]
    if (action && !adapter.actions.has(action)) {
      adapter.onDeniedAction(action)
      return Promise.reject(new Error(`This station connection does not allow ${name}.`))
    }
    const handler = adapter.handlers[name]
    if (!handler) return Promise.reject(new Error(`This station connection does not provide ${name}.`))
    return Reflect.apply(handler, undefined, args)
  }])) as OperateApi
  return { api, adapter, canAction: action => adapter.actions.has(action),
    canTier: tier => adapter.actions.has('decoder') && adapter.tiers?.has(tier) === true, can: name => {
    const action = actionFor[name]
    return typeof adapter.handlers[name] === 'function' && (!action || adapter.actions.has(action))
  } }
}
export function OperateServices({ adapter, children }: { adapter?: OperateServiceAdapter; children: ReactNode }) {
  const inherited = useContext(Context)
  const value = useMemo(() => adapter ? bindOperateServices(adapter) : inherited, [adapter, inherited])
  return <Context.Provider value={value}>{children}</Context.Provider>
}
export function useOperateServices(): Services { return useContext(Context) }

export function guardOperateTier(adapter: OperateServiceAdapter | undefined, callback: (tier: Tier) => void): (tier: Tier) => void {
  if (!adapter) return callback
  return tier => {
    if (!adapter.actions.has('decoder') || !adapter.tiers?.has(tier)) { adapter.onDeniedAction('decoder'); return }
    callback(tier)
  }
}

export function guardOperateAction<A extends unknown[]>(adapter: OperateServiceAdapter | undefined, action: OperateAction, callback: (...args: A) => void): (...args: A) => void {
  if (!adapter) return callback
  return (...args) => {
    if (!adapter.actions.has(action)) { adapter.onDeniedAction(action); return }
    callback(...args)
  }
}
