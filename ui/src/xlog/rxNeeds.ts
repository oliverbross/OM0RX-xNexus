// SPDX-License-Identifier: GPL-3.0-only
import type { RxNeeds, RxRoute, RxRow, RxView } from './rxApi'

export interface OwnerIdentity { server_instance: string; workspace_id: string }
export interface NeedsLease { evidence: string; deadline: number }
export type NeedsLeases = Map<string, NeedsLease>

function routeKey(route: RxRoute): string {
  return JSON.stringify([route.capture_id, route.epoch, route.radio_id, route.receiver_id,
    route.mapping_generation, route.device_id, route.slice, route.dax_channel, route.frequency_hz,
    route.mode, route.source_channels, route.selected_channel, route.sample_rate, route.sample_format,
    route.timestamp_basis, route.remote_rf_latency_us, route.utc_sync_accuracy_us])
}

/** Validate the native attachment, never classify or derive an award score here. */
export function validNeeds(row: RxRow, view: Pick<RxView, 'route' | 'mode'>, identity?: OwnerIdentity): RxNeeds | null {
  const { route, mode } = view
  const facts = row.needs, context = row.context, owner = facts?.owner, authority = facts?.authority
  if (!identity || !route || !context?.route || !facts || !owner?.need || !authority || !row.sender_callsign ||
    !row.decode_id || row.decode_id.length > 128 || context.server_instance !== identity.server_instance ||
    context.workspace_id !== identity.workspace_id || context.decoder_mode !== row.mode || context.decoder_mode !== mode ||
    routeKey(context.route) !== routeKey(route) || owner.normalized_callsign !== row.sender_callsign ||
    owner.dial_frequency_hz !== context.route.frequency_hz || owner.decoder_mode !== context.decoder_mode) return null
  if (!Number.isSafeInteger(facts.generated_at_unix_ms) || facts.generated_at_unix_ms! <= 0 || facts.generated_at_unix_ms! > 8640000000000000 ||
    !Number.isSafeInteger(facts.max_age_ms) || facts.max_age_ms! <= 0 || facts.max_age_ms! > 2000 ||
    !Number.isSafeInteger(facts.remaining_valid_ms) || facts.remaining_valid_ms <= 0 ||
    facts.remaining_valid_ms > facts.max_age_ms! || !Number.isSafeInteger(authority.logbook_revision) || authority.logbook_revision < 0 ||
    !Number.isSafeInteger(authority.classification_revision) || authority.classification_revision < 0 || !authority.country_reference_revision || !authority.policy_revision) return null
  if (!['needed', 'worked', 'unknown'].includes(facts.status) || facts.status !== owner.need.status ||
    facts.score !== owner.need.score || facts.reason_code !== owner.need.reason_code ||
    !Array.isArray(facts.reasons) || facts.reasons.some(reason => typeof reason !== 'string') ||
    JSON.stringify(facts.reasons) !== JSON.stringify(owner.need.reasons)) return null
  if (facts.status === 'unknown' ? facts.score !== null :
    !Number.isInteger(facts.score) || facts.score! < -2147483648 || facts.score! > 2147483647) return null
  return facts
}

function evidenceKey(row: RxRow, facts: RxNeeds): string {
  const context = row.context, authority = facts.authority!, owner = facts.owner!
  return JSON.stringify([context.worker_instance, context.worker_epoch, context.server_instance,
    context.workspace_id, context.decoder_mode, routeKey(context.route), row.sender_callsign,
    facts.generated_at_unix_ms, facts.max_age_ms, authority.logbook_revision, authority.classification_revision,
    authority.country_reference_revision, authority.policy_revision, owner.query_id,
    owner.normalized_callsign, owner.dial_frequency_hz, owner.decoder_mode, owner.classification_mode,
    owner.band, owner.country, owner.adif, owner.country_resolution, owner.call_status, owner.dxcc_status,
    owner.need.status, owner.need.score, owner.need.reason_code, owner.need.reasons])
}

export function receiveNeeds(previous: NeedsLeases, view: RxView, requestStarted: number, identity?: OwnerIdentity): NeedsLeases {
  const next: NeedsLeases = new Map()
  for (const row of view.rows.slice(-200)) {
    const old = previous.get(row.decode_id)
    const facts = validNeeds(row, view, identity)
    if (!facts) {
      // Keep a tombstone until trimming/context replacement: an old fact cannot
      // become fresh again after a revoked snapshot or a transient IPC failure.
      if (old) next.set(row.decode_id, { ...old, deadline: 0 })
      continue
    }
    const evidence = evidenceKey(row, facts)
    const deadline = requestStarted + facts.remaining_valid_ms
    next.set(row.decode_id, { evidence, deadline: old?.evidence === evidence ? Math.min(old.deadline, deadline) : deadline })
  }
  return next
}

export function revokeNeeds(previous: NeedsLeases): NeedsLeases {
  return new Map([...previous].map(([id, lease]) => [id, { ...lease, deadline: 0 }]))
}

export function currentNeeds(row: RxRow, view: Pick<RxView, 'route' | 'mode'>, leases: NeedsLeases, now: number, identity?: OwnerIdentity): RxNeeds | null {
  const facts = validNeeds(row, view, identity), lease = leases.get(row.decode_id)
  return facts && lease && lease.deadline > now && lease.evidence === evidenceKey(row, facts) ? facts : null
}

const unknownReasons: Record<string, string> = {
  native_unclassified: 'Awaiting xLog classification.', native_capability_absent: 'xLog needs lookup unavailable.',
  native_not_literal_sender: 'No unambiguous sender callsign.', native_stale: 'xLog evidence expired.',
  native_lookup_unavailable: 'xLog needs lookup unavailable.', unavailable: 'xLog classification unavailable.',
  native_write_pending: 'Awaiting the outcome of a queued logbook change.',
  refreshing: 'xLog is refreshing its logbook evidence.', uncertain: 'xLog is resolving a pending logbook change.',
  stale: 'xLog evidence expired.', country_ambiguous: 'Country is ambiguous in xLog.',
  country_unknown: 'Country is unknown in xLog.', known_no_entity: 'No DXCC entity applies.', unsupported_band: 'Band is unavailable for this lookup.',
}
export function unknownReason(code: string | null | undefined): string {
  return unknownReasons[code ?? ''] ?? 'No current xLog classification.'
}
