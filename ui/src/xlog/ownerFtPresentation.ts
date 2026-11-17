// SPDX-License-Identifier: GPL-3.0-only
import type { OperatingSnapshot } from '../components/operatingView'
import type { FtView } from './ftApi'
export function ownerTxRefusal(reason: string): string {
  const messages: Record<string, string> = {
    local_tx_disabled: 'xLog transmit service is unavailable.',
    commissioned_tx_route_unavailable: 'Waiting for xLog’s current FLEX and DAX TX connection.',
    confirmed_station_identity_unavailable: 'Your saved xLog callsign and locator are unavailable.',
    worker_tx_lease_unsupported: 'Update the xLog station worker before enabling digital TX.',
    pairing_revoked: 'Waiting for the automatic local xLog connection.',
  }
  return messages[reason] ?? (reason || 'Waiting for current xLog transmit context.')
}
/** Original controls show native Station facts. Radio transmitting continues to
 * come from xLog's station observation, including another controller's TX. */
export function ownerFtPresentation(base: OperatingSnapshot, view: FtView, fresh: boolean, commandAvailable = fresh && view.available): OperatingSnapshot {
  if (!view.connection_token && !commandAvailable) return base
  const hasQso = view.qso_state !== 'Listening' || !!view.dx_call || !!view.pending_message
  return { ...base,
    qso: hasQso ? { state: view.qso_state, dxcall: view.dx_call, dxgrid: view.dx_grid,
      rxReport: view.rx_report, running: hasQso, cqRunning: view.cq_running,
      txNow: view.pending_message, stalled: view.stalled, txCount: view.tx_count,
      watchdogStopped: view.watchdog_stopped } : null,
    radio: { ...base.radio, txEnabled: commandAvailable && view.armed, txAllowed: commandAvailable,
      txRefusal: commandAvailable ? undefined : ownerTxRefusal(view.reason),
      txEven: view.tx_even, txCycleAuto: view.tx_cycle_auto, holdTxFreq: view.hold,
      skipTx1: view.skip_tx1, txOffsetHz: view.offset_hz, txLevel: view.gain / .8,
      tuning: fresh ? view.tuning : null, txWatchdog: fresh ? view.watchdog_stopped : null },
  }
}
