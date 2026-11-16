// SPDX-License-Identifier: GPL-3.0-only
// Original FT controls reach only the native xLog coordinator. No browser HTTP,
// local Engine, audio device or fallback station is available here.
import { invoke, type OutboxEntry } from './api'
import type { RxMode } from './rxApi'
export interface FtView {
  connection_token: string; available: boolean; armed: boolean; phase: string; reason: string
  mode: RxMode | null; tx_even: boolean; tx_cycle_auto: boolean; auto_sequence: boolean
  hold: boolean; skip_tx1: boolean; search_and_pounce: boolean; offset_hz: number; gain: number
  pending_message: string | null; tx_count: number; keyed: boolean; tuning: boolean; done: boolean
  dx_call: string | null; dx_grid: string | null; rx_report: number | null; qso_state: string
  cq_running: boolean; stalled: boolean; watchdog_stopped: boolean; remaining_valid_ms: number
  operation_id: string | null; release_pending: boolean
}
export type FtAction =
  | { kind: 'resend' | 'clear' | 'stop' }
  | { kind: 'cq'; direction?: string | null }
  | { kind: 'work'; decode_id: string }
  | { kind: 'enable' | 'auto_cycle' | 'auto_sequence' | 'hold' | 'skip_tx1' | 'search_and_pounce' | 'tune'; enabled: boolean }
  | { kind: 'parity'; even: boolean }
  | { kind: 'override'; message: string; call: string | null; grid: string | null }
  | { kind: 'free_text'; message: string }
  | { kind: 'offset'; hz: number }
  | { kind: 'drive'; gain: number }
export const ftOff: FtView = {
  connection_token: '', available: false, armed: false, phase: 'off', reason: 'No current xLog transmit connection.',
  mode: null, tx_even: true, tx_cycle_auto: true, auto_sequence: true, hold: false, skip_tx1: false,
  search_and_pounce: true, offset_hz: 1500, gain: .8, pending_message: null, tx_count: 0,
  keyed: false, tuning: false, done: false, dx_call: null, dx_grid: null, rx_report: null,
  qso_state: 'Listening', cq_running: false, stalled: false, watchdog_stopped: false,
  remaining_valid_ms: 0, operation_id: null, release_pending: false,
}
export const ftApi = {
  status: (connectionToken: string) => invoke<FtView>('xlog_ft_status', { connectionToken }),
  action: (connectionToken: string, action: FtAction) => invoke<FtView>('xlog_ft_action', { connectionToken, action }),
  log: (connectionToken: string) => invoke<OutboxEntry[]>('xlog_ft_log', { connectionToken }),
}
export type FtApi = typeof ftApi
