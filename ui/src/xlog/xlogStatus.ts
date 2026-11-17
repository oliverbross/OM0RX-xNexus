// SPDX-License-Identifier: GPL-3.0-only
//! xLog's DX-cluster status vocabulary, so a station xLog classifies reads the same in Nexus as in
//! xLog's cluster: CS (Call status, this exact callsign) and DS (DXCC status, the entity), each with
//! its short code and colour. These mirror xLog's `prototype/src/live/clusterPalette.ts`
//! (`clusterStatusColors`, `clusterStatusCode`) and the band map's merged opportunity order
//! (`BandMaps.tsx` `opportunityStatus`). The statuses themselves are xLog's answer, sent over the
//! companion needs endpoint; nothing here classifies a station, it only presents the answer.

/** xLog's default status colours (Wavelog band-map defaults). */
export const XLOG_STATUS_COLORS: Readonly<Record<string, string>> = {
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
export function xlogStatusCode(status: string | null | undefined): string {
  if (!status) return '?'
  return CODES[status] ?? status
}

/** The colour xLog shows for a status; anything unrecognised takes xLog's Unknown colour. */
export function xlogStatusColor(status: string | null | undefined): string {
  return (status && XLOG_STATUS_COLORS[status]) || XLOG_STATUS_COLORS.Unknown
}

/** The one status a single mark shows, in xLog's band-map order: a needed entity (ATNO or a
 * C/ or W/ "New …") first, then a needed call ("New …"), then Confirmed, then Worked. */
export function xlogOpportunityStatus(
  callStatus: string | null | undefined,
  dxccStatus: string | null | undefined,
): string {
  const call = callStatus ?? ''
  const dxcc = dxccStatus ?? ''
  if (dxcc === 'ATNO' || dxcc.includes('/New ')) return dxcc
  if (call.startsWith('New ')) return call
  if (dxcc === 'Confirmed' || call === 'Confirmed') return 'Confirmed'
  if (dxcc === 'Worked' || call === 'Worked') return 'Worked'
  return dxcc || call || 'Unknown'
}

/** Whether a status asks the operator to work the station (the ones xLog's needs answer counts as
 * "needed"): any "New …", ATNO, or a C/ or W/ "New …". */
export function isXlogNeed(status: string | null | undefined): boolean {
  return !!status && (status === 'ATNO' || status.startsWith('New ') || status.includes('/New '))
}
