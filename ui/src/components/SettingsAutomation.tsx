// SPDX-License-Identifier: GPL-3.0-only
// OM0RX-xNexus: Settings ▸ Automation — Answer callers and the supervised autopilot. One object
// (`settings.automation`) edited whole, as Js8PaneStyleSettings edits its own. The engine clamps
// every number again on use (automation.rs `sanitized`), so a stray value can never wedge it; this
// form clamps too, so what it shows is what will run.
import { t } from '../i18n'
import type { AutomationSettings, AutopilotTiers, AutopilotWindow, Settings } from '../types'

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
  targetBudgetSecs: 120,
  rareBudgetSecs: 900,
  maxQsoPerHour: 60,
  tiers: {
    atno: true,
    dxpedition: true,
    dxccUnconfirmed: true,
    dxccNewSlot: true,
    reference: true,
    newCall: true,
  },
  chaseSpots: true,
}

/** The stored value with every missing field filled from the defaults. */
export function automationOf(s: Pick<Settings, 'automation'> | null | undefined): AutomationSettings {
  const a = s?.automation
  return {
    ...DEFAULT_AUTOMATION,
    ...(a ?? {}),
    tiers: { ...DEFAULT_AUTOMATION.tiers, ...(a?.tiers ?? {}) },
    schedule: Array.isArray(a?.schedule) ? a!.schedule : DEFAULT_AUTOMATION.schedule,
  }
}

/** Hold a typed number to a range; an empty or bad entry keeps the old value. */
export function clampInt(raw: string, lo: number, hi: number, old: number): number {
  const n = Math.round(Number(raw))
  if (raw.trim() === '' || !Number.isFinite(n)) return old
  return Math.min(hi, Math.max(lo, n))
}

const TIER_ROWS: { key: keyof AutopilotTiers; label: Parameters<typeof t>[0] }[] = [
  { key: 'atno', label: 'settings.automation.tiers.atno' },
  { key: 'dxpedition', label: 'settings.automation.tiers.dxpedition' },
  { key: 'dxccUnconfirmed', label: 'settings.automation.tiers.dxccUnconfirmed' },
  { key: 'dxccNewSlot', label: 'settings.automation.tiers.dxccNewSlot' },
  { key: 'reference', label: 'settings.automation.tiers.reference' },
  { key: 'newCall', label: 'settings.automation.tiers.newCall' },
]

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
      <h3 className="settings-subhead">{t('settings.automation.title')}</h3>
      <p className="settings-hint">{t('settings.automation.intro')}</p>

      <h4 className="settings-subhead">{t('settings.automation.answer.head')}</h4>
      <Switch
        label={t('settings.automation.answer.on')}
        on={a.answerCallers}
        onChange={(answerCallers) => set({ answerCallers })}
        hint={t('settings.automation.answer.on.hint')}
        testId="automation-answer-callers"
      />
      <NumberField
        label={t('settings.automation.answer.takeover')}
        value={a.answerTakeoverSlots}
        lo={0}
        hi={20}
        onChange={(answerTakeoverSlots) => set({ answerTakeoverSlots })}
      />
      <NumberField
        label={t('settings.automation.answer.chain')}
        value={a.answerChainLimit}
        lo={1}
        hi={20}
        onChange={(answerChainLimit) => set({ answerChainLimit })}
      />

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
        label={t('settings.automation.pilot.budget')}
        value={a.targetBudgetSecs}
        lo={30}
        hi={1800}
        onChange={(targetBudgetSecs) => set({ targetBudgetSecs })}
      />
      <NumberField
        label={t('settings.automation.pilot.rareBudget')}
        value={a.rareBudgetSecs}
        lo={60}
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
      <Switch
        label={t('settings.automation.pilot.chase')}
        on={a.chaseSpots}
        onChange={(chaseSpots) => set({ chaseSpots })}
      />

      <h4 className="settings-subhead">{t('settings.automation.tiers.head')}</h4>
      {TIER_ROWS.map((row) => (
        <Switch
          key={row.key}
          label={t(row.label)}
          on={a.tiers[row.key]}
          onChange={(on) => set({ tiers: { ...a.tiers, [row.key]: on } })}
        />
      ))}

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
    </div>
  )
}
