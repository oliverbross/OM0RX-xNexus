// SPDX-License-Identifier: GPL-3.0-only
// Local native spectrum only. No browser HTTP, Engine, capture or invented bins.
import { invoke, type View } from './api'
import type { RxContext, RxRoute, RxView } from './rxApi'
import type { Spectrum } from '../types'

export interface SpectrumCursor { worker_instance: string; worker_epoch: number; sequence: number }
export interface SpectrumFrame {
  context: RxContext; sequence: number; remaining_valid_ms: number
  sample_rate_hz: 12000; fft_samples: 2048; first_output_index: number
  source_origin_frame: number; source_origin_unix_us: number; source_sample_rate_hz: number
  fir_center_source_frames_twice: 0 | 63
  window_start_unix_us: number; window_end_unix_us: number
  timestamp_basis: 'windows_endpoint_capture'
  host_mapping_uncertainty_us: number; calibration_uncertainty_us: number
  utc_sync_accuracy_us: null; remote_rf_latency_us: null
  scale_lo_dbfs: -120; scale_hi_dbfs: 0
  spectrum: Spectrum & { loHz: 0; hiHz: 5000; source: 'audio' }
}
export interface SpectrumBinding {
  /** Host increments before Stop, start, disconnect/re-pair, even for identical IDs. */
  sessionGeneration: number
  owner: Pick<View, 'paired' | 'connected' | 'hello'>
  rx: Pick<RxView, 'enabled' | 'mode' | 'route' | 'current_context'>
}
interface Delivery {
  bindingGeneration: number; resetToken: number; reset: boolean; contextKey: string | null
}
export type SpectrumRead =
  | (Delivery & { kind: 'frame'; frame: SpectrumFrame; spectrum: Spectrum; deadline: number })
  | (Delivery & { kind: 'unavailable'; reason: 'unavailable' | 'busy' | 'no_new_frame' | 'expired' | 'invalid' | 'error' | 'superseded'; deadline: number | null })

export const spectrumApi = {
  read: (after: SpectrumCursor | null) => invoke<unknown>('xlog_rx_spectrum', { after }),
}
const safe = (v: unknown, min = 0, max = Number.MAX_SAFE_INTEGER): v is number =>
  typeof v === 'number' && Number.isSafeInteger(v) && v >= min && v <= max
const text = (v: unknown, max = 4096): v is string =>
  typeof v === 'string' && v.length > 0 && v.length <= max && !/[\u0000-\u001f\u007f]/.test(v)
const object = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v)
const routeFields: (keyof RxRoute)[] = ['capture_id', 'epoch', 'radio_id', 'receiver_id', 'mapping_generation', 'device_id', 'slice', 'dax_channel', 'frequency_hz', 'mode', 'source_channels', 'selected_channel', 'sample_rate', 'sample_format', 'timestamp_basis', 'remote_rf_latency_us', 'utc_sync_accuracy_us']
function routeKey(value: unknown): string | null {
  if (!object(value) || !['capture_id', 'epoch', 'radio_id', 'receiver_id', 'device_id', 'mode'].every(k => text(value[k]))
    || !safe(value.mapping_generation, 1) || !safe(value.frequency_hz, 1)
    || value.slice !== 'A' || !safe(value.dax_channel, 1, 8)
    || !safe(value.source_channels, 1, 16) || !safe(value.selected_channel, 0, value.source_channels as number - 1)
    || !safe(value.sample_rate, 8000, 192000) || value.sample_format !== 's16le'
    || value.timestamp_basis !== 'windows_endpoint_capture' || value.remote_rf_latency_us !== null || value.utc_sync_accuracy_us !== null) return null
  return JSON.stringify(routeFields.map(k => value[k]))
}
export function spectrumContextKey(value: unknown): string | null {
  if (!object(value) || !text(value.worker_instance, 128) || !safe(value.worker_epoch, 1)
    || !text(value.server_instance, 128) || !text(value.workspace_id, 128)
    || (value.decoder_mode !== 'FT8' && value.decoder_mode !== 'FT4')) return null
  const route = routeKey(value.route)
  return route === null ? null : JSON.stringify([value.worker_instance, value.worker_epoch, value.server_instance, value.workspace_id, value.decoder_mode, route])
}
function bindingKey(binding: SpectrumBinding): { key: string; contextKey: string } | null {
  const { owner, rx } = binding
  if (!safe(binding.sessionGeneration) || !owner.paired || !owner.connected || !owner.hello
    || !owner.hello.capabilities.includes('audio.rx') || !rx.enabled || !rx.current_context) return null
  const contextKey = spectrumContextKey(rx.current_context)
  if (contextKey === null || rx.current_context.server_instance !== owner.hello.server_instance
    || rx.current_context.workspace_id !== owner.hello.workspace_id || rx.current_context.decoder_mode !== rx.mode
    || routeKey(rx.route) !== routeKey(rx.current_context.route)) return null
  return { key: JSON.stringify([binding.sessionGeneration, contextKey]), contextKey }
}
function validFrame(value: unknown, contextKey: string): value is SpectrumFrame {
  if (!object(value) || spectrumContextKey(value.context) !== contextKey || !object(value.context)
    || !object(value.context.route) || !safe(value.sequence, 1) || !safe(value.remaining_valid_ms, 1, 500)
    || value.sample_rate_hz !== 12000 || value.fft_samples !== 2048
    || !safe(value.first_output_index, 0, Number.MAX_SAFE_INTEGER - 2048)
    || !safe(value.source_origin_frame) || !safe(value.source_origin_unix_us, 1)
    || value.source_sample_rate_hz !== value.context.route.sample_rate || !safe(value.source_sample_rate_hz, 8000, 192000)
    || value.fir_center_source_frames_twice !== (value.source_sample_rate_hz === 12000 ? 0 : 63)
    || !safe(value.window_start_unix_us, 1) || !safe(value.window_end_unix_us, 1)
    || value.timestamp_basis !== 'windows_endpoint_capture' || value.utc_sync_accuracy_us !== null || value.remote_rf_latency_us !== null
    || !safe(value.host_mapping_uncertainty_us, 0, 50000) || !safe(value.calibration_uncertainty_us, 0, value.host_mapping_uncertainty_us)
    || value.scale_lo_dbfs !== -120 || value.scale_hi_dbfs !== 0 || !object(value.spectrum)
    || value.spectrum.loHz !== 0 || value.spectrum.hiHz !== 5000 || value.spectrum.source !== 'audio'
    || !Array.isArray(value.spectrum.row) || value.spectrum.row.length !== 640
    || !value.spectrum.row.every(v => typeof v === 'number' && Number.isFinite(v) && v >= 0 && v <= 1)) return false
  // Nominal rational source schedule, not an exact analog FIR center or RF UTC.
  const rate = BigInt(value.source_sample_rate_hz)
  const center = BigInt(value.fir_center_source_frames_twice as number)
  const time = (i: number) => BigInt(value.source_origin_unix_us as number)
    + (center * 12000n + 2n * BigInt(i) * rate) * 1000000n / (2n * rate * 12000n)
  return time(value.first_output_index) === BigInt(value.window_start_unix_us)
    && time(value.first_output_index + 2048) === BigInt(value.window_end_unix_us)
}

/** One per host binding. No pending queue; at most one native invocation. */
export function createSpectrumReader(options: { read?: typeof spectrumApi.read; now?: () => number } = {}) {
  const native = options.read ?? spectrumApi.read
  const clock = options.now ?? (() => performance.now())
  let key: string | null = null, contextKey: string | null = null
  let generation = 0, resetToken = 0, deliveredReset = 0
  let cursor: SpectrumCursor | null = null, deadline: number | null = null
  let pending = false, clockBad = false, lastClock = 0
  const increment = () => { if (generation >= Number.MAX_SAFE_INTEGER || resetToken >= Number.MAX_SAFE_INTEGER) { clockBad = true; return } generation++; resetToken++ }
  const clearPicture = () => { if (deadline !== null) { deadline = null; if (resetToken < Number.MAX_SAFE_INTEGER) resetToken++; else clockBad = true } }
  const now = () => {
    const value = clock()
    if (!Number.isFinite(value) || value < lastClock || value < 0) { clockBad = true; clearPicture(); return null }
    lastClock = value
    if (deadline !== null && value >= deadline) clearPicture()
    return value
  }
  const snapshot = () => ({ bindingGeneration:generation, resetToken, contextKey, deadline })
  const reset = () => { increment(); key = null; contextKey = null; cursor = null; deadline = null; return snapshot() }
  const synchronize = (binding: SpectrumBinding) => {
    const next = bindingKey(binding)
    if ((next?.key ?? null) !== key) { increment(); cursor = null; deadline = null; key = next?.key ?? null; contextKey = next?.contextKey ?? null }
    return snapshot()
  }
  const delivery = (): Delivery => {
    const result = { bindingGeneration:generation, resetToken, reset:resetToken !== deliveredReset, contextKey }
    deliveredReset = resetToken
    return result
  }
  const unavailable = (reason: Extract<SpectrumRead, { kind:'unavailable' }>['reason']): SpectrumRead =>
    ({ ...delivery(), kind:'unavailable', reason, deadline })
  const isCurrent = (result: SpectrumRead) => {
    const time = now()
    return time !== null && !clockBad && result.bindingGeneration === generation && result.contextKey === contextKey
      && result.resetToken === resetToken && (result.kind !== 'frame' || time < result.deadline)
  }
  return {
    reset,
    /** Call synchronously on status/lifecycle changes, not just on the next waterfall poll. */
    synchronize,
    isCurrent,
    /** Timer callback can observe expiry and clear the viewport without another IPC. */
    inspect: () => { now(); return snapshot() },
    async read(binding: SpectrumBinding): Promise<SpectrumRead> {
      synchronize(binding)
      const started = now()
      if (clockBad || started === null || key === null || contextKey === null) return unavailable('unavailable')
      if (pending) return unavailable('busy')
      const admittedGeneration = generation, admittedKey = key, admittedContext = contextKey
      pending = true
      try {
        const value = await native(cursor ? { ...cursor } : null)
        const finished = now()
        if (generation !== admittedGeneration || key !== admittedKey) return { kind:'unavailable', reason:'superseded', bindingGeneration:admittedGeneration, resetToken, reset:false, contextKey:admittedContext, deadline:null }
        if (clockBad || finished === null) return unavailable('unavailable')
        if (value === null) return unavailable(deadline === null ? (cursor ? 'expired' : 'unavailable') : 'no_new_frame')
        if (!validFrame(value, admittedContext)) { clearPicture(); return unavailable('invalid') }
        if (cursor && value.sequence <= cursor.sequence) return unavailable(deadline === null ? 'expired' : 'no_new_frame')
        const expires = started + value.remaining_valid_ms
        // Remember consumed evidence even if it expired in transit. Same bytes
        // must never acquire a fresh lease just because the native reply repeats.
        cursor = { worker_instance:value.context.worker_instance, worker_epoch:value.context.worker_epoch, sequence:value.sequence }
        if (finished >= expires) { clearPicture(); return unavailable('expired') }
        deadline = expires
        return { ...delivery(), kind:'frame', frame:value, spectrum:value.spectrum, deadline:expires }
      } catch {
        if (generation !== admittedGeneration || key !== admittedKey) return { kind:'unavailable', reason:'superseded', bindingGeneration:admittedGeneration, resetToken, reset:false, contextKey:admittedContext, deadline:null }
        clearPicture(); return unavailable('error')
      } finally { pending = false }
    },
  }
}
