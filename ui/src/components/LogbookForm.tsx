// The original Logbook manual form. Native callers retain their existing policy;
// owner callers explicitly withhold unsupported inputs and provide queue actions.
import type { FormEventHandler, ReactNode, Ref } from 'react'
import { t } from '../i18n'
import { oneLineTyped } from '../typedInput'
import { UTC_DATE_FORMAT, UTC_TIME_FORMATS } from '../features/utcLog'
const QRZ_LABEL = 'QRZ'
export interface LogbookFormDraft {
  call: string
  grid: string
  band: string
  freq: string
  mode: string
  rstSent: string
  rstRcvd: string
  name: string
  qth: string
  comment: string
  notes: string
  /** When the contact happened, in UTC: the date (`YYYY-MM-DD`) and a 24-hour time (`HH:MM` or
   * `HH:MM:SS`, features/utcLog). THE fields that make hand-logging work: you log a 2 m contact
   * after the fact, so stamping "now" writes the wrong time into the log and into every upload
   * downstream. The time is a text box, not a native control, because WebView2 draws those in
   * the OS locale — 00:58 read "12:58 AM" on a 12-hour PC, and records got "fixed" by 12 hours
   * (#280). Both blank = now (a new contact) or the stored time (an edit). */
  whenDate: string
  whenTime: string
  /** #329: when the contact ENDED (ADIF TIME_OFF), 24-hour UTC, on the contact's own date —
   * one box, because an end time on a different day is the rollover case and `submit` reads
   * it as one. The field was written at log time and exported all along, but nothing here
   * showed it, so a wrong end time (Ham Radio Deluxe's 00:00) could not even be seen.
   *
   * ⚠️ Blank means LEAVE ALONE, not "clear it": an omitted end time is restored from the
   * stored record by `Logbook::update_record`, so clearing one is deliberately not offered. */
  endTime: string
  /** US state (WAS). The auto-log path fills this from the callsign/grid; a hand-logged
   * contact has no decode to derive it from, so it has to be typeable or WAS silently misses. */
  state: string
  /** TX power in watts (ADIF TX_PWR) — part of a complete hand-log, and some awards want it. */
  txPower: string
  /** POTA/SIG park references: the worked station's park (`ota.theirRef`, the hunter case) and
   * your own activation (`ota.myRef`). Editable so a park can be viewed, corrected or added on
   * an existing QSO (#60). Only these two of the five `Ota` fields are exposed; the loaded
   * program (POTA/SOTA/WWFF) rides through `submit` from the stored record so an edit never
   * clobbers a non-POTA program. */
  parkTheirRef: string
  parkMyRef: string
  /** The worked station's IOTA reference ("NA-001"). Blank on an edit leaves the stored one, as
   * the park boxes do; anything else must be a reference (`IOTA_RE`). */
  iota: string
  /** #239: your own grid square and rig for this contact (ADIF MY_GRIDSQUARE / MY_RIG). */
  myGrid: string
  myRig: string
  /** #239: QSL sent via '' | 'B' | 'D' | 'E', or 'SENT' for a stored "sent" with no route; and
   * a paper card received ('1' | ''). Strings, like every draft field, so `setField` serves them. */
  qslSentVia: string
  qslCard: string
}

interface Props {
  draft: LogbookFormDraft; setField: (key: keyof LogbookFormDraft, value: string) => void
  submit: FormEventHandler<HTMLFormElement>; cancelForm: () => void; formRef?: Ref<HTMLFormElement>
  qrzBusy?: boolean; onQrzLookup: () => void
  whenDateBad: boolean; whenTimeBad: boolean; endTimeBad: boolean
  onIota: (value: string) => void; iotaInvalid: boolean; iotaNotes?: ReactNode; iotaIsland?: ReactNode
  editingKey: string | null; qslViaLabel: (value: string) => string | null
  alsoStored: readonly (readonly [string, string])[]; err: string | null
  disabled?: boolean; actions?: ReactNode
  owner?: { supported: ReadonlySet<keyof LogbookFormDraft>; unavailableLabel: string }
}
export function LogbookForm({ draft, setField, submit, cancelForm, formRef, qrzBusy=false,
  onQrzLookup, whenDateBad, whenTimeBad, endTimeBad, onIota, iotaInvalid, iotaNotes, iotaIsland,
  editingKey, qslViaLabel, alsoStored, err, disabled=false, actions, owner }: Props) {
  const unavailable = (key: keyof LogbookFormDraft) => !!owner && !owner.supported.has(key)
  return (
<form
          className="logbook-form"
          onSubmit={submit}
          ref={formRef}
          onKeyDown={(e) => {
            if (e.key !== 'Escape') return
            e.preventDefault()
            cancelForm()
          }}
        >
          <div className="logbook-form-grid">
            <label className="logbook-field logbook-field-call">
              <span>{t('logbook.field.call.label')}</span>
              <div className="settings-input-row">
                <input
                  className="settings-input"
                  value={draft.call} disabled={disabled || unavailable('call')}
                  onChange={(e) => setField('call', e.target.value)}
                  autoComplete="off"
                  spellCheck={false}
                />
                <button
                  type="button"
                  className="settings-refresh"
                  onClick={onQrzLookup}
                  disabled={disabled || owner !== undefined || qrzBusy || !draft.call.trim()}
                  title={t('logbook.field.qrz.title')}
                >
                  {qrzBusy ? '…' : QRZ_LABEL}
                </button>
              </div>
            {owner && unavailable('call') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.grid.label')}</span>
              <input className="settings-input" value={draft.grid} disabled={disabled || unavailable('grid')} onChange={(e) => setField('grid', e.target.value)} autoComplete="off" spellCheck={false} />
            {owner && unavailable('grid') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.band.label')}</span>
              <input className="settings-input" value={draft.band} disabled={disabled || unavailable('band')} onChange={(e) => setField('band', e.target.value)} autoComplete="off" />
            {owner && unavailable('band') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.freq.label')}</span>
              <input className="settings-input" type="number" step={owner ? "0.000001" : "0.0001"} value={draft.freq} disabled={disabled || unavailable('freq')} onChange={(e) => setField('freq', e.target.value)} autoComplete="off" />
            {owner && unavailable('freq') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.mode.label')}</span>
              <input className="settings-input" value={draft.mode} disabled={disabled || unavailable('mode')} onChange={(e) => setField('mode', e.target.value)} autoComplete="off" />
            {owner && unavailable('mode') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.rstSent.label')}</span>
              <input className="settings-input" value={draft.rstSent} disabled={disabled || unavailable('rstSent')} onChange={(e) => setField('rstSent', e.target.value)} autoComplete="off" />
            {owner && unavailable('rstSent') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.rstRcvd.label')}</span>
              <input className="settings-input" value={draft.rstRcvd} disabled={disabled || unavailable('rstRcvd')} onChange={(e) => setField('rstRcvd', e.target.value)} autoComplete="off" />
            {owner && unavailable('rstRcvd') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.date.label')}</span>
              {/* UTC as typed, beside the time and for the same reason (#280): a native date
                  control is drawn in the OS locale and its "Today" fills the LOCAL date, which
                  after 0000Z west of Greenwich is the previous UTC day. */}
              <input
                className={`settings-input mono logbook-when${whenDateBad ? ' invalid' : ''}`}
                value={draft.whenDate} disabled={disabled || unavailable('whenDate')}
                onChange={(e) => setField('whenDate', e.target.value)}
                placeholder={UTC_DATE_FORMAT}
                maxLength={10}
                spellCheck={false}
                autoComplete="off"
                aria-invalid={whenDateBad}
                title={t('logbook.field.when.title')}
              />
            {owner && unavailable('whenDate') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.time.label')}</span>
              {/* 24-hour UTC as typed — a native time control would draw it in the OS locale (#280). */}
              <input
                className={`settings-input mono${whenTimeBad ? ' invalid' : ''}`}
                value={draft.whenTime} disabled={disabled || unavailable('whenTime')}
                onChange={(e) => setField('whenTime', e.target.value)}
                placeholder={UTC_TIME_FORMATS.short}
                maxLength={owner ? 16 : 8}
                spellCheck={false}
                autoComplete="off"
                aria-invalid={whenTimeBad}
                title={t('logbook.field.when.title')}
              />
            {owner && unavailable('whenTime') && <small>{owner.unavailableLabel}</small>}</label>
            {/* #329: the contact's END, beside the time it started. */}
            <label className="logbook-field">
              <span>{t('logbook.field.end.label')}</span>
              <input
                className={`settings-input mono${endTimeBad ? ' invalid' : ''}`}
                value={draft.endTime} disabled={disabled || unavailable('endTime')}
                onChange={(e) => setField('endTime', e.target.value)}
                placeholder={UTC_TIME_FORMATS.short}
                maxLength={8}
                spellCheck={false}
                autoComplete="off"
                aria-invalid={endTimeBad}
                title={t('logbook.field.end.title')}
              />
            {owner && unavailable('endTime') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.state.label')}</span>
              <input
                className="settings-input"
                value={draft.state} disabled={disabled || unavailable('state')}
                onChange={(e) => setField('state', e.target.value)}
                maxLength={2}
                autoComplete="off"
                title={t('logbook.field.state.title')}
              />
            {owner && unavailable('state') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.txPower.label')}</span>
              <input
                className="settings-input"
                type="number"
                min="0"
                value={draft.txPower} disabled={disabled || unavailable('txPower')}
                onChange={(e) => setField('txPower', e.target.value)}
                autoComplete="off"
              />
            {owner && unavailable('txPower') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.parkTheirs.label')}</span>
              <input
                className="settings-input"
                value={draft.parkTheirRef} disabled={disabled || unavailable('parkTheirRef')}
                onChange={(e) => setField('parkTheirRef', e.target.value)}
                autoComplete="off"
                title={t('logbook.field.parkTheirs.title')}
              />
            {owner && unavailable('parkTheirRef') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.parkMine.label')}</span>
              <input
                className="settings-input"
                value={draft.parkMyRef} disabled={disabled || unavailable('parkMyRef')}
                onChange={(e) => setField('parkMyRef', e.target.value)}
                autoComplete="off"
                title={t('logbook.field.parkMine.title')}
              />
            {owner && unavailable('parkMyRef') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.iota.label')}</span>
              <input
                className="settings-input"
                value={draft.iota} disabled={disabled || unavailable('iota')}
                onChange={(e) => onIota(e.target.value)}
                autoComplete="off"
                spellCheck={false}
                maxLength={6}
                aria-invalid={iotaInvalid}
                title={t('logbook.field.iota.title')}
              />
              {iotaNotes}
            {owner && unavailable('iota') && <small>{owner.unavailableLabel}</small>}</label>
            {iotaIsland}
            <label className="logbook-field">
              <span>{t('logbook.field.name.label')}</span>
              <input className="settings-input" value={draft.name} disabled={disabled || unavailable('name')} onChange={(e) => setField('name', e.target.value)} autoComplete="off" />
            {owner && unavailable('name') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field">
              <span>{t('logbook.field.qth.label')}</span>
              <input className="settings-input" value={draft.qth} disabled={disabled || unavailable('qth')} onChange={(e) => setField('qth', e.target.value)} autoComplete="off" />
            {owner && unavailable('qth') && <small>{owner.unavailableLabel}</small>}</label>
            {/* #239: your own location and rig for THIS contact (ADIF MY_GRIDSQUARE / MY_RIG). */}
            <>
              <label className="logbook-field">
                <span>{t('logbook.field.myGrid.label')}</span>
                <input
                  className="settings-input"
                  value={draft.myGrid} disabled={disabled || unavailable('myGrid')}
                  onChange={(e) => setField('myGrid', e.target.value)}
                  title={t('logbook.field.myGrid.title')}
                  autoComplete="off"
                  spellCheck={false}
                />
              {owner && unavailable('myGrid') && <small>{owner.unavailableLabel}</small>}</label>
              <label className="logbook-field">
                <span>{t('logbook.field.myRig.label')}</span>
                <input
                  className="settings-input"
                  value={draft.myRig} disabled={disabled || unavailable('myRig')}
                  onChange={(e) => setField('myRig', e.target.value)}
                  title={t('logbook.field.myRig.title')}
                  autoComplete="off"
                />
              {owner && unavailable('myRig') && <small>{owner.unavailableLabel}</small>}</label>
            </>
            {/* #239: QSL status where the rest of the contact is edited — saved in the SAME change
                as the fields (`editQsoById`), and only where it differs from the stored contact.
                LoTW and eQSL confirmations stay the services'; only the card is the operator's to
                mark. Edit mode only: both marks address a logged record. */}
            {editingKey !== null && (
              <>
                <div className="logbook-field">
                  <label htmlFor="logbook-qsl-sent">{t('logbook.field.qslSent.label')}</label>
                  <select
                    id="logbook-qsl-sent"
                    className="settings-input"
                    value={draft.qslSentVia} disabled={disabled || unavailable('qslSentVia')}
                    onChange={(e) => setField('qslSentVia', e.target.value)}
                  >
                    <option value="">{t('logbook.field.qslSent.none')}</option>
                    {draft.qslSentVia === 'SENT' && (
                      <option value="SENT">{t('logbook.field.qslSent.sent')}</option>
                    )}
                    <option value="B">{qslViaLabel('B')}</option>
                    <option value="D">{qslViaLabel('D')}</option>
                    <option value="E">{qslViaLabel('E')}</option>
                  </select>
                </div>
                <label className="logbook-field">
                  <span>{t('logbook.field.qslCard.label')}</span>
                  <input
                    type="checkbox"
                    disabled={disabled || unavailable('qslCard')} checked={draft.qslCard === '1'}
                    onChange={(e) => setField('qslCard', e.target.checked ? '1' : '')}
                  />
                {owner && unavailable('qslCard') && <small>{owner.unavailableLabel}</small>}</label>
              </>
            )}
            {/* The shared comment: full width, always directly above Notes (it was a 140 px cell
                among the short fields). It wraps over lines but holds ONE, as ADIF's COMMENT does:
                a line break typed or pasted lands as a space, and Enter saves as the old box did.
                A stored comment with line breaks (an import's) is shown and saved as it is unless
                the operator edits this box. */}
            <label className="logbook-field logbook-field-wide">
              <span>{t('logbook.field.comment.label')}</span>
              <textarea
                className="settings-input logbook-comment"
                value={draft.comment} disabled={disabled || unavailable('comment')}
                onChange={(e) => setField('comment', owner ? e.target.value : oneLineTyped(e.target))}
                onKeyDown={(e) => {
                  if (owner || e.key !== 'Enter') return
                  e.preventDefault()
                  e.currentTarget.form?.requestSubmit()
                }}
                placeholder={t('logbook.field.comment.placeholder')}
                rows={2}
                autoComplete="off"
              />
            {owner && unavailable('comment') && <small>{owner.unavailableLabel}</small>}</label>
            <label className="logbook-field logbook-field-wide">
              <span>{t('logbook.field.notes.label')}</span>
              <textarea
                className="settings-input logbook-notes"
                value={draft.notes} disabled={disabled || unavailable('notes')}
                onChange={(e) => setField('notes', e.target.value)}
                placeholder={t('logbook.field.notes.placeholder')}
                rows={3}
              />
            {owner && unavailable('notes') && <small>{owner.unavailableLabel}</small>}</label>
            {/* What the contact carries that no box here edits — the zones, the county, the QSL
                dates, the upload state and the contest exchange — shown so an edit is made with
                the whole contact in view. Read-only: each is the record's or a service's. */}
            {alsoStored.length > 0 && (
              <div className="logbook-field logbook-field-wide logbook-also">
                <span>{t('logbook.form.alsoStored')}</span>
                <dl>
                  {alsoStored.map(([label, value]) => (
                    <div key={label}>
                      <dt>{label}</dt>
                      <dd className="mono">{value}</dd>
                    </div>
                  ))}
                </dl>
              </div>
            )}
          </div>
          <div className="logbook-form-actions">
            {actions ?? <>
            {err && <span className="settings-error" role="alert">{err}</span>}
            {editingKey !== null && (
              <span className="logbook-editing-note">{t('logbook.form.editingNote')}</span>
            )}
            {/* #239: leave the form without saving, as Esc does. */}
            <button type="button" className="logconfirm-discard" onClick={cancelForm}>
              {t('logbook.form.cancel')}
            </button>
            <button type="submit" className="settings-save" disabled={!draft.call.trim()}>
              {editingKey !== null ? t('logbook.form.save') : t('logbook.form.log')}
            </button>
            </>}
          </div>
        </form>
  )
}
