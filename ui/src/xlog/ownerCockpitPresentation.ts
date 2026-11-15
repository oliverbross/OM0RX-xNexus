// SPDX-License-Identifier: GPL-3.0-only
import { bandLabelForMhz } from '../band'
import type { OperatingSnapshot } from '../components/operatingView'
import type { OwnerOperatingModel } from './ownerOperatingModel'

/** Translate facts into the original cockpit's presentation contract, not the
 * native engine contract. The fields below explicitly separate local disabled
 * controls and display selection from unavailable owner measurements. */
export function ownerCockpitPresentation(model: OwnerOperatingModel, selectedCall: string | null): OperatingSnapshot {
  const periodSecs = model.decoderMode === 'FT4' ? 7.5 : 15
  const dialMhz = model.station?.frequency_hz == null ? null : model.station.frequency_hz / 1e6
  const lastPeriod = model.receive.rows[model.receive.rows.length - 1]?.host_period_start_unix_us
  return {
    backend: 'xlog',
    mycall: model.identity?.mycall ?? null, mygrid: model.identity?.mygrid ?? null,
    // The original FT operating screen is selected locally; this is not an
    // assertion that xLog is running Nexus's QSO sequencer or contest module.
    mode: 'qso', activePeer: selectedCall, qso: null, fieldDay: null,
    // Native engine buffers are not populated. Explicit external row props carry
    // the actual immutable owner entries, including unknown classifications.
    recentDecodes: [], stations: [], harqRescues: null,
    radio: {
      dialMhz, band: dialMhz == null ? null : bandLabelForMhz(dialMhz),
      sideband: model.station?.mode ?? null,
      transmitting: model.station?.transmitting ?? null,
      slot: lastPeriod == null ? null : Math.floor(lastPeriod / (periodSecs * 1e6)) + 1,
      nextSlotMs: null, timeSyncOk: null, rxLevel: null,
      // These are the local application controls: xNexus has no TX authority,
      // recording session or pending transmit. They do not claim radio-idle.
      txEnabled: false, txAllowed: false, txRefusal: model.transmit.reason,
      tuning: null, txWatchdog: null, decodeDepth: null, qsoRecording: false,
      txEven: null, rxOffsetHz: model.rxMarkerHz, txOffsetHz: null,
      holdTxFreq: false, txLevel: null,
      catOk: model.station?.connected ?? null,
      source: 'xlog', sourceLabel: 'xLog',
      clockOffsetMs: null, clockAgeSecs: null,
      clockOwnerNote: 'UTC synchronization and remote RF delay are unverified.',
    },
    link: {
      tier: model.decoderMode, periodSecs,
      snrDb: null, dtSec: null, freqHz: null, rv: null, quality: null,
      state: model.receive.phase,
    },
  }
}
