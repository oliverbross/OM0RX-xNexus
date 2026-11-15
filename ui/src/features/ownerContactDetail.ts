/** Read-only mirror of CompanionContactDetail.swift (source draft A70BE126).
 * This is a presentation input, not a transport validator or a live read lease. */
export type DetailField<T> =
  | { state: 'value'; value: T; reason: null }
  | { state: 'empty'; value: null; reason: null }
  | { state: 'unavailable'; value: null; reason: string }
type Text = DetailField<string>
type Integer = DetailField<number>
type TextFields<K extends string> = { [P in K]: Text }
export type DetailReferences = TextFields<'pota' | 'sota' | 'wwff' | 'iota' | 'sig' | 'sig_info'>
export type DetailConfirmation = TextFields<'source' | 'evidence_kind' | 'sent_status' | 'sent_date' | 'sent_via' | 'received_status' | 'received_date' | 'received_via'> & { match_age_seconds: DetailField<number> }
export type DetailUpload = TextFields<'source' | 'state'> & {
  receipt_revision: Integer; last_attempt_at_unix_ms: Integer; is_current_revision: DetailField<boolean>
}
export interface OwnerContactFields {
  contact: TextFields<'callsign' | 'band' | 'mode' | 'submode' | 'stored_time_unit' | 'original_start_precision' | 'original_end_precision' | 'raw_qso_date' | 'raw_time_on' | 'raw_qso_date_off' | 'raw_time_off' | 'rst_sent' | 'rst_received' | 'grid' | 'comment' | 'notes'> & {
    started_at_unix_ms: Integer; ended_at_unix_ms: Integer
  }
  radio: TextFields<'receive_band' | 'tx_power_raw' | 'propagation_mode' | 'antenna_path' | 'satellite_name'> & {
    frequency_hz: Integer; receive_frequency_hz: Integer; tx_power_w: DetailField<number>
  }
  station: TextFields<'name' | 'qth' | 'state' | 'county' | 'country' | 'continent' | 'cq_zone' | 'itu_zone'> & { dxcc: Integer }
  own_station: TextFields<'operator_call' | 'callsign' | 'grid' | 'recorded_rig' | 'radio_label' | 'station_location'> & { station_profile_id: Integer }
  references: { their: DetailReferences; my: DetailReferences; dok: Text }
  confirmations: { paper: DetailConfirmation; lotw: DetailConfirmation; eqsl: DetailConfirmation; qrz: DetailConfirmation }
  awards: TextFields<'credit_granted' | 'credit_submitted' | 'authority'>
  uploads: { lotw: DetailUpload; eqsl: DetailUpload; qrz: DetailUpload; clublog: DetailUpload }
  contest: TextFields<'contest_id' | 'stx' | 'srx' | 'stx_string' | 'srx_string'>
}
export interface OwnerContactDetail {
  schema_version: 1
  contact_id: string
  status: 'found' | 'deleted' | 'not_found' | 'unavailable'
  reason: string | null
  logbook_revision: number
  contact_revision: number | null
  fields: OwnerContactFields | null
  editability: {
    allowed: boolean; reason: string; allowed_fields: string[]
    base: import('../xlog/api').QsoDraft | null
  }
  locate: Text
}

/** Caller must retire current immediately when its real owner read expires or
 * changes. No component timer or re-render grants a fresh authority lease. */
export type OwnerContactRead =
  | { state: 'current'; detail: OwnerContactDetail }
  | { state: 'loading' | 'unavailable'; message: string }

export function detailText<T>(field: DetailField<T>, unavailable: string, format: (value: T) => string = String): string {
  return field.state === 'value' ? format(field.value) : field.state === 'empty' ? '' : unavailable
}

export function mapDetailField<T>(field: DetailField<T>, format: (value: T) => string): Text {
  return field.state === 'value' ? { state: 'value', value: format(field.value), reason: null } : field
}

/** Do not collapse multiple references into the native single-program model. */
export function referenceText(side: DetailReferences, unavailable: string): string {
  return (['pota', 'sota', 'wwff', 'iota', 'sig', 'sig_info'] as const)
    .filter((key) => side[key].state !== 'empty')
    .map((key) => `${key.toUpperCase()}: ${detailText(side[key], unavailable)}`).join(' · ')
}
