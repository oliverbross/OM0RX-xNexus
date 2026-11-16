// SPDX-License-Identifier: GPL-3.0-only
import type { ExternalDecodeEntry } from '../decodeHistory'
import type { OperatingStation } from '../components/operatingView'
import { gridFromMessage } from '../txMessages'
import type { RxView } from './rxApi'
import { currentNeeds, type NeedsLeases, type OwnerIdentity } from './rxNeeds'

/** Preserve each native decode ID and its exact host-period timestamp. Owner
 * need classification is a separate fact, never a substitute for decode quality
 * or Nexus's HARQ/redundancy version, nor a fabricated per-contact B4 answer. */
export function ownerDecodeEntries(args: {
  view: RxView; leases: NeedsLeases; now: number; owner: OwnerIdentity | null
  needsAvailable: boolean; mycall: string | null
}): ExternalDecodeEntry[] {
  const { view, leases, now, owner, needsAvailable, mycall } = args
  return view.rows.slice(-200).map(row => {
    const facts = needsAvailable && owner ? currentNeeds(row, view, leases, now, owner) : null
    const periodStartMs = row.host_period_start_unix_us / 1000
    const periodMs = row.mode === 'FT4' ? 7500 : 15000
    const words = row.message.trim().toUpperCase().split(/\s+/)
    return {
      id: row.decode_id,
      // Slot is a display sort/group key only. rowTimeMs uses the exact external
      // period, not the upstream slot-minus-one inference.
      slot: Math.floor(periodStartMs / periodMs), at: periodStartMs,
      from: row.sender_callsign, snr: row.snr_db, dtSec: row.time_offset_seconds,
      freqHz: row.audio_frequency_hz, message: row.message, tier: row.mode,
      isCq: words[0] === 'CQ',
      directedToMe: mycall ? words[0] === mycall.toUpperCase() : null,
      worked: facts?.owner && facts.owner.call_status !== 'Unknown'
        ? facts.owner.call_status !== 'New Call' : null,
      // Only the owner states that prove this band are mapped. New Band+Mode
      // may also mean a new combination of separately worked band and mode.
      workedBand: facts?.owner && ['Worked', 'Confirmed', 'New Mode'].includes(facts.owner.call_status)
        ? true : facts?.owner?.call_status === 'New Band' ? false : undefined,
      newDxcc: facts?.owner?.dxcc_status === 'ATNO', rv: null,
      country: facts?.owner?.country ?? null,
      external: {
        periodStartMs,
        callStatus: facts?.owner?.call_status ?? null,
        dxccStatus: facts?.owner?.dxcc_status ?? null,
        classificationMode: facts?.owner?.classification_mode ?? null,
        band: facts?.owner?.band ?? null,
        status: facts?.status ?? 'unknown', score: facts?.score ?? null,
        reasons: facts?.reasons ?? [],
        reason: facts?.reason_code ?? (!needsAvailable ? 'Needs lookup unavailable' :
          row.needs?.owner ? 'Owner classification expired' : row.needs?.reason_code ?? 'Not classified'),
      },
    }
  })
}

/** The original roster is one row per call. Aggregation covers this bounded RX
 * window only; the exact most-recent decode ID remains attached to its facts. */
export function ownerRoster(entries: readonly ExternalDecodeEntry[]): OperatingStation[] {
  const stations = new Map<string, OperatingStation>()
  for (const entry of entries) {
    if (!entry.from) continue
    const old = stations.get(entry.from)
    const count = (old?.heardCount ?? 0) + 1
    if (old && old.lastHeardSlot > entry.slot) { old.heardCount = count; continue }
    stations.set(entry.from, {
      call: entry.from, grid: gridFromMessage(entry.message) || null,
      snr: entry.snr, lastHeardSlot: entry.slot, heardCount: count,
      // Presence names a retained decode in the current RX route, not a live
      // radio or propagation prediction. Original recency filtering still runs.
      presence: 'active', worked: entry.worked, workedBand: entry.workedBand, country: entry.country, tier: entry.tier,
      freqHz: entry.freqHz,
      external: { ...entry.external, decodeId: entry.id, isCq: entry.isCq },
    })
  }
  return [...stations.values()]
}
