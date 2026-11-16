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
export interface RxContext {
  worker_instance: string; worker_epoch: number; server_instance: string
  workspace_id: string; route: RxRoute; decoder_mode: RxMode
}
export interface RxRow {
  decode_id: string; sender_callsign: string | null
  context: RxContext
  needs: RxNeeds
  host_period_start_unix_us: number; mode: RxMode; message: string; snr_db: number
  time_offset_seconds: number; audio_frequency_hz: number
  clock: { timestamp_basis: string; host_mapping_uncertainty_us: number; calibration_uncertainty_us: number; utc_sync_accuracy_us: number | null; remote_rf_latency_us: number | null; sample_alignment_offset_ns: number; resampler_center_offset_ns: number }
}
export interface NeedsAuthority {
  logbook_revision: number; classification_revision: number
  country_reference_revision: string; policy_revision: string
}
export interface NeedsStatus {
  status: 'needed' | 'worked' | 'unknown'; score: number | null
  reason_code: string | null; reasons: string[]
}
export interface NeedsResult {
  query_id: string; normalized_callsign: string; dial_frequency_hz: number
  decoder_mode: RxMode; classification_mode: string; band: string | null
  country: string | null; adif: number | null; country_resolution: string | null
  call_status: string; dxcc_status: string; need: NeedsStatus
}
export interface RxNeeds extends NeedsStatus {
  /** Revision/session-bound passive facts, not live audio/control authority. */
  display_valid?: boolean
  owner: NeedsResult | null; authority: NeedsAuthority | null
  generated_at_unix_ms: number | null; max_age_ms: number | null; remaining_valid_ms: number
}
export interface RxView { enabled: boolean; mode: RxMode | null; phase: string; reason: string; route: RxRoute | null; current_context?: RxContext | null; rows: RxRow[] }
export const rxOff: RxView = { enabled:false, mode:null, phase:'off', reason:'Receive decoding is off.', route:null, rows:[] }
export const rxApi = {
  status: () => invoke<RxView>('xlog_rx_status'),
  start: (mode: RxMode) => invoke<RxView>('xlog_rx_start', { mode }),
  stop: () => invoke<RxView>('xlog_rx_stop'),
}
export type RxApi = typeof rxApi
