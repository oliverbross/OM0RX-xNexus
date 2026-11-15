// SPDX-License-Identifier: GPL-3.0-only
// A separate native-only bridge. Never falls back to browser HTTP or Nexus API.
export type NeedStatus = 'needed' | 'worked' | 'unknown'
export interface Spot {
  id: string
  callsign: string
  frequency_hz: number
  mode: string
  observed_at_unix_ms: number
  source: string
  country?: string | null
  adif?: number | null
  latitude?: number | null
  longitude?: number | null
  band?: string | null
  call_status?: string | null
  dxcc_status?: string | null
  need: { status: NeedStatus; score: number; reasons: string[] }
}
export interface StationIdentityBinding {
  workspace_generation: string
  worker_generation: number
  settings_revision: number
  identity_epoch: string
  identity_revision: number
  operating_module: 'normal_logging'
}
/** Saved owner identity; usable only with its current validated Snapshot. Never TX authority. */
export interface StationIdentity {
  schema_version: 1
  source: 'saved_station_settings'
  state: 'current' | 'incomplete' | 'unavailable'
  mycall: string | null
  mygrid: string | null
  binding: StationIdentityBinding | null
  transmit_authority: false
}
export interface Snapshot {
  protocol_version: number
  server_instance: string
  workspace_id: string
  revision: number
  generated_at_unix_ms: number
  max_age_ms: number
  station: { connected: boolean; radio_id: string | null; frequency_hz: number | null; mode: string | null; transmitting: boolean | null }
  rotor: { connected: boolean; rotor_id: string | null; azimuth_deg: number | null; moving: boolean | null }
  spots: Spot[]
  station_identity?: StationIdentity | null
}
export interface Hello {
  application: string
  protocol_version: number
  server_instance: string
  workspace_id: string
  capabilities: string[]
}
export interface View {
  paired: boolean
  connected: boolean
  endpoint: string | null
  hello: Hello | null
  snapshot: Snapshot | null
  /** Missing on older native shells; passive owner reads stay unavailable. */
  connection_token?: string | null
}
export type Command =
  | { kind: 'station.tune'; radio_id: string; frequency_hz: number; mode: string }
  | { kind: 'rotor.point'; rotor_id: string; azimuth_deg: number }
  | { kind: 'rotor.stop'; rotor_id: string }
export interface Receipt {
  request_id: string
  server_instance: string
  workspace_id: string
  status: 'accepted' | 'applied' | 'observed' | 'rejected' | 'uncertain'
  message: string | null
  contact_id?: string | null
  contact_revision?: number | null
}
export interface QsoDraft {
  contact_id: string
  callsign: string
  band: string
  mode: string
  started_at_unix_ms: number
  frequency_hz: number | null
  rst_sent: string
  rst_received: string
  grid: string
  comment: string
}
export type LogbookMutation = { kind: 'logbook.create'; qso: QsoDraft } | { kind: 'logbook.update'; qso: QsoDraft; expected_contact_revision: number }
export interface OutboxEntry {
  queued: { version: number; request_id: string; workspace_id: string; mutation: LogbookMutation }
  state: 'Pending' | 'NeedsReceipt' | { Settled: Receipt }
}
export type DeliveryAction = 'submit' | 'reconcile' | 'retry_missing'
export interface HistoryContact extends QsoDraft {
  contact_revision: number | null
  /** Additive owner schema. Missing fields are legacy, never complete details. */
  submode?: string
  exact_mode?: string
  details_complete?: boolean
}
export interface HistorySummary {
  schema_version: 1
  scope: 'normal_log_base_callsign'
  complete: boolean
  total: number
  worked_before: boolean
  confirmed_count: number
  confirmation_policy: 'xlog_cluster_paper_or_lotw_v1'
  confirmation_sources: ['paper_qsl', 'lotw']
  last_started_at_unix_ms: number | null
  bands: string[]
  bands_complete: boolean
  modes: string[]
  modes_complete: boolean
  requested_band: string | null
  requested_exact_mode: string | null
  requested_band_worked: boolean | null
  requested_band_exact_mode_worked: boolean | null
}
export interface History {
  protocol_version: number
  server_instance: string
  workspace_id: string
  callsign: string
  generated_at_unix_ms: number
  contacts: HistoryContact[]
  matrix: { band: string; mode: string; worked: boolean; confirmed: boolean }[]
  /** Optional as one complete group. Legacy pages are not full-call summaries. */
  generation?: string
  logbook_revision?: number
  max_age_ms?: number
  contacts_limit?: 25
  contacts_complete?: boolean
  matrix_complete?: boolean
  history_summary?: HistorySummary
}
export interface RecentContacts {
  protocol_version: number
  server_instance: string
  workspace_id: string
  schema_version: 1
  generation: string
  logbook_revision: number
  generated_at_unix_ms: number
  max_age_ms: number
  contacts: HistoryContact[]
  limit: 5
  total: number
  complete: boolean
  ordering: 'qso_start_desc_contact_id_asc'
}
/** Native emits this only after rechecking the original session/read lease.
 * UI anchors expiry at IPC request start, never receipt time or each re-render. */
export interface HistoryReadView<T> {
  payload: T
  connection_token: string
  remaining_valid_ms: number
}
type Native = { core: { invoke: <T>(name: string, args?: Record<string, unknown>) => Promise<T> } }
export function invoke<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  const native = (window as unknown as { __TAURI__?: Native }).__TAURI__
  if (!native) return Promise.reject(new Error('Open the OM0RX-xNexus desktop to connect to xLog.'))
  return native.core.invoke<T>(name, args)
}
export const bridge = {
  status: () => invoke<View>('xlog_status'),
  importPairing: () => invoke<View | null>('xlog_import_pairing'),
  connect: () => invoke<View>('xlog_connect'),
  poll: () => invoke<View>('xlog_poll'),
  disconnect: (forget = false) => invoke<View>('xlog_disconnect', { forget }),
  command: (command: Command, requestId = crypto.randomUUID()) => invoke<Receipt>('xlog_command', { command, requestId }),
  outbox: () => invoke<OutboxEntry[]>('xlog_outbox_list'),
  enqueue: (mutation: LogbookMutation, requestId: string, expectedWorkspaceId: string) => invoke<OutboxEntry[]>('xlog_outbox_enqueue', { mutation, requestId, expectedWorkspaceId }),
  deliver: (requestId: string, action: DeliveryAction) => invoke<Receipt>('xlog_outbox_deliver', { requestId, action }),
  history: (callsign: string) => invoke<History>('xlog_history', { callsign }),
}
export type Bridge = typeof bridge
/** Caller captures performance.now BEFORE invoke and fences token + query on return. */
export const historyReads = {
  history: (connectionToken: string, callsign: string, band?: string, exactMode?: string) =>
    invoke<HistoryReadView<History>>('xlog_history_read', { connectionToken, callsign, band: band ?? null, exactMode: exactMode ?? null }),
  recent: (connectionToken: string) =>
    invoke<HistoryReadView<RecentContacts>>('xlog_recent', { connectionToken }),
}
export const disconnected: View = { paired: false, connected: false, endpoint: null, hello: null, snapshot: null }
