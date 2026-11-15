// External observations are not native SpotRow/award/licence/propagation facts.
export type ExternalSpotAssessment =
  | { readonly status: 'needed' | 'worked'; readonly score: number; readonly reasons: readonly string[] }
  | { readonly status: 'unknown'; readonly explanation: string | null }

export interface ExternalSpotRow {
  readonly id: string
  readonly callsign: string
  readonly frequencyHz: number
  readonly mode: string
  readonly observedAtUnixMs: number | null
  readonly source: string
  readonly country: string | null
  readonly band: string | null
  readonly assessment: ExternalSpotAssessment
}

/** No IO in current(): the host checks its latest connection/snapshot identity.
 * Deadline is inherited from the original poll, never renewed by this board. */
export interface ExternalSpotsSource {
  readonly kind: 'xlog'
  readonly key: string
  readonly state: 'current' | 'unavailable'
  readonly deadline: number | null
  readonly generatedAtUnixMs: number | null
  readonly rows: readonly ExternalSpotRow[]
  readonly current: () => boolean
  readonly select: (row: ExternalSpotRow) => void
}

export const EXTERNAL_SPOTS_CAPABILITIES = {
  pounceFilter: false, entityCentroids: false, callCard: false, rowMenu: false,
  postSpot: false, popOut: false, privileges: false, reporter: false,
  workedDates: false, states: false, awards: false,
} as const

export function externalSpotsCurrent(source: ExternalSpotsSource, now: number): boolean {
  return source.state === 'current' && source.deadline !== null && Number.isFinite(now) && now >= 0
    && Number.isFinite(source.deadline) && now < source.deadline && source.current()
}

export function selectExternalSpot(source: ExternalSpotsSource, row: ExternalSpotRow, now: number): boolean {
  if (!externalSpotsCurrent(source, now) || !source.rows.includes(row)) return false
  source.select(row)
  return true
}

/** Display age only; never substitutes for the original monotonic source lease. */
export function externalSpotAge(row: ExternalSpotRow, generatedAtUnixMs: number | null, wallNow: number): number {
  const observed = row.observedAtUnixMs
  if (observed === null || generatedAtUnixMs === null || observed <= 0 || observed > generatedAtUnixMs) return -1
  const elapsed = wallNow - observed
  return Number.isFinite(elapsed) && elapsed >= 0 ? Math.floor(elapsed / 1000) : -1
}
