// SPDX-License-Identifier: GPL-3.0-only
import type { ExternalSpotRow, ExternalSpotsSource } from '../components/spotsExternal'
import type { View } from './api'
import type { SnapshotLease } from './ownerOperatingModel'

export interface OwnerSpotsInput { view: View; lease: SnapshotLease | null; session: number }

/** Only the already authenticated snapshot is read. No owner request or local lookup. */
export function ownerSpotsSource(input: OwnerSpotsInput, latest: () => OwnerSpotsInput,
  select: (row: ExternalSpotRow) => void): ExternalSpotsSource {
  const { view, lease, session } = input
  const snapshot = lease?.snapshot
  const token = view.connection_token
  const valid = !!(view.paired && view.connected && token && view.hello?.capabilities.includes('cluster.read') && snapshot
    && view.snapshot === snapshot && snapshot.server_instance === view.hello.server_instance
    && snapshot.workspace_id === view.hello.workspace_id)
  const rows: ExternalSpotRow[] = valid ? snapshot!.spots.map(spot => ({
    id: spot.id, callsign: spot.callsign, frequencyHz: spot.frequency_hz, mode: spot.mode,
    observedAtUnixMs: spot.observed_at_unix_ms > 0 && spot.observed_at_unix_ms <= snapshot!.generated_at_unix_ms
      ? spot.observed_at_unix_ms : null,
    source: spot.source, country: spot.country ?? null, band: spot.band ?? null,
    assessment: spot.need.status === 'unknown'
      ? { status: 'unknown', explanation: spot.need.reasons.join(' · ') || null }
      : { status: spot.need.status, score: spot.need.score, reasons: [...spot.need.reasons] },
  })) : []
  const current = () => {
    const next = latest()
    return valid && next.view.hello !== null && next.session === session && next.view.connection_token === token
      && next.view.paired && next.view.connected && next.view.hello?.server_instance === snapshot?.server_instance
      && next.view.hello.capabilities.includes('cluster.read')
      && next.view.hello?.workspace_id === snapshot?.workspace_id
      && next.view.snapshot === snapshot && next.lease !== null && next.lease.snapshot === snapshot
      && next.lease.deadline === lease?.deadline
  }
  return {
    kind: 'xlog', key: JSON.stringify([session, token, snapshot?.server_instance, snapshot?.workspace_id]),
    state: valid ? 'current' : 'unavailable', deadline: valid ? lease!.deadline : null,
    generatedAtUnixMs: valid ? snapshot!.generated_at_unix_ms : null, rows, current,
    select: row => {
      const now = performance.now()
      if (Number.isFinite(now) && now >= 0 && current() && lease!.deadline > now && rows.includes(row)) select(row)
    },
  }
}
