// SPDX-License-Identifier: GPL-3.0-only
// The original cockpit's external read model. This is deliberately not an
// AppSnapshot: xLog does not run Nexus's sequencer, CAT engine or station feeds.
import type { Snapshot, StationIdentity, View } from './api'
import type { RxMode, RxView } from './rxApi'
import type { NeedsLeases } from './rxNeeds'

export interface SnapshotLease {
  readonly snapshot: Snapshot
  readonly deadline: number
}

/** The IPC request start is the conservative origin, never response arrival. */
export function leaseSnapshot(view: View, requestStarted: number, wallNow: number): SnapshotLease | null {
  const snapshot = view.snapshot, hello = view.hello
  if (!view.connected || !view.paired || !snapshot || !hello ||
    snapshot.server_instance !== hello.server_instance || snapshot.workspace_id !== hello.workspace_id ||
    !Number.isFinite(requestStarted) || !Number.isFinite(wallNow) ||
    !Number.isSafeInteger(snapshot.generated_at_unix_ms) ||
    !Number.isSafeInteger(snapshot.max_age_ms) || snapshot.max_age_ms <= 0) return null
  const age = wallNow - snapshot.generated_at_unix_ms
  if (age < 0 || age >= snapshot.max_age_ms) return null
  return { snapshot, deadline: requestStarted + snapshot.max_age_ms - age }
}

export interface OwnerOperatingModel {
  readonly kind: 'xlog'
  readonly session: number
  readonly owner: { server_instance: string; workspace_id: string } | null
  readonly fresh: boolean
  /** Saved station settings, not RF identity or permission to transmit. */
  readonly identity: StationIdentity | null
  readonly station: Snapshot['station'] | null
  readonly rotor: Snapshot['rotor'] | null
  readonly spots: Snapshot['spots']
  readonly receive: RxView
  readonly needsLeases: NeedsLeases
  /** A local decoder selection, independent of the rig's DIGU/USB mode. */
  readonly decoderMode: RxMode
  readonly rxMarkerHz: number
  readonly transmit: { readonly available: false; readonly reason: string }
  readonly timing: { readonly utcSyncAccuracyUs: null; readonly remoteRfLatencyUs: null }
}

export function ownerOperatingModel(args: {
  view: View; lease: SnapshotLease | null; now: number; session: number
  receive: RxView; needsLeases: NeedsLeases; decoderMode: RxMode; rxMarkerHz: number
}): OwnerOperatingModel {
  const { view, lease, now, session, receive, needsLeases, decoderMode, rxMarkerHz } = args
  const owner = view.hello && view.paired ? {
    server_instance: view.hello.server_instance, workspace_id: view.hello.workspace_id,
  } : null
  const snapshot = lease?.snapshot
  const fresh = !!(view.connected && owner && snapshot && lease.deadline > now &&
    view.snapshot === snapshot && snapshot.server_instance === owner.server_instance &&
    snapshot.workspace_id === owner.workspace_id)
  const identity = fresh && snapshot?.station_identity?.state !== 'unavailable' &&
    snapshot?.station_identity?.binding?.workspace_generation === owner?.workspace_id
    ? snapshot.station_identity ?? null : null
  return {
    kind: 'xlog', session, owner, fresh, identity,
    station: fresh ? snapshot!.station : null,
    rotor: fresh ? snapshot!.rotor : null,
    spots: fresh ? snapshot!.spots : [],
    receive, needsLeases, decoderMode, rxMarkerHz,
    transmit: { available: false, reason: 'Transmit is not available through this xLog connection.' },
    timing: { utcSyncAccuracyUs: null, remoteRfLatencyUs: null },
  }
}
