// SPDX-License-Identifier: GPL-3.0-only
// OM0RX-xNexus: Settings ▸ Autopilot — the supervised autopilot and Answer callers, on their own tab
// (operator, 2026-11-22: "create a special area for autopilot in settings, so all will be in one
// place … allow the user to customise rules and prioritise them"). One object
// (`settings.automation`) edited whole, as Js8PaneStyleSettings edits its own. The engine clamps
// every number again on use (automation.rs `sanitized`), so a stray value can never wedge it; this
// form clamps too, so what it shows is what will run.
import { useState } from 'react'
import { t } from '../i18n'
import type {
  AutomationSettings,
  AutopilotRule,
  AutopilotTiers,
  AutopilotWindow,
  Settings,
} from '../types'

/** The rules in the engine's default order (automation.rs `DEFAULT_PRIORITY`). */
export const DEFAULT_PRIORITY: AutopilotRule[] = [
  'atno',
  'mostWanted',
  'dxpedition',
  'dxccUnconfirmed',
  'dxccNewSlot',
  'usState',
  'reference',
  'grid',
  'newCall',
]

/** The engine's defaults (automation.rs `AutomationSettings::default`). */
export const DEFAULT_AUTOMATION: AutomationSettings = {
  answerCallers: false,
  answerTakeoverSlots: 2,
  answerChainLimit: 3,
  autopilotEnabled: false,
  presenceCheckMin: 10,
  schedule: [
    { start: '07:00', bands: ['20m', '17m', '15m', '12m', '10m'], mode: 'FT8' },
    { start: '20:00', bands: ['160m', '80m', '40m', '30m', '20m'], mode: 'FT8' },
  ],
  listenSlots: 2,
  minDwellSecs: 180,
  quietSecs: 300,
  ordinaryDwellSecs: 600,
  targetBudgetSecs: 120,
  rareBudgetSecs: 900,
  maxQsoPerHour: 60,
  tiers: {
    atno: true,
    dxpedition: true,
    dxccUnconfirmed: true,
    dxccNewSlot: true,
    usState: false,
    reference: true,
    newCall: true,
    mostWanted: true,
    grid: true,
  },
  chaseSpots: true,
  wantedStates: [],
  huntDxpeditions: true,
  huntHound: true,
  priority: DEFAULT_PRIORITY,
  preferOutsideEu: true,
  targetMaxCalls: 4,
  skipContinents: [],
  clearTxOffset: true,
}

/** The continents offered, in the order shown (cty.dat codes; automation.rs `CONTINENTS`). */
export const CONTINENTS = ['NA', 'SA', 'OC', 'AS', 'AF', 'EU'] as const

/**
 * The stored order made whole, as the engine makes it (automation.rs `sanitized_priority`): each
 * known rule once, in the operator's order, and a rule missing from it (one added by a later
 * release) put in after the rules that precede it in the default order.
 */
export function ruleOrder(stored: readonly string[] | null | undefined): AutopilotRule[] {
  const out: AutopilotRule[] = []
  for (const r of stored ?? []) {
    const rule = r as AutopilotRule
    if (DEFAULT_PRIORITY.includes(rule) && !out.includes(rule)) out.push(rule)
  }
  DEFAULT_PRIORITY.forEach((rule, i) => {
    if (out.includes(rule)) return
    const before = DEFAULT_PRIORITY.slice(0, i)
      .map((b) => out.indexOf(b))
      .filter((p) => p >= 0)
    out.splice(before.length ? Math.max(...before) + 1 : 0, 0, rule)
  })
  return out
}

/** `order` with the rule at `i` moved one place up (`-1`) or down (`+1`); unchanged at an edge. */
export function moveRule(order: AutopilotRule[], i: number, dir: -1 | 1): AutopilotRule[] {
  const j = i + dir
  if (i < 0 || i >= order.length || j < 0 || j >= order.length) return order
  const out = [...order]
  ;[out[i], out[j]] = [out[j], out[i]]
  return out
}

/** The stored value with every missing field filled from the defaults. */
export function automationOf(s: Pick<Settings, 'automation'> | null | undefined): AutomationSettings {
  const a = s?.automation
  return {
    ...DEFAULT_AUTOMATION,
    ...(a ?? {}),
    tiers: { ...DEFAULT_AUTOMATION.tiers, ...(a?.tiers ?? {}) },
    schedule: Array.isArray(a?.schedule) ? a!.schedule : DEFAULT_AUTOMATION.schedule,
    wantedStates: Array.isArray(a?.wantedStates) ? a!.wantedStates : [],
    skipContinents: Array.isArray(a?.skipContinents) ? a!.skipContinents : [],
    priority: ruleOrder(Array.isArray(a?.priority) ? a!.priority : DEFAULT_PRIORITY),
  }
}

/** Hold a typed number to a range; an empty or bad entry keeps the old value. */
export function clampInt(raw: string, lo: number, hi: number, old: number): number {
  const n = Math.round(Number(raw))
  if (raw.trim() === '' || !Number.isFinite(n)) return old
  return Math.min(hi, Math.max(lo, n))
}

/** A rule's words. Each label is a literal `t('…')` so the i18n guards read it (a key table
 * resolved at runtime would be unchecked). */
function ruleLabel(rule: AutopilotRule): string {
  switch (rule) {
    case 'atno': return t('settings.automation.tiers.atno')
    case 'mostWanted': return t('settings.automation.tiers.mostWanted')
    case 'dxpedition': return t('settings.automation.tiers.dxpedition')
    case 'dxccUnconfirmed': return t('settings.automation.tiers.dxccUnconfirmed')
    case 'dxccNewSlot': return t('settings.automation.tiers.dxccNewSlot')
    case 'usState': return t('settings.automation.tiers.usState')
    case 'reference': return t('settings.automation.tiers.reference')
    case 'grid': return t('settings.automation.tiers.grid')
    case 'newCall': return t('settings.automation.tiers.newCall')
  }
}

/** The rules called for the long budget and from 1200 Hz above (automation.rs `is_rare`). */
const RARE: ReadonlySet<AutopilotRule> = new Set<AutopilotRule>(['atno', 'mostWanted', 'dxpedition'])

function Switch({
  label,
  on,
  onChange,
  hint,
  testId,
}: {
  label: string
  on: boolean
  onChange: (on: boolean) => void
  hint?: string
  testId?: string
}) {
  return (
    <div className="settings-field">
      <label className="settings-toggle">
        <span className="settings-label">{label}</span>
        <button
          type="button"
          role="switch"
          aria-checked={on}
          className={`toggle${on ? ' on' : ''}`}
          onClick={() => onChange(!on)}
          data-testid={testId}
        >
          <span className="toggle-knob" />
        </button>
      </label>
      {hint && <span className="settings-hint">{hint}</span>}
    </div>
  )
}

function NumberField({
  label,
  value,
  lo,
  hi,
  onChange,
  hint,
  testId,
}: {
  label: string
  value: number
  lo: number
  hi: number
  onChange: (n: number) => void
  hint?: string
  testId?: string
}) {
  return (
    <label className="settings-field">
      <span className="settings-label">{label}</span>
      <input
        className="settings-input"
        type="number"
        inputMode="numeric"
        min={lo}
        max={hi}
        value={String(value)}
        onChange={(e) => onChange(clampInt(e.target.value, lo, hi, value))}
        autoComplete="off"
        data-testid={testId}
      />
      {hint && <span className="settings-hint">{hint}</span>}
    </label>
  )
}

/** Split a typed list of state codes ("vt, ND  me") into clean upper-case codes. */
export function parseStates(raw: string): string[] {
  const out: string[] = []
  for (const part of raw.split(/[\s,;]+/)) {
    const code = part.trim().toUpperCase()
    if (code && !out.includes(code)) out.push(code)
  }
  return out
}

function WantedStates({
  value,
  disabled,
  onChange,
}: {
  value: string[]
  disabled: boolean
  onChange: (states: string[]) => void
}) {
  // Typed freely and parsed on blur, so a half-typed code is not thrown away mid-word.
  const [draft, setDraft] = useState<string | null>(null)
  return (
    <label className="settings-field">
      <span className="settings-label">{t('settings.automation.tiers.states')}</span>
      <input
        className="settings-input"
        type="text"
        value={draft ?? value.join(', ')}
        placeholder={t('settings.automation.tiers.states.placeholder')}
        disabled={disabled}
        onChange={(e) => setDraft(e.target.value)}
        onBlur={() => {
          if (draft !== null) onChange(parseStates(draft))
          setDraft(null)
        }}
        autoComplete="off"
        spellCheck={false}
        data-testid="automation-wanted-states"
      />
      <span className="settings-hint">{t('settings.automation.tiers.states.hint')}</span>
    </label>
  )
}

/** A continent's name. Each label is a literal `t('…')`, as the rules' are. */
function continentLabel(code: (typeof CONTINENTS)[number]): string {
  switch (code) {
    case 'NA': return t('hideCountries.continent.na')
    case 'SA': return t('hideCountries.continent.sa')
    case 'OC': return t('hideCountries.continent.oc')
    case 'AS': return t('hideCountries.continent.as')
    case 'AF': return t('hideCountries.continent.af')
    case 'EU': return t('hideCountries.continent.eu')
  }
}

/** One checkbox per continent: ticked = called. */
function Continents({ skip, onChange }: { skip: string[]; onChange: (skip: string[]) => void }) {
  return (
    <fieldset className="settings-field automation-continents" data-testid="automation-continents">
      <legend className="settings-label">{t('settings.automation.pilot.continents')}</legend>
      <div className="automation-continents-row">
        {CONTINENTS.map((code) => (
          <label key={code}>
            <input
              type="checkbox"
              checked={!skip.includes(code)}
              onChange={(e) =>
                onChange(e.target.checked ? skip.filter((c) => c !== code) : [...skip, code])
              }
              data-testid={`automation-continent-${code}`}
            />
            {continentLabel(code)}
          </label>
        ))}
      </div>
      <span className="settings-hint">{t('settings.automation.pilot.continents.hint')}</span>
    </fieldset>
  )
}

/** The ordered rule list: a switch per rule and arrows to move it. */
function Rules({
  order,
  tiers,
  onOrder,
  onTier,
}: {
  order: AutopilotRule[]
  tiers: AutopilotTiers
  onOrder: (order: AutopilotRule[]) => void
  onTier: (rule: AutopilotRule, on: boolean) => void
}) {
  return (
    <ol className="automation-rules" data-testid="automation-rules">
      {order.map((rule, i) => {
        const label = ruleLabel(rule)
        const on = tiers[rule]
        return (
          <li
            key={rule}
            className={`automation-rule${on ? '' : ' off'}`}
            data-testid={`automation-rule-${rule}`}
          >
            <span className="automation-rule-rank" aria-hidden="true">
              {i + 1}
            </span>
            <button
              type="button"
              role="switch"
              aria-checked={on}
              aria-label={label}
              className={`toggle${on ? ' on' : ''}`}
              onClick={() => onTier(rule, !on)}
              data-testid={`automation-rule-${rule}-on`}
            >
              <span className="toggle-knob" />
            </button>
            <span className="automation-rule-label">
              {label}
              {RARE.has(rule) && (
                <span
                  className="automation-rule-rare"
                  title={t('settings.automation.rules.rare')}
                  aria-label={t('settings.automation.rules.rare')}
                >
                  ★
                </span>
              )}
            </span>
            <span className="automation-rule-move">
              <button
                type="button"
                className="automation-rule-btn"
                disabled={i === 0}
                onClick={() => onOrder(moveRule(order, i, -1))}
                aria-label={t('settings.automation.rules.up', { rule: label })}
                title={t('settings.automation.rules.up', { rule: label })}
                data-testid={`automation-rule-${rule}-up`}
              >
                ▲
              </button>
              <button
                type="button"
                className="automation-rule-btn"
                disabled={i === order.length - 1}
                onClick={() => onOrder(moveRule(order, i, 1))}
                aria-label={t('settings.automation.rules.down', { rule: label })}
                title={t('settings.automation.rules.down', { rule: label })}
                data-testid={`automation-rule-${rule}-down`}
              >
                ▼
              </button>
            </span>
          </li>
        )
      })}
    </ol>
  )
}

export function SettingsAutomation({
  form,
  onChange,
}: {
  form: Settings
  onChange: (automation: AutomationSettings) => void
}) {
  const a = automationOf(form)
  const set = (patch: Partial<AutomationSettings>) => onChange({ ...a, ...patch })
  const setWindow = (i: number, patch: Partial<AutopilotWindow>) =>
    set({ schedule: a.schedule.map((w, j) => (j === i ? { ...w, ...patch } : w)) })
  return (
    <div className="settings-automation" data-testid="settings-automation">
      <p className="settings-hint">{t('settings.automation.intro')}</p>

      <section className="automation-card">
        <h4 className="settings-subhead">{t('settings.automation.pilot.head')}</h4>
        <Switch
          label={t('settings.automation.pilot.on')}
          on={a.autopilotEnabled}
          onChange={(autopilotEnabled) => set({ autopilotEnabled })}
          testId="automation-autopilot-enabled"
        />
        <NumberField
          label={t('settings.automation.pilot.presence')}
          value={a.presenceCheckMin}
          lo={2}
          hi={999}
          onChange={(presenceCheckMin) => set({ presenceCheckMin })}
          hint={t('settings.automation.pilot.presence.hint')}
          testId="automation-presence-min"
        />
        <Switch
          label={t('settings.automation.pilot.chase')}
          on={a.chaseSpots}
          onChange={(chaseSpots) => set({ chaseSpots })}
        />
        <Switch
          label={t('settings.automation.pilot.hunt')}
          hint={t('settings.automation.pilot.hunt.hint')}
          on={a.huntDxpeditions}
          onChange={(huntDxpeditions) => set({ huntDxpeditions })}
          testId="automation-hunt"
        />
        <Switch
          label={t('settings.automation.pilot.huntHound')}
          on={a.huntHound}
          onChange={(huntHound) => set({ huntHound })}
          testId="automation-hunt-hound"
        />
        <Switch
          label={t('settings.automation.pilot.outsideEu')}
          hint={t('settings.automation.pilot.outsideEu.hint')}
          on={a.preferOutsideEu}
          onChange={(preferOutsideEu) => set({ preferOutsideEu })}
          testId="automation-outside-eu"
        />
        <Switch
          label={t('settings.automation.pilot.clearTx')}
          hint={t('settings.automation.pilot.clearTx.hint')}
          on={a.clearTxOffset}
          onChange={(clearTxOffset) => set({ clearTxOffset })}
          testId="automation-clear-tx"
        />
        <Continents
          skip={a.skipContinents}
          onChange={(skipContinents) => set({ skipContinents })}
        />
      </section>

      <section className="automation-card">
        <h4 className="settings-subhead">{t('settings.automation.rules.head')}</h4>
        <p className="settings-hint">{t('settings.automation.rules.hint')}</p>
        <Rules
          order={a.priority}
          tiers={a.tiers}
          onOrder={(priority) => set({ priority })}
          onTier={(rule, on) => set({ tiers: { ...a.tiers, [rule]: on } })}
        />
        <div className="automation-rules-actions">
          <button
            type="button"
            className="automation-window-add"
            onClick={() => set({ priority: DEFAULT_PRIORITY })}
            data-testid="automation-rules-reset"
          >
            {t('settings.automation.rules.reset')}
          </button>
        </div>
        <WantedStates
          value={a.wantedStates}
          disabled={!a.tiers.usState}
          onChange={(wantedStates) => set({ wantedStates })}
        />
      </section>

      <section className="automation-card">
        <h4 className="settings-subhead">{t('settings.automation.schedule.head')}</h4>
        <p className="settings-hint">{t('settings.automation.schedule.hint')}</p>
        <div className="automation-schedule">
          {a.schedule.map((w, i) => (
            <div className="automation-window" key={i}>
              <label>
                <span className="settings-label">{t('settings.automation.schedule.start')}</span>
                <input
                  className="settings-input"
                  type="time"
                  value={w.start}
                  onChange={(e) => setWindow(i, { start: e.target.value })}
                />
              </label>
              <label className="automation-window-bands">
                <span className="settings-label">{t('settings.automation.schedule.bands')}</span>
                <input
                  className="settings-input"
                  type="text"
                  value={w.bands.join(' ')}
                  spellCheck={false}
                  onChange={(e) =>
                    setWindow(i, { bands: e.target.value.split(/[\s,]+/).filter((b) => b !== '') })
                  }
                />
              </label>
              <label>
                <span className="settings-label">{t('settings.automation.schedule.mode')}</span>
                <select
                  className="settings-input"
                  value={w.mode}
                  onChange={(e) => setWindow(i, { mode: e.target.value })}
                >
                  <option value="FT8">FT8</option>
                  <option value="FT4">FT4</option>
                  <option value="FT2">FT2</option>
                  <option value="FT8+FT4">FT8 + FT4</option>
                  {!['FT8', 'FT4', 'FT2', 'FT8+FT4'].includes(w.mode) && (
                    <option value={w.mode}>{w.mode}</option>
                  )}
                </select>
              </label>
              <button
                type="button"
                className="automation-window-remove"
                onClick={() => set({ schedule: a.schedule.filter((_, j) => j !== i) })}
                aria-label={t('settings.automation.schedule.remove')}
                title={t('settings.automation.schedule.remove')}
              >
                ✕
              </button>
            </div>
          ))}
          <button
            type="button"
            className="automation-window-add"
            onClick={() => set({ schedule: [...a.schedule, { start: '12:00', bands: ['20m'], mode: 'FT8' }] })}
          >
            {t('settings.automation.schedule.add')}
          </button>
        </div>
      </section>

      <section className="automation-card">
        <h4 className="settings-subhead">{t('settings.automation.timing.head')}</h4>
        <div className="automation-grid">
          <NumberField
            label={t('settings.automation.pilot.listen')}
            value={a.listenSlots}
            lo={1}
            hi={20}
            onChange={(listenSlots) => set({ listenSlots })}
          />
          <NumberField
            label={t('settings.automation.pilot.dwell')}
            value={a.minDwellSecs}
            lo={30}
            hi={3600}
            onChange={(minDwellSecs) => set({ minDwellSecs })}
          />
          <NumberField
            label={t('settings.automation.pilot.quiet')}
            value={a.quietSecs}
            lo={60}
            hi={7200}
            onChange={(quietSecs) => set({ quietSecs })}
          />
          <NumberField
            label={t('settings.automation.pilot.ordinaryDwell')}
            value={a.ordinaryDwellSecs}
            lo={120}
            hi={7200}
            onChange={(ordinaryDwellSecs) => set({ ordinaryDwellSecs })}
            testId="automation-ordinary-dwell"
          />
          <NumberField
            label={t('settings.automation.pilot.maxCalls')}
            value={a.targetMaxCalls}
            lo={1}
            hi={30}
            onChange={(targetMaxCalls) => set({ targetMaxCalls })}
            testId="automation-max-calls"
          />
          <NumberField
            label={t('settings.automation.pilot.budget')}
            value={a.targetBudgetSecs}
            lo={30}
            hi={1800}
            onChange={(targetBudgetSecs) => set({ targetBudgetSecs })}
          />
          <NumberField
            label={t('settings.automation.pilot.rareBudget')}
            value={a.rareBudgetSecs}
            lo={600}
            hi={10800}
            onChange={(rareBudgetSecs) => set({ rareBudgetSecs })}
          />
          <NumberField
            label={t('settings.automation.pilot.maxHour')}
            value={a.maxQsoPerHour}
            lo={1}
            hi={300}
            onChange={(maxQsoPerHour) => set({ maxQsoPerHour })}
          />
        </div>
      </section>

      <section className="automation-card">
        <h4 className="settings-subhead">{t('settings.automation.answer.head')}</h4>
        <Switch
          label={t('settings.automation.answer.on')}
          on={a.answerCallers}
          onChange={(answerCallers) => set({ answerCallers })}
          hint={t('settings.automation.answer.on.hint')}
          testId="automation-answer-callers"
        />
        <div className="automation-grid">
          <NumberField
            label={t('settings.automation.answer.takeover')}
            value={a.answerTakeoverSlots}
            lo={0}
            hi={20}
            onChange={(answerTakeoverSlots) => set({ answerTakeoverSlots })}
            hint={t('settings.automation.answer.takeover.hint')}
          />
          <NumberField
            label={t('settings.automation.answer.chain')}
            value={a.answerChainLimit}
            lo={1}
            hi={20}
            onChange={(answerChainLimit) => set({ answerChainLimit })}
          />
        </div>
      </section>
    </div>
  )
}
