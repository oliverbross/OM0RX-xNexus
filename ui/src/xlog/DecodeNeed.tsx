// SPDX-License-Identifier: GPL-3.0-only
import type { RxNeeds } from './rxApi'
import { unknownReason } from './rxNeeds'

export function DecodeNeed({ facts, reason, remaining }: { facts: RxNeeds | null; reason?: string | null; remaining: number }) {
  const status = facts?.status ?? 'unknown', owner = facts?.owner
  return <div className="xn-decode-need">
    <strong className={`xn-need-status xn-need-${status}`}>{status === 'needed' ? 'Needed' : status === 'worked' ? 'Worked' : 'Unknown'}</strong>
    {facts && status !== 'unknown' && <span className="xn-mono">xLog score {facts.score}</span>}
    {status === 'unknown' && <small>{unknownReason(facts?.reason_code ?? reason)}</small>}
    {owner && facts && <details><summary>xLog evidence</summary>
      {facts.reasons.map((reason, index) => <p key={`${index}-${reason}`}>{reason}</p>)}
      <p>{owner.band ?? 'Band unknown'} · {owner.decoder_mode} · xLog {owner.classification_mode}</p>
      <p>{owner.country ?? 'Country unknown'}{owner.adif != null ? ` · DXCC ${owner.adif}` : ''}</p>
      <p>Call: {owner.call_status} · DXCC: {owner.dxcc_status}</p>
      <p>Owner time {new Date(facts.generated_at_unix_ms!).toISOString()}<br />Evidence expires in at most {(Math.max(0, remaining) / 1000).toFixed(1)} s.</p>
    </details>}
  </div>
}
