import type { Bridge, OutboxEntry, Receipt } from './api'
import { blankManualFields, manualCreateQso, sameCreateQso, validManualFields, type ManualCreateFields } from './manualCreateFields'
import { loadCreateDraft, saveCreateDraft, discardCreateDraft, retireAppliedCreateDraft, type ManualCreateRecord, type CreateStorage } from './manualCreateStore'
export interface ManualCreateAuthority {
  workspaceId:string; serverInstance:string; connectionToken:string; generation:number; deadline:number; writeGranted:boolean
}
type Port=Pick<Bridge,'enqueue'|'deliver'|'outbox'>
export type ManualCreateStatus='ready'|'dirty'|'invalid_input'|'invalid'|'unavailable'|'occupied'|'storage_unavailable'|'foreign'
  |'frozen'|'queue_unknown'|'queued'|'needs_receipt'|'accepted'|'uncertain'|'observed'|'rejected'|'applied'|'receipt_invalid'|'source_changed'
function clone<T>(value:T):T{return JSON.parse(JSON.stringify(value)) as T}
const scope=(s:ManualCreateAuthority)=>JSON.stringify([s.workspaceId,s.serverInstance,s.connectionToken,s.generation])
export function createManualCreate(options:{source:()=>ManualCreateAuthority|null;port:Port;store:CreateStorage;now?:()=>number;utc?:()=>number;id?:()=>string}) {
  const now=options.now??(()=>performance.now()),id=options.id??(()=>crypto.randomUUID()),load=loadCreateDraft(options.store)
  let record=load.state==='current'?clone(load.value):null,raw=load.raw
  const initial=blankManualFields((options.utc??Date.now)()),requestId=record?.request_id??id(),contactId=record?.contact_id??id()
  let fields=record?clone(record.fields):initial,status:ManualCreateStatus=load.state==='current'?(record?.frozen?'frozen':'dirty'):load.state==='empty'?'ready':load.state==='unavailable'?'storage_unavailable':load.state
  let blocked=load.state==='invalid'||load.state==='unavailable',busy=false,disposed=false,clockBad=false,lastClock:number|null=null
  // Restored frozen intent may already have crossed an enqueue boundary. Never
  // infer absence of an attempt from a missing queue entry or a missing receipt.
  let enqueueAttempted=!!record?.frozen
  let verified=false,checkedMissing=false,durable=!!record,entry:OutboxEntry|null=null,receipt:Receipt|null=null,version=0
  const listeners=new Set<()=>void>(),emit=()=>{version++;listeners.forEach(f=>f())}
  function authority():ManualCreateAuthority|null {
    if(disposed||clockBad)return null
    let current:number,source:ManualCreateAuthority|null
    try{current=now();source=options.source()}catch{clockBad=true;return null}
    if(!Number.isFinite(current)||current<0||lastClock!==null&&current<lastClock){clockBad=true;return null}
    lastClock=current
    if(!source||!source.writeGranted||!source.workspaceId.trim()||!source.serverInstance||!source.connectionToken
      ||!Number.isSafeInteger(source.generation)||source.generation<0||!Number.isFinite(source.deadline)||current>=source.deadline)return null
    return source
  }
  function persist(next:ManualCreateRecord):boolean {
    record=clone(next);fields=clone(next.fields);durable=false
    try{raw=saveCreateDraft(options.store,record,raw);durable=true;return true}
    catch(error){status=error instanceof Error&&error.message==='occupied'?'occupied':'storage_unavailable';if(status==='occupied')blocked=true;return false}
  }
  function matching():ManualCreateAuthority|null {
    const source=authority();return source&&(!record||record.workspace_id===source.workspaceId)?source:null
  }
  function view() {
    const current=authority(),source=current&&(!record||record.workspace_id===current.workspaceId)?current:null,frozen=!!record?.frozen,q=manualCreateQso(contactId,fields)
    const terminal=!!receipt&&['applied','observed','rejected'].includes(receipt.status)
    const displayStatus=record&&current&&current.workspaceId!==record.workspace_id?'foreign':!current&&['ready','dirty','invalid_input'].includes(status)?'unavailable':status==='dirty'&&!q?'invalid_input':status
    return {fields:clone(fields),requestId,contactId,workspaceId:record?.workspace_id??null,status:displayStatus as ManualCreateStatus,
      receipt:receipt?clone(receipt):null,busy,dirty:!!record,editable:!busy&&!blocked&&!frozen&&!!source,
      canQueue:!busy&&!blocked&&!frozen&&!!q&&!!source,
      canRetryQueue:!busy&&!blocked&&frozen&&checkedMissing&&!entry&&!receipt&&!!source,
      canSubmit:!busy&&!blocked&&durable&&entry?.state==='Pending'&&!!source,
      canReconcile:!busy&&!blocked&&durable&&verified&&frozen&&!terminal&&entry?.state!=='Pending'&&!!source,
      canFinish:!busy&&!blocked&&durable&&receipt?.status==='applied',canPersist:!busy&&!blocked&&!!record&&!durable,canInspect:!busy&&!blocked&&frozen,canDiscard:!busy&&!blocked&&!!record&&(!frozen||!durable&&!enqueueAttempted)}
  }
  function edit(key:keyof ManualCreateFields,value:string) {
    if(!view().editable)return
    const source=matching();if(!source)return
    const next={...fields,[key]:value};if(!validManualFields(next))return
    if(persist({version:1,kind:'owner_manual_create',request_id:requestId,contact_id:contactId,
      workspace_id:record?.workspace_id??source.workspaceId,fields:next,frozen:null}))status='dirty'
    emit()
  }
  function acceptReceipt(value:Receipt):boolean {
    if(!verified||!record||!value||value.request_id!==requestId||value.workspace_id!==record.workspace_id
      ||!['accepted','applied','observed','rejected','uncertain'].includes(value.status)
      ||value.contact_id!=null&&value.contact_id.toLowerCase()!==contactId.toLowerCase()
      ||value.status==='applied'&&(value.contact_id?.toLowerCase()!==contactId.toLowerCase()
        ||!Number.isSafeInteger(value.contact_revision)||value.contact_revision!<1)){
      status='receipt_invalid';return false
    }
    receipt=clone(value);status=value.status;return true
  }
  function acceptEntry(value:OutboxEntry):boolean {
    if(!record?.frozen||!value?.queued?.mutation||value.queued.version!==1||value.queued.request_id!==requestId||value.queued.workspace_id!==record.workspace_id
      ||value.queued.mutation.kind!=='logbook.create'||!sameCreateQso(value.queued.mutation.qso,record.frozen)){
      status='receipt_invalid';blocked=true;return false
    }
    enqueueAttempted=true;verified=true;checkedMissing=false
    if(value.state==='Pending'||value.state==='NeedsReceipt'){entry=clone(value);status=value.state==='Pending'?'queued':'needs_receipt';return true}
    if(value.state&&typeof value.state==='object'&&acceptReceipt(value.state.Settled)){entry=clone(value);return true}
    status='receipt_invalid';return false
  }
  async function queue(retry=false) {
    const v=view();if(retry?!v.canRetryQueue:!v.canQueue)return
    const source=matching();if(!source)return
    const frozen=record?.frozen??manualCreateQso(contactId,fields);if(!frozen)return
    busy=true;emit()
    // Persist the exact immutable payload before *any* enqueue attempt, including
    // an acknowledged storage failure's explicit retry. Never invent a new ID.
    if(!persist({version:1,kind:'owner_manual_create',request_id:requestId,contact_id:contactId,
      workspace_id:record?.workspace_id??source.workspaceId,fields:clone(fields),frozen:clone(frozen)})){busy=false;emit();return}
    const current=matching()
    if(!current||scope(current)!==scope(source)){status='source_changed';busy=false;emit();return}
    status='frozen'
    // This latch precedes the actual port call, including synchronous throws.
    enqueueAttempted=true
    try{
      const rows=await options.port.enqueue({kind:'logbook.create',qso:clone(frozen)},requestId,record!.workspace_id)
      const found=rows.filter(row=>row.queued.request_id===requestId)
      if(found.length===1)acceptEntry(found[0]);else status='queue_unknown'
    }catch{status='queue_unknown'}finally{busy=false;emit()}
  }
  async function inspectQueue(){
    if(!view().canInspect)return
    busy=true;entry=null;verified=false;checkedMissing=false;emit()
    try{const rows=await options.port.outbox(),found=rows.filter(row=>row.queued.request_id===requestId)
      if(found.length===1)acceptEntry(found[0]);else if(found.length>1){blocked=true;status='receipt_invalid'}else {checkedMissing=true;status='frozen'}
    }catch{status='queue_unknown'}finally{busy=false;emit()}
  }
  async function deliver(action:'submit'|'reconcile') {
    const v=view();if(action==='submit'?!v.canSubmit:!v.canReconcile)return
    const source=matching();if(!source)return
    busy=true;status='needs_receipt';entry=null;emit()
    try{
      // Source getter is synchronous and checks current connection/capability;
      // it is checked again after the busy publication before external dispatch.
      const current=matching();if(!current||scope(current)!==scope(source)){status='source_changed';return}
      const value=await options.port.deliver(requestId,action)
      acceptReceipt(value)
    }catch{status='uncertain'}finally{busy=false;emit()}
  }
  function finish():boolean {
    if(!view().canFinish||!receipt||!raw)return false
    try{retireAppliedCreateDraft(options.store,raw,receipt);disposed=true;return true}
    catch{status='occupied';blocked=true;emit();return false}
  }
  function discard(){
    if(!view().canDiscard)return
    // A non-durable local freeze may be discarded only while this controller
    // can prove enqueue was never attempted. The storage helper also verifies
    // the exact prior empty/unfrozen bytes; any stored frozen record is refused.
    try{discardCreateDraft(options.store,raw);record=null;raw=null;durable=false;fields=clone(initial);status='ready'}
    catch{status='occupied';blocked=true}emit()
  }
  return {view,edit,finish,retrySave:()=>{if(view().canPersist&&record&&persist(record)){status=record.frozen?'frozen':'dirty';emit()}},queue:()=>queue(),retryQueue:()=>queue(true),inspectQueue,submit:()=>deliver('submit'),reconcile:()=>deliver('reconcile'),discard,
    closePolicy:()=>busy||!!record&&!durable?'blocked' as const:record?'retain' as const:'close' as const,
    subscribe:(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener)}},version:()=>version,
    dispose:()=>{disposed=true;listeners.clear()}}
}
export type ManualCreateController=ReturnType<typeof createManualCreate>
