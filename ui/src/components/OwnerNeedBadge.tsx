// SPDX-License-Identifier: GPL-3.0-only
import type { CSSProperties } from 'react'
import type { ExternalDecodeEntry } from '../decodeHistory'
import { t } from '../i18n'
import { unknownReason } from '../xlog/rxNeeds'
import { xlogStatusCode, xlogStatusColor } from '../xlog/xlogStatus'

/** xLog's two cluster columns for this station: CS (Call status) and DS (DXCC status), each in
 * xLog's own short code and colour, so a station reads the same here as in xLog's cluster.
 * Classifications are owner data, independently useful when entity resolution is unknown.
 * Never turn a missing answer into an unworked callsign. The chip keeps Nexus's chip treatment
 * (the word in the theme's ink, the status colour on the border and tint) so it reads in every
 * theme; only the colour is xLog's. */
function factsTitle(facts: ExternalDecodeEntry['external']): string {
  return ['xLog',
    `CS ${facts.callStatus ?? 'Unknown'}`, `DS ${facts.dxccStatus ?? 'Unknown'}`,
    [facts.band, facts.classificationMode].filter(Boolean).join(' '),
    facts.score != null ? `score ${facts.score}` : '', ...facts.reasons,
    facts.status === 'unknown' ? unknownReason(facts.reason) : facts.reason].filter(Boolean).join(' · ')
}

/** One of the decode list's two xLog columns, CS or DS, as xLog's cluster draws it: the short code
 * in the status colour. A row xLog has not classified (or a row of our own) leaves the cell empty
 * or "?", so the columns stay aligned and a missing answer never reads as unworked. */
export function XlogStatusCell({ kind, facts }: { kind: 'CS' | 'DS'; facts?: ExternalDecodeEntry['external'] }) {
  const cls = `decode-xlog decode-${kind.toLowerCase()}`
  if (!facts) return <span className={cls} />
  const raw = kind === 'CS' ? facts.callStatus : facts.dxccStatus
  const status = raw && raw !== 'Unknown' ? raw : null
  return (
    <span
      className={cls}
      data-xlog-status={status ?? 'Unknown'}
      style={{ color: xlogStatusColor(status) }}
      title={factsTitle(facts)}
      aria-label={`${kind} ${status ?? 'Unknown'}`}
    >
      {xlogStatusCode(status)}
    </span>
  )
}

/** CS or DS from the operator's own logbook (the full app's decode lists and roster): the short
 * code in the status colour, xLog's scheme. An absent status (off the bands, own TX, no entity)
 * leaves the cell empty so the columns stay aligned and nothing reads as unworked. */
export function StatusCell({ kind, status }: { kind: 'CS' | 'DS'; status?: string | null }) {
  const cls = `decode-xlog decode-${kind.toLowerCase()}`
  if (!status) return <span className={cls} />
  return (
    <span
      className={cls}
      data-xlog-status={status}
      style={{ color: xlogStatusColor(status) }}
      title={`${kind === 'CS' ? t('logStatus.cs') : t('logStatus.ds')}: ${status}`}
      aria-label={`${kind} ${status}`}
    >
      {xlogStatusCode(status)}
    </span>
  )
}

export function OwnerNeedBadge({ facts }: { facts: ExternalDecodeEntry['external'] }) {
  const call = facts.callStatus && facts.callStatus !== 'Unknown' ? facts.callStatus : null
  const dxcc = facts.dxccStatus && facts.dxccStatus !== 'Unknown' ? facts.dxccStatus : null
  const fallback = facts.status === 'needed' ? t('xlog.need.needed')
    : facts.status === 'worked' ? t('xlog.need.worked') : t('xlog.need.unknown')
  const title = factsTitle(facts)
  if (!call && !dxcc) {
    return <span className="need-chip" title={title}>{fallback}</span>
  }
  const chip = (kind: 'CS' | 'DS', status: string | null) => (
    <span
      className="need-chip xlog-status"
      data-xlog-status={status ?? 'Unknown'}
      style={{ '--need-color': xlogStatusColor(status) } as CSSProperties}
      title={title}
      aria-label={`${kind} ${status ?? 'Unknown'}`}
    >
      {xlogStatusCode(status)}
    </span>
  )
  return <>
    {chip('CS', call)}
    {chip('DS', dxcc)}
  </>
}
