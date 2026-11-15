// SPDX-License-Identifier: GPL-3.0-only
import type { AppSnapshot, LinkState, RadioStatus, Station } from '../types'
import type { ExternalDecodeEntry } from '../decodeHistory'

/** Presentation contracts shared by the original cockpit and chrome. Native
 * snapshots remain structurally assignable and keep their existing semantics.
 * An external owner may explicitly lack an observation; null is never zero,
 * transmitter-idle, synchronized-clock, or an empty log. */
type Nullable<T, K extends keyof T> = Omit<T, K> & { [P in K]: T[P] | null }
export type OperatingRadio = Omit<Nullable<RadioStatus,
  'dialMhz' | 'band' | 'sideband' | 'transmitting' | 'slot' | 'nextSlotMs' | 'timeSyncOk' |
  'rxLevel' | 'tuning' | 'txWatchdog' | 'decodeDepth' | 'txEven' | 'txOffsetHz' | 'txLevel'>,
  'source'> & { source: RadioStatus['source'] | 'xlog' }
export type OperatingLink = Nullable<LinkState, 'snrDb' | 'dtSec' | 'freqHz' | 'rv' | 'quality'>
export type OperatingStation = Omit<Station, 'worked'> & {
  worked: boolean | null
  external?: ExternalDecodeEntry['external'] & { decodeId: string; isCq: boolean }
}
export type OperatingSnapshot = Omit<AppSnapshot,
  'radio' | 'link' | 'mycall' | 'mygrid' | 'aiCw' | 'conversations' | 'harqRescues' | 'stations'> & {
  readonly backend?: 'native' | 'xlog'
  mycall: string | null
  mygrid: string | null
  radio: OperatingRadio
  link: OperatingLink
  harqRescues: number | null
  stations: OperatingStation[]
}

export const reading = (value: number | null | undefined, digits = 0): string =>
  value == null || !Number.isFinite(value) ? '—' : value.toFixed(digits)

/** A positive on-air observation. Null does not certify that the transmitter is idle. */
export function observedOnAir(radio: Pick<OperatingRadio, 'transmitting' | 'txBusyReason' | 'rigKeyed'>): boolean | null {
  if (radio.transmitting === true || radio.txBusyReason != null || radio.rigKeyed === true) return true
  return radio.transmitting === false ? false : null
}
