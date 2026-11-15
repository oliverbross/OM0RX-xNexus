/** Owner observations are not native rotator settings, satellite state or a
 * movement acknowledgement. Stop admission is independent of read freshness. */
export interface ExternalBearingPresentation {
  readonly callsign: string
  readonly state: 'loading' | 'unavailable' | 'current'
  readonly facts: import('../xlog/bearingApi').BearingData | null
  readonly current: () => boolean
  readonly canRead: () => boolean
  readonly read: () => Promise<void>
  readonly canPoint: () => boolean
  readonly point: (longPath: boolean) => Promise<void>
  readonly evidence: import('../xlog/pointEvidence').PointEvidenceView | null
  readonly observationCurrent: () => boolean
}
export interface ExternalRotorPresentation {
  readonly kind: 'xlog'
  readonly targetKey: string | null
  readonly observation: { readonly azimuthDeg: number | null; readonly moving: boolean | null; readonly deadline: number } | null
  readonly observationCurrent: () => boolean
  readonly canStop: () => boolean
  readonly stop: (() => Promise<void>) | null
  readonly bearing?: ExternalBearingPresentation
}
