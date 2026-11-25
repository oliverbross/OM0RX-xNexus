// SPDX-License-Identifier: GPL-3.0-only
// OM0RX-xNexus: the need board's alerts as the autopilot's feed — the facts a decode cannot carry
// (a DXpedition, a park or island not yet worked, an entity or slot needed, the band and mode a
// spot names). Data only: the engine (engine/autopilot.rs) ranks and decides.
import type { AutopilotFeedEntry, NeedAlert } from '../types'

export function autopilotFeedFromNeeds(alerts: readonly NeedAlert[]): AutopilotFeedEntry[] {
  const out = new Map<string, AutopilotFeedEntry>()
  for (const a of alerts) {
    if (!a.call || !a.band || a.licensed === false) continue
    const tags = new Set(a.tags)
    const entry: AutopilotFeedEntry = {
      call: a.call.toUpperCase(),
      band: a.band,
      mode: a.exactMode ?? null,
      freqMhz: a.freqMhz ?? null,
      dxpedition: tags.has('Dxped'),
      newReference: tags.has('NewPark') || tags.has('NewIota'),
      newEntity: tags.has('NewEntity'),
      // A new mode alone is not a new band slot: the calling rules count FT4 with FT8, so a station
      // hunted for it was never called. A DXpedition is still hunted for one.
      newSlot: tags.has('NewBand') || (tags.has('NewMode') && tags.has('Dxped')),
      newState: tags.has('NewState'),
      state: a.state ?? null,
    }
    // One row per call, band and mode: keep the one that says the most (a needed entity over a
    // plain spot).
    const key = `${entry.call}|${entry.band}|${entry.mode ?? ''}`
    const have = out.get(key)
    const weight = (e: AutopilotFeedEntry) =>
      (e.newEntity ? 16 : 0) +
      (e.dxpedition ? 8 : 0) +
      (e.newSlot ? 4 : 0) +
      (e.newState ? 2 : 0) +
      (e.newReference ? 1 : 0)
    if (!have || weight(entry) > weight(have)) out.set(key, entry)
  }
  return [...out.values()]
}
