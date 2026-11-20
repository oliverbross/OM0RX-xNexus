// SPDX-License-Identifier: GPL-3.0-only
// OM0RX-xNexus: the CS / DS status filter — which call statuses and DXCC statuses (in the
// operator's own Nexus log, the CS and DS columns) a pane shows. The same two boxes as
// OM0RX-xLog's cluster filter, so "only new calls" reads the same in both apps.
//
// Stored as the statuses the operator UNTICKED, never the ticked ones: an empty value is "show
// everything", so a status Nexus learns later is shown until the operator hides it, and a value
// that fails to parse can only ever show more, never hide rows with no ticked box to explain why.
//
// Per surface, like every Operate view filter (operateFilters.ts): a popped-out Operate window
// keeps its own choice, and inherits the main window's until it makes one.
import { useCallback, useEffect, useState } from 'react'
import { surfaceGet, surfaceSet } from './features/windowScope'

/** Call status values, in xLog's order. `Unknown` = Nexus gave no status (off the bands, no call). */
export const CS_VALUES = [
  'New Call', 'New Band', 'New Mode', 'New Band+Mode', 'Worked', 'Confirmed', 'Unknown',
] as const

/** DXCC status values, in xLog's order. `Unknown` = no status (off the bands, or no entity). */
export const DS_VALUES = [
  'ATNO', 'C/New Band', 'C/New Mode', 'C/New Band+Mode', 'W/New Band', 'W/New Mode',
  'W/New Band+Mode', 'Worked', 'Confirmed', 'Unknown',
] as const

export interface LogStatusFilter {
  /** Call statuses hidden. */
  hideCs: string[]
  /** DXCC statuses hidden. */
  hideDs: string[]
}

export const SHOW_ALL: LogStatusFilter = { hideCs: [], hideDs: [] }

/** One-click choices. Each REPLACES the whole filter. */
export const LOG_STATUS_PRESETS = {
  all: SHOW_ALL,
  /** Calls not yet worked on this band and mode. */
  newCalls: { hideCs: ['Worked', 'Confirmed'], hideDs: [] },
  /** A DXCC still needed here: ATNO or a new band / mode slot. */
  newDxcc: { hideCs: [], hideDs: ['Worked', 'Confirmed', 'Unknown'] },
  /** Anything not yet confirmed. */
  unconfirmed: { hideCs: ['Confirmed'], hideDs: ['Confirmed'] },
} as const satisfies Record<string, LogStatusFilter>

export type LogStatusPreset = keyof typeof LOG_STATUS_PRESETS

export const DECODE_LOG_STATUS_KEY = 'nexus.decodes.logStatus'
export const ROSTER_LOG_STATUS_KEY = 'nexus.roster.logStatus'

/** Same-window change signal (detail = { key, value }): Band Activity can be mounted twice (a
 *  docked and a floating copy), and both must follow one choice live. */
export const LOG_STATUS_EVENT = 'nexus-log-status-filter-changed'

const keep = (raw: unknown, allowed: readonly string[]): string[] =>
  Array.isArray(raw) ? [...new Set(raw.filter((v): v is string => typeof v === 'string' && allowed.includes(v)))] : []

/** Read a stored filter; anything unreadable or unknown is dropped, which can only show more. */
export function parseLogStatusFilter(raw: string | null | undefined): LogStatusFilter {
  if (!raw) return { hideCs: [], hideDs: [] }
  try {
    const o = (JSON.parse(raw) ?? {}) as Partial<LogStatusFilter>
    return { hideCs: keep(o.hideCs, CS_VALUES), hideDs: keep(o.hideDs, DS_VALUES) }
  } catch {
    return { hideCs: [], hideDs: [] }
  }
}

export function loadLogStatusFilter(key: string): LogStatusFilter {
  return parseLogStatusFilter(surfaceGet(key))
}

export function saveLogStatusFilter(key: string, f: LogStatusFilter): void {
  surfaceSet(key, JSON.stringify(f))
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent(LOG_STATUS_EVENT, { detail: { key, value: f } }))
  }
}

/** How many statuses are hidden: what the chip counts. */
export function hiddenCount(f: LogStatusFilter): number {
  return f.hideCs.length + f.hideDs.length
}

export function isFiltering(f: LogStatusFilter): boolean {
  return hiddenCount(f) > 0
}

/** Does a row with this CS and DS pass? An absent status is `Unknown`. */
export function passesLogStatus(
  f: LogStatusFilter,
  callStatus: string | null | undefined,
  dxccStatus: string | null | undefined,
): boolean {
  if (f.hideCs.length === 0 && f.hideDs.length === 0) return true
  const cs = callStatus || 'Unknown'
  const ds = dxccStatus || 'Unknown'
  return !f.hideCs.includes(cs) && !f.hideDs.includes(ds)
}

/** Flip one status in or out of the hidden set. */
export function toggleStatus(f: LogStatusFilter, kind: 'cs' | 'ds', status: string): LogStatusFilter {
  const list = kind === 'cs' ? f.hideCs : f.hideDs
  const next = list.includes(status) ? list.filter((s) => s !== status) : [...list, status]
  return kind === 'cs' ? { ...f, hideCs: next } : { ...f, hideDs: next }
}

/** Which preset, if any, the filter is exactly. */
export function matchingPreset(f: LogStatusFilter): LogStatusPreset | null {
  const same = (a: readonly string[], b: readonly string[]) =>
    a.length === b.length && a.every((v) => b.includes(v))
  for (const [name, p] of Object.entries(LOG_STATUS_PRESETS)) {
    if (same(f.hideCs, p.hideCs) && same(f.hideDs, p.hideDs)) return name as LogStatusPreset
  }
  return null
}

/** The filter for one storage key, persisted, and live across every pane that uses that key. */
export function useLogStatusFilter(key: string): [LogStatusFilter, (f: LogStatusFilter) => void] {
  const [filter, setFilter] = useState<LogStatusFilter>(() => loadLogStatusFilter(key))
  useEffect(() => {
    const onChange = (e: Event) => {
      const d = (e as CustomEvent<{ key: string; value: LogStatusFilter }>).detail
      if (d && d.key === key) setFilter(d.value)
    }
    window.addEventListener(LOG_STATUS_EVENT, onChange)
    return () => window.removeEventListener(LOG_STATUS_EVENT, onChange)
  }, [key])
  const set = useCallback((f: LogStatusFilter) => saveLogStatusFilter(key, f), [key])
  return [filter, set]
}
