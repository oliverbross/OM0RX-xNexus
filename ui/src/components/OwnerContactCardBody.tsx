import { useEffect, useReducer, useSyncExternalStore, type InputHTMLAttributes } from 'react'
import { t } from '../i18n'
import { useFocusReturn } from '../focusReturn'
import type { CardDraft } from '../features/contactCard'
import { detailText, mapDetailField, referenceText, type DetailField, type OwnerContactFields } from '../features/ownerContactDetail'
import { ContactCardSurface, type OwnerContactCardProps } from './ContactCard'
import { confirmDialog } from '../confirm'
import type { OwnerEditFields } from '../xlog/draftStore'
import { ownerEditorMessage } from '../features/ownerEditorMessages'

const absent: DetailField<string> = { state: 'unavailable', value: null, reason: 'unsupported' }
const noop = () => {}
const noSubscribe = () => noop
const zero = () => 0
const editKeys: Partial<Record<keyof CardDraft,keyof OwnerEditFields>> = {
  date:'date',timeOn:'time',freq:'frequency',rstSent:'rstSent',rstRcvd:'rstReceived',grid:'grid',comment:'comment',
}
/** Provider name and glyph, as on the original card. */
const QRZ_PAGE = 'QRZ ↗'

/** No native service imports, subscription or write callbacks. A current input
 * is already admitted by its caller; this body cannot fetch or renew it. */
export function OwnerContactCardBody({ external, onClose, editor: suppliedEditor }: OwnerContactCardProps) {
  const editor=suppliedEditor?.contactId===external.contactId?suppliedEditor:undefined
  useSyncExternalStore(editor?.subscribe??noSubscribe,editor?.version??zero)
  const [,tick]=useReducer((n:number)=>n+1,0)
  // Display expiry only. This timer neither reads the owner nor grants a lease.
  useEffect(()=>{if(!editor)return;const timer=setInterval(tick,100);return()=>clearInterval(timer)},[editor])
  const editing=editor?.inspect()
  const returnFocus = useFocusReturn(true)
  const unavailable = t('contactCard.owner.unavailable')
  const read = external.read
  const detail = editing ? editing.detail : read.state === 'current' ? read.detail : null
  const matched = detail?.schema_version === 1 && detail.contact_id === external.contactId
  const f = detail && matched && detail.status === 'found' ? detail.fields : null
  function value<T>(field: DetailField<T> | undefined, format?: (v: T) => string): string {
    return field ? detailText(field, unavailable, format) : unavailable
  }
  const date = (field: DetailField<number> | undefined, part: 'date' | 'time') => {
    if (!field) return absent
    if (field.state === 'value' && (!Number.isSafeInteger(field.value) || !Number.isFinite(new Date(field.value).getTime()))) {
      return { state: 'unavailable', value: null, reason: 'invalid' } as const
    }
    return mapDetailField(field, (ms) => {
      const [day, time] = new Date(ms).toISOString().split('T')
      return part === 'date' ? day : time.slice(0, -1)
    })
  }
  const mapping: Record<keyof CardDraft, DetailField<string>> = {
    call: f?.contact.callsign ?? absent,
    date: date(f?.contact.started_at_unix_ms, 'date'),
    timeOn: date(f?.contact.started_at_unix_ms, 'time'),
    timeOff: date(f?.contact.ended_at_unix_ms, 'time'),
    band: f?.contact.band ?? absent,
    freq: f ? mapDetailField(f.radio.frequency_hz, (hz) => String(hz / 1_000_000)) : absent,
    mode: f?.contact.mode ?? absent,
    rstSent: f?.contact.rst_sent ?? absent,
    rstRcvd: f?.contact.rst_received ?? absent,
    txPower: f?.radio.tx_power_raw ?? absent,
    name: f?.station.name ?? absent, qth: f?.station.qth ?? absent,
    grid: f?.contact.grid ?? absent, state: f?.station.state ?? absent,
    // A program selection cannot represent several independently retained references.
    theirProgram: absent, myProgram: absent,
    theirRef: f ? { state: 'value', value: referenceText(f.references.their, unavailable), reason: null } : absent,
    myRef: f ? { state: 'value', value: referenceText(f.references.my, unavailable), reason: null } : absent,
    iota: f?.references.their.iota ?? absent,
    myGrid: f?.own_station.grid ?? absent, myRig: f?.own_station.recorded_rig ?? absent,
    qslSentVia: f?.confirmations.paper.sent_status ?? absent,
    qslCard: f?.confirmations.paper.received_status ?? absent,
    comment: f?.contact.comment ?? absent, notes: f?.contact.notes ?? absent,
  }
  // Strings are only form presentation, never a LoggedQso or mutation payload.
  const draft = Object.fromEntries(Object.entries(mapping).map(([key, field]) => [key, value(field)])) as unknown as CardDraft
  if(editing?.base){draft.call=editing.base.callsign;draft.band=editing.base.band;draft.mode=editing.base.mode}
  if(editing?.fields)for(const [cardKey,editKey] of Object.entries(editKeys) as [keyof CardDraft,keyof OwnerEditFields][]){
    draft[cardKey]=editing.fields[editKey]
  }
  const set=(key:keyof CardDraft,text:string)=>{const editable=editKeys[key];if(editable)editor?.set(editable,text)}
  const box = (key: keyof CardDraft, label: string, extra: InputHTMLAttributes<HTMLInputElement> = {}) => (
    <label className="logbook-field">
      <span>{label}</span>
      <input className="settings-input" {...extra} value={draft[key]} readOnly={!editing?.editable||!editKeys[key]}
        maxLength={editing&&key==='timeOn'?16:extra.maxLength}
        onChange={editKeys[key]?(event)=>set(key,event.target.value):undefined}
        autoComplete="off" spellCheck={false} aria-label={label}
        aria-description={mapping[key].state === 'unavailable' ? unavailable : undefined} />
    </label>
  )
  const message = editing ? ownerEditorMessage(editing) : read.state !== 'current' ? read.message
    : !matched ? t('contactCard.owner.mismatch')
    : detail?.status === 'deleted' ? t('contactCard.owner.deleted')
    : detail?.status === 'not_found' ? t('contactCard.owner.notFound')
    : !f ? unavailable : t('contactCard.owner.readOnly')
  const facts = f ? ownerFacts(f, unavailable) : []
  const head = (
    <div className="callcard-head contact-card-head">
      <span className="callcard-avatar" aria-hidden title={unavailable} />
      <div className="callcard-id">
        <div className="callcard-title">
          <span className="callcard-call mono">{draft.call}</span>
          {mapping.name.state === 'value' && <span className="callcard-name">{draft.name}</span>}
        </div>
        <div className="callcard-actions">
          <button type="button" className="callcard-lookup" disabled title={unavailable}>{t('logEntry.lookup.label')}</button>
          <button type="button" className="callcard-qrz" disabled title={unavailable}>{QRZ_PAGE}</button>
        </div>
        <p className="settings-hint" role="status">{message}</p>
        {!editing?.busy&&editing?.receipt?.message&&<p className="settings-hint" role="status">{editing.receipt.message}</p>}
      </div>
    </div>
  )
  const confirmation = <>
    {box('qslSentVia', t('logbook.field.qslSent.label'))}
    {box('qslCard', t('logbook.field.qslCard.label'))}
    <p className="settings-hint logbook-field-wide">{t('contactCard.owner.rawConfirmations')}</p>
  </>
  const recent = <section className="qso-detail-section">
    <h3 className="qso-detail-heading">{t('contactCard.recent')}</h3>
    <p className="qso-detail-bare">{t('contactCard.owner.dependentUnavailable')}</p>
  </section>
  async function leave(then:()=>void){
    const policy=editor?.closePolicy()??'close'
    if(policy==='blocked')return
    if(policy==='confirm-discard'){
      const confirmed=await confirmDialog({title:t('contactCard.discard.title',{call:draft.call}),
        body:t('contactCard.discard.body'),confirmLabel:t('contactCard.discard.confirm'),cancelLabel:t('contactCard.discard.keep')})
      if(!confirmed||!editor?.discard())return
    }
    then()
  }
  async function reload(){
    if(!editor?.inspect().canReload)return
    if(editor.inspect().dirty&&!await confirmDialog({title:t('contactCard.discard.title',{call:draft.call}),
      body:t('contactCard.editor.reloadBody'),confirmLabel:t('contactCard.discard.confirm'),cancelLabel:t('contactCard.discard.keep')}))return
    editor.reload()
  }
  const actions=editing&&editor?<>
    {editing.canInspectQueue&&<button type="button" className="log-rowbtn" onClick={()=>void editor.inspectQueue()}>{t('contactCard.editor.queueCheck')}</button>}
    {editing.canRetryQueue&&<button type="button" className="log-rowbtn" onClick={()=>void editor.retryQueue()}>{t('contactCard.editor.queueRetry')}</button>}
    {editing.canSubmit&&<button type="button" className="settings-save" onClick={()=>void editor.submit()}>{t('contactCard.editor.submit')}</button>}
    {editing.canReconcile&&<button type="button" className="log-rowbtn" onClick={()=>void editor.reconcile()}>{t('contactCard.editor.receiptCheck')}</button>}
    {editing.canReload&&<button type="button" className="log-rowbtn" onClick={()=>void reload()}>{t('contactCard.editor.reload')}</button>}
  </>:undefined
  return <ContactCardSurface
    commentPolicy={editing?'owner-multiline':'readonly-multiline'} commentEditable={editing?.editable??false} extendedEditing={false}
    call={draft.call} draft={draft} fields={[]} err={editing?.error?t('contactCard.editor.failure'):null} busy={editing?.busy??false} dirty={editing?.dirty??false}
    canSave={editing?.canSave??false} notResent={[]} facts={facts} box={box}
    program={(key, label) => box(key, label)} set={set} setField={noop} setFields={noop}
    editingNote={editing?<span className="logbook-editing-note" role="status">{message}</span>:undefined}
    extraActions={actions} saveLabel={editing?t('contactCard.editor.queue'):undefined}
    leave={leave} onClose={onClose} onSave={async(event)=>{event?.preventDefault();await editor?.save()}}
    returnFocus={returnFocus} header={head} iota={null} confirmation={confirmation}
    path={null} awards={null} recent={recent} history={null}
  />
}

/** Whitelisted source DTO only. Raw statuses/dates are shown as recorded, not
 * converted into Nexus confirmation booleans or award eligibility. */
function ownerFacts(f: OwnerContactFields, unavailable: string): [string, string][] {
  const facts: [string, string][] = []
  function add<T>(label: string, field: DetailField<T>) {
    // Known empty and unavailable remain distinct even inside the retained facts.
    facts.push([label, detailText(field, unavailable)])
  }
  add('SUBMODE', f.contact.submode)
  add('QSO_DATE', f.contact.raw_qso_date); add('TIME_ON', f.contact.raw_time_on)
  add('QSO_DATE_OFF', f.contact.raw_qso_date_off); add('TIME_OFF', f.contact.raw_time_off)
  add(t('contactCard.freqRx'), mapDetailField(f.radio.receive_frequency_hz, (hz) => String(hz / 1_000_000)))
  add(t('qso.detail.propMode'), f.radio.propagation_mode); add(t('qso.detail.satName'), f.radio.satellite_name)
  add('ANT_PATH', f.radio.antenna_path)
  add(t('logbook.column.country'), f.station.country); add('DXCC', f.station.dxcc)
  add('CNTY', f.station.county); add('CONT', f.station.continent)
  add('CQZ', f.station.cq_zone); add('ITUZ', f.station.itu_zone)
  add(t('logbook.column.operator'), f.own_station.operator_call)
  add(t('qso.detail.stationCallsign'), f.own_station.callsign)
  add(t('qso.detail.credit.granted'), f.awards.credit_granted)
  add(t('qso.detail.credit.submitted'), f.awards.credit_submitted)
  add(t('contactCard.owner.awardAuthority'), f.awards.authority)
  for (const [service, upload] of [['LoTW', f.uploads.lotw], ['eQSL', f.uploads.eqsl], ['QRZ', f.uploads.qrz], ['Club Log', f.uploads.clublog]] as const) {
    add(t('contactCard.owner.uploadReceipt', { service }), upload.state)
  }
  for (const [prefix, c] of [['QSL', f.confirmations.paper], ['LOTW_QSL', f.confirmations.lotw], ['EQSL_QSL', f.confirmations.eqsl], ['QRZ_QSL', f.confirmations.qrz]] as const) {
    add(`${prefix}_SENT`, c.sent_status); add(`${prefix}_RCVD`, c.received_status)
    add(`${prefix}_SENT_DATE`, c.sent_date); add(`${prefix}_RCVD_DATE`, c.received_date)
    add(`${prefix}_SENT_VIA`, c.sent_via); add(`${prefix}_RCVD_VIA`, c.received_via)
  }
  for (const [key, field] of Object.entries(f.contest)) add(key.toUpperCase(), field)
  return facts
}
