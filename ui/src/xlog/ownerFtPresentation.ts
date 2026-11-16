// SPDX-License-Identifier: GPL-3.0-only
import type { OperatingSnapshot } from '../components/operatingView'
import type { FtView } from './ftApi'
/** Original controls show native Station facts. Radio transmitting continues to
 * come from xLog's station observation, including another controller's TX. */
export function ownerFtPresentation(base: OperatingSnapshot, view: FtView, fresh: boolean): OperatingSnapshot {
  if (!view.connection_token) return base
  const hasQso = view.qso_state !== 'Listening' || !!view.dx_call || !!view.pending_message
  return { ...base,
    qso: hasQso ? { state: view.qso_state, dxcall: view.dx_call, dxgrid: view.dx_grid,
      rxReport: view.rx_report, running: hasQso, cqRunning: view.cq_running,
      txNow: view.pending_message, stalled: view.stalled, txCount: view.tx_count,
      watchdogStopped: view.watchdog_stopped } : null,
    radio: { ...base.radio, txEnabled: fresh && view.armed, txAllowed: fresh && view.available,
      txRefusal: fresh && view.available ? undefined : view.reason || 'Waiting for current xLog transmit context.',
      txEven: view.tx_even, txCycleAuto: view.tx_cycle_auto, holdTxFreq: view.hold,
      skipTx1: view.skip_tx1, txOffsetHz: view.offset_hz, txLevel: view.gain / .8,
      tuning: fresh ? view.tuning : null, txWatchdog: fresh ? view.watchdog_stopped : null },
  }
}
