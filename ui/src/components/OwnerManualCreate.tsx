import { useSyncExternalStore } from 'react'
import { Dialog } from './ui/Dialog'
import { LogbookForm, type LogbookFormDraft } from './LogbookForm'
import { confirmDialog } from '../confirm'
import { t } from '../i18n'
import { manualCreateMessage } from '../xlog/manualCreateMessages'
import { manualCreateKeys } from '../xlog/manualCreateFields'
import type { ManualCreateController } from '../xlog/manualCreateController'
const supported:ReadonlySet<keyof LogbookFormDraft>=new Set(manualCreateKeys)
const absent={name:'',qth:'',notes:'',endTime:'',state:'',txPower:'',parkTheirRef:'',parkMyRef:'',iota:'',myGrid:'',myRig:'',qslSentVia:'',qslCard:''}
const unavailable=()=>{} // Unsupported native actions have no service/IPC fallback.
/** The actual upstream form inside the existing dialog primitive. No page rows,
 * native log source, lookup widgets or completed-QSO inference are mounted. */
export function OwnerManualCreate({controller,onClose,connection}:{controller:ManualCreateController;onClose:()=>void;connection:()=>void}) {
  useSyncExternalStore(controller.subscribe,controller.version,controller.version)
  const view=controller.view(),close=()=>{if(controller.closePolicy()!=='blocked')onClose()}
  const discard=async()=>{
    const version=controller.version()
    const yes=await confirmDialog({title:t('logbook.ownerCreate.discard'),body:t('contactCard.discard.body'),confirmLabel:t('contactCard.discard.confirm'),cancelLabel:t('contactCard.discard.keep')})
    if(yes&&controller.version()===version)controller.discard()
  }
  return <Dialog open onOpenChange={open=>{if(!open)close()}} title={t('logbook.ownerCreate.title')}
    description={t('logbook.ownerCreate.instructions')} className="owner-manual-create-dialog">
    <LogbookForm draft={{...absent,...view.fields}} setField={(key,value)=>{
      if(supported.has(key))controller.edit(key as typeof manualCreateKeys[number],value)
    }} submit={event=>{event.preventDefault();void controller.queue()}} cancelForm={close}
      disabled={!view.editable} owner={{supported,unavailableLabel:t('logbook.ownerCreate.unsupported')}}
      qrzBusy={false} onQrzLookup={unavailable} whenDateBad={false} whenTimeBad={false} endTimeBad={false}
      onIota={unavailable} iotaInvalid={false} editingKey={null} qslViaLabel={()=>null} alsoStored={[]} err={null}
      actions={<>
        {(view.busy || view.status !== 'ready' && view.status !== 'dirty') && <span role="status">{manualCreateMessage(view.status,view.busy)}</span>}
        <button type="button" className="logconfirm-discard" disabled={view.busy} onClick={connection}>{t('contactCard.editor.connection')}</button>
        <button type="button" className="logconfirm-discard" disabled={controller.closePolicy()==='blocked'} onClick={close}>{t('logbook.form.close')}</button>
        {view.canPersist&&<button type="button" className="settings-save" onClick={controller.retrySave}>{t('logbook.ownerCreate.retryDraft')}</button>}
        {view.canDiscard&&<button type="button" className="logconfirm-discard" onClick={()=>void discard()}>{t('logbook.ownerCreate.discard')}</button>}
        {!view.canInspect&&<button type="submit" className="settings-save" disabled={!view.canQueue}>{t('logbook.ownerCreate.queue')}</button>}
        {view.canInspect&&<button type="button" className="settings-save" disabled={view.busy} onClick={()=>void controller.inspectQueue()}>{t('contactCard.editor.queueCheck')}</button>}
        {view.canRetryQueue&&<button type="button" className="settings-save" onClick={()=>void controller.retryQueue()}>{t('contactCard.editor.queueRetry')}</button>}
        {view.canSubmit&&<button type="button" className="settings-save" onClick={()=>void controller.submit()}>{t('contactCard.editor.submit')}</button>}
        {view.canReconcile&&<button type="button" className="settings-save" onClick={()=>void controller.reconcile()}>{t('contactCard.editor.receiptCheck')}</button>}
        {view.canFinish&&<button type="button" className="settings-save" onClick={()=>{if(controller.finish())onClose()}}>{t('logbook.ownerCreate.done')}</button>}
      </>} />
  </Dialog>
}
