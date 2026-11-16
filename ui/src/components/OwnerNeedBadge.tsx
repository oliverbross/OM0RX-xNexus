// SPDX-License-Identifier: GPL-3.0-only
import type { ExternalDecodeEntry } from '../decodeHistory'
import { t } from '../i18n'
import { unknownReason } from '../xlog/rxNeeds'

/** Classifications are owner data, independently useful when entity resolution
 * is unknown. Never turn a missing answer into an unworked callsign. */
export function OwnerNeedBadge({ facts }: { facts: ExternalDecodeEntry['external'] }) {
  const call = facts.callStatus && facts.callStatus !== 'Unknown' ? facts.callStatus : null
  const dxcc = facts.dxccStatus && facts.dxccStatus !== 'Unknown' ? facts.dxccStatus : null
  const fallback = facts.status === 'needed' ? t('xlog.need.needed')
    : facts.status === 'worked' ? t('xlog.need.worked') : t('xlog.need.unknown')
  const title = ['xLog', call, facts.dxccStatus && `DXCC ${facts.dxccStatus}`,
    [facts.band, facts.classificationMode].filter(Boolean).join(' '), ...facts.reasons,
    facts.status === 'unknown' ? unknownReason(facts.reason) : facts.reason].filter(Boolean).join(' · ')
  const color = call === 'New Band' ? 'need-band' : call === 'New Mode' ? 'need-mode'
    : facts.status === 'needed' ? 'need-entity' : ''
  return <>
    <span className={`need-chip ${color}`} title={title}>
      {call ?? fallback}{facts.score != null ? ` · ${facts.score}` : ''}
    </span>
    {dxcc && dxcc !== call && <span className={`need-chip ${dxcc === 'ATNO' ? 'need-entity' : ''}`} title={title}>
      {`DXCC ${dxcc}`}
    </span>}
  </>
}
