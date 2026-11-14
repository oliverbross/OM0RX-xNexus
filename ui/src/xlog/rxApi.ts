// SPDX-License-Identifier: GPL-3.0-only
import { invoke } from './api'
export type RxMode = 'FT8' | 'FT4'
export interface RxRoute {
  capture_id: string; epoch: string; radio_id: string; receiver_id: string
  mapping_generation: number; device_id: string; slice: string; dax_channel: number
  frequency_hz: number; mode: string; source_channels: number; selected_channel: number
  sample_rate: number; sample_format: string; timestamp_basis: string
  remote_rf_latency_us: number | null; utc_sync_accuracy_us: number | null
}
export interface RxRow {
  host_period_start_unix_us: number; mode: RxMode; message: string; snr_db: number
  time_offset_seconds: number; audio_frequency_hz: number
  clock: { timestamp_basis: string; host_mapping_uncertainty_us: number; calibration_uncertainty_us: number; utc_sync_accuracy_us: number | null; remote_rf_latency_us: number | null; sample_alignment_offset_ns: number; resampler_center_offset_ns: number }
}
export interface RxView { enabled: boolean; mode: RxMode | null; phase: string; reason: string; route: RxRoute | null; rows: RxRow[] }
export const rxOff: RxView = { enabled:false, mode:null, phase:'off', reason:'Receive decoding is off.', route:null, rows:[] }
export const rxApi = {
  status: () => invoke<RxView>('xlog_rx_status'),
  start: (mode: RxMode) => invoke<RxView>('xlog_rx_start', { mode }),
  stop: () => invoke<RxView>('xlog_rx_stop'),
}
export type RxApi = typeof rxApi
