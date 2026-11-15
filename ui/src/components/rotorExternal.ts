/** Owner observations are not native rotator settings, satellite state or a
 * movement acknowledgement. Stop admission is independent of read freshness. */
export interface ExternalRotorPresentation {
  readonly kind: 'xlog'
  readonly targetKey: string | null
  readonly observation: { readonly azimuthDeg: number | null; readonly moving: boolean | null; readonly deadline: number } | null
  readonly observationCurrent: () => boolean
  readonly canStop: () => boolean
  readonly stop: (() => Promise<void>) | null
}
