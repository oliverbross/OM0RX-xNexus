// SPDX-License-Identifier: GPL-3.0-only
// OM0RX-xNexus: the supervised autopilot's control in the top bar, beside the Tx cycle — the
// on/off switch, what it is doing, the presence countdown with its "I'm here" button, and the
// recent decisions. Shown only when Settings ▸ Automation allows the autopilot (or Answer callers
// has callers waiting). Every decision is the engine's (engine/autopilot.rs); this only shows it
// and passes the operator's two acts: on/off and the presence confirmation.
//
// The presence prompt appears in the last minute of each interval, over everything, and stays
// until answered or until the engine turns transmit off.
import { useState } from 'react'
import { t } from '../i18n'
import type { AutomationState } from '../types'

const STATUS_KEYS = {
  off: { textKey: 'autopilot.status.off' },
  listening: { textKey: 'autopilot.status.listening' },
  calling: { textKey: 'autopilot.status.calling' },
  inQso: { textKey: 'autopilot.status.inQso' },
  moving: { textKey: 'autopilot.status.moving' },
  paused: { textKey: 'autopilot.status.paused' },
  noSchedule: { textKey: 'autopilot.status.noSchedule' },
  wrongMode: { textKey: 'autopilot.status.wrongMode' },
  presenceDue: { textKey: 'autopilot.status.presenceDue' },
  holding: { textKey: 'autopilot.status.holding' },
  finishing: { textKey: 'autopilot.status.finishing' },
  hunting: { textKey: 'autopilot.status.hunting' },
} as const

const TIER_KEYS = {
  callingMe: { textKey: 'autopilot.tier.callingMe' },
  atno: { textKey: 'autopilot.tier.atno' },
  dxpedition: { textKey: 'autopilot.tier.dxpedition' },
  dxccUnconfirmed: { textKey: 'autopilot.tier.dxccUnconfirmed' },
  dxccNewSlot: { textKey: 'autopilot.tier.dxccNewSlot' },
  usState: { textKey: 'autopilot.tier.usState' },
  reference: { textKey: 'autopilot.tier.reference' },
  newCall: { textKey: 'autopilot.tier.newCall' },
} as const

/** The status in words; an unknown token (a newer engine) reads as itself. */
export function autopilotStatusText(status: string): string {
  const k = STATUS_KEYS[status as keyof typeof STATUS_KEYS]
  return k ? t(k.textKey) : status
}

function mmss(secs: number): string {
  const s = Math.max(0, Math.floor(secs))
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`
}

/** The presence prompt opens in the last minute of the interval. */
export const PRESENCE_PROMPT_SECS = 60

export function AutopilotControl({
  automation,
  onSetAutopilot,
  onConfirmPresence,
}: {
  automation?: AutomationState | null
  onSetAutopilot: (on: boolean) => void
  onConfirmPresence: () => void
}) {
  const [logOpen, setLogOpen] = useState(false)
  if (!automation) return null
  const callers = automation.answerCallers ? automation.callers : []
  if (!automation.autopilotEnabled && callers.length === 0) return null
  const on = automation.autopilotOn
  const tierKey = automation.targetTier ? TIER_KEYS[automation.targetTier as keyof typeof TIER_KEYS]?.textKey : undefined
  const prompt =
    on && (automation.status === 'presenceDue' || automation.presenceRemainingSecs <= PRESENCE_PROMPT_SECS)
  return (
    <div className="topbar-group autopilot" role="group" aria-label={t('autopilot.aria')}>
      {automation.autopilotEnabled && (
        <button
          type="button"
          className={`tier-btn autopilot-toggle${on ? ' active' : ''}`}
          aria-pressed={on}
          onClick={() => onSetAutopilot(!on)}
          title={on ? t('autopilot.toggle.titleOn') : t('autopilot.toggle.titleOff')}
          data-testid="autopilot-toggle"
        >
          {on ? t('autopilot.toggle.on') : t('autopilot.toggle.off')}
        </button>
      )}
      {on && (
        <>
          <span className="autopilot-status" data-status={automation.status} title={automation.reason}>
            {autopilotStatusText(automation.status)}
            {automation.band ? ` · ${automation.band}` : ''}
            {automation.mode ? ` ${automation.mode}` : ''}
            {automation.target ? ` · ${automation.target}` : ''}
            {tierKey ? ` (${t(tierKey)})` : ''}
          </span>
          <button
            type="button"
            className={`tier-btn autopilot-presence${prompt ? ' due' : ''}`}
            onClick={onConfirmPresence}
            title={t('autopilot.presence.title', { minutes: automation.presenceMinutes })}
            data-testid="autopilot-presence"
          >
            {t('autopilot.presence.label', { time: mmss(automation.presenceRemainingSecs) })}
          </button>
          <button
            type="button"
            className="tier-btn autopilot-log-btn"
            aria-expanded={logOpen}
            onClick={() => setLogOpen((o) => !o)}
            title={t('autopilot.log.title')}
          >
            {t('autopilot.log.label', { count: automation.qsosLastHour })}
          </button>
        </>
      )}
      {callers.length > 0 && (
        <span className="autopilot-callers" title={t('autopilot.callers.title')}>
          {t('autopilot.callers.label', { calls: callers.join(' ') })}
        </span>
      )}
      {logOpen && on && (
        <div className="autopilot-log" role="log" aria-label={t('autopilot.log.title')}>
          {automation.nextWindow && (
            <div className="autopilot-log-next">{t('autopilot.log.next', { time: automation.nextWindow })}</div>
          )}
          {automation.log.length === 0 ? (
            <div className="autopilot-log-empty">{t('autopilot.log.empty')}</div>
          ) : (
            [...automation.log].reverse().map((line, i) => (
              <div key={`${i}-${line}`} className="autopilot-log-line">
                {line}
              </div>
            ))
          )}
        </div>
      )}
      {prompt && (
        <div className="autopilot-prompt" role="alertdialog" aria-label={t('autopilot.prompt.title')}>
          <div className="autopilot-prompt-title">{t('autopilot.prompt.title')}</div>
          <div className="autopilot-prompt-text">
            {t('autopilot.prompt.text', { time: mmss(automation.presenceRemainingSecs) })}
          </div>
          <button
            type="button"
            className="autopilot-prompt-btn"
            onClick={onConfirmPresence}
            data-testid="autopilot-prompt-confirm"
          >
            {t('autopilot.prompt.confirm')}
          </button>
        </div>
      )}
    </div>
  )
}
