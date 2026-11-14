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
  need: { status: NeedStatus; score: number; reasons: string[] }
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
}
type Native = { core: { invoke: <T>(name: string, args?: Record<string, unknown>) => Promise<T> } }
function invoke<T>(name: string, args?: Record<string, unknown>): Promise<T> {
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
}
export type Bridge = typeof bridge
export const disconnected: View = { paired: false, connected: false, endpoint: null, hello: null, snapshot: null }
