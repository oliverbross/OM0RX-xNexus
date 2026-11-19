// SPDX-License-Identifier: GPL-3.0-only
// OM0RX-xNexus: the CS and DS columns (call status and DXCC status in the operator's own Nexus
// log), drawn in OM0RX-xLog's short codes and colours so a station reads the same in both apps.
// The statuses come from the hot index (crates/tempo-core logbook::hot `call_status` /
// `dxcc_status`) on each decode row and roster station; nothing here classifies a station.
import { t } from '../i18n'

/** xLog's status colours (its cluster defaults, from Wavelog's band map). */
export const LOG_STATUS_COLORS: Readonly<Record<string, string>> = {
  'New Call': '#FF6B35', 'New Band': '#00BFFF', 'New Mode': '#FFC000', 'New Band+Mode': '#E91E63',
  ATNO: '#FF8C00', 'C/New Band': '#4A9EFF', 'C/New Mode': '#FFD93D', 'C/New Band+Mode': '#9C27B0',
  'W/New Band': '#00BCD4', 'W/New Mode': '#FFA726', 'W/New Band+Mode': '#FF5722',
  Worked: '#B0B0B0', Confirmed: '#10B981', Unknown: '#777F8C',
}

const CODES: Readonly<Record<string, string>> = {
  'New Call': 'NC', 'New Band': 'NB', 'New Mode': 'NM', 'New Band+Mode': 'NB+M',
  ATNO: 'ATNO', 'C/New Band': 'NB/C', 'C/New Mode': 'NM/C', 'C/New Band+Mode': 'NB+NM/C',
  'W/New Band': 'NB/W', 'W/New Mode': 'NM/W', 'W/New Band+Mode': 'NB+NM/W',
  Worked: 'W', Confirmed: 'C', Unknown: '?',
}

/** The short code xLog's cluster shows for a status (an unrecognised status shows as itself). */
export function logStatusCode(status: string | null | undefined): string {
  if (!status) return '?'
  return CODES[status] ?? status
}

/** The colour xLog shows for a status; anything unrecognised takes xLog's Unknown colour. */
export function logStatusColor(status: string | null | undefined): string {
  return (status && LOG_STATUS_COLORS[status]) || LOG_STATUS_COLORS.Unknown
}

/** CS or DS from the operator's own logbook: the short code in the status colour. An absent
 * status (off the bands, own TX, no entity) leaves the cell empty so the columns stay aligned and
 * nothing reads as unworked. */
export function StatusCell({ kind, status }: { kind: 'CS' | 'DS'; status?: string | null }) {
  const cls = `log-status log-status-${kind.toLowerCase()}`
  if (!status) return <span className={cls} />
  return (
    <span
      className={cls}
      data-log-status={status}
      style={{ color: logStatusColor(status) }}
      title={`${kind === 'CS' ? t('logStatus.cs') : t('logStatus.ds')}: ${status}`}
      aria-label={`${kind} ${status}`}
    >
      {logStatusCode(status)}
    </span>
  )
}
