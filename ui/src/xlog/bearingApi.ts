// SPDX-License-Identifier: GPL-3.0-only
import { invoke, type StationIdentityBinding } from './api'

export interface BearingTarget {
  source: 'exact_logbook_grid' | 'cty_entity_approximation'
  grid: string | null
  latitude: number
  longitude: number
  precision: 'historical_grid_cell_center' | 'entity_approximation'
  contact_id: string | null
  contact_revision: number | null
  contact_started_at_unix_ms: number | null
  reference_revision: string | null
  matched_reference_revision: string | null
  matched_rule: string | null
  country_status: 'resolved' | null
}
export interface BearingData {
  schema_version: 1
  requested_callsign: string
  normalized_callsign: string
  status: 'available' | 'unavailable'
  reason: string | null
  origin: { source: 'saved_station_settings'; grid: string; latitude: number; longitude: number } | null
  target: BearingTarget | null
  geometry: { short_path_deg: number; long_path_deg: number; distance_km: number } | null
  logbook_revision: number
  reference_revision: string | null
  motion_authority: false
  transmit_authority: false
}
export interface BearingEnvelope {
  protocol_version: 1
  server_instance: string
  workspace_id: string
  generation: string
  worker_generation: number
  station_identity_binding: StationIdentityBinding
  generated_at_unix_ms: number
  max_age_ms: 5000
  bearing: BearingData
}
export interface BearingReadView {
  connection_token: string
  payload: BearingEnvelope
  remaining_valid_ms: number
}
/** Native-only; no frontend HTTP, resolver, device or motion fallback. */
export const bearingReads = {
  read: (connectionToken: string, callsign: string) => invoke<unknown>('xlog_bearing_read', { connectionToken, callsign }),
}
