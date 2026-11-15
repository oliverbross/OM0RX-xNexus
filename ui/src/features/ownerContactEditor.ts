import type { Bridge, OutboxEntry, QsoDraft, Receipt } from '../xlog/api'
import { clearOwnerEditorDraft, loadOwnerEditorDraft, saveOwnerEditorDraft,
  type OwnerEditorDraft, type OwnerEditFields } from '../xlog/draftStore'
import { ownerEditFields, ownerEditQso, wireQso } from '../xlog/ownerEditFields'
export { ownerEditFields, ownerEditQso } from '../xlog/ownerEditFields'
import type { OwnerContactDetail } from './ownerContactDetail'

/** Supplied only by the authenticated native read/session adapter. Restored
 * drafts are never a source. Tokens remain in memory, never draft storage. */
export interface OwnerEditorSource {
  connection: { token: string; serverInstance: string; workspaceId: string; generation: number; writeGranted: boolean }
  read: { detail: OwnerContactDetail; deadlineMono: number;
    binding: { token: string; serverInstance: string; workspaceId: string; generation: number } } | null
}
type Port = Pick<Bridge, 'enqueue' | 'deliver' | 'outbox'>
const keys = ['contact_id','callsign','band','mode','started_at_unix_ms','frequency_hz','rst_sent','rst_received','grid','comment'] as const
const allowed = ['started_at_unix_ms','frequency_hz','rst_sent','rst_received','grid','comment'] as const
const uuid = (v: string) => /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(v)
const sameId = (a: string,b: string) => uuid(a)&&uuid(b)&&a.toLowerCase()===b.toLowerCase()
const sameQso = (a: QsoDraft,b: QsoDraft) => keys.every(k=>a[k]===b[k])
const scope = (s: OwnerEditorSource) => JSON.stringify(s.connection)
function clone<T>(v: T): T { return JSON.parse(JSON.stringify(v)) as T }
export type OwnerEditorStatus = 'ready'|'unavailable'|'occupied'|'invalid'|'invalid_input'|'storage_unavailable'|'conflict'|'foreign'
  |'dirty'|'frozen'|'queue_unknown'|'queued'|'needs_receipt'|'accepted'|'observed'|'rejected'|'applied'|'receipt_invalid'|'source_changed'
export interface OwnerEditorView {
  base: QsoDraft | null; fields: OwnerEditFields | null; requestId: string | null
  status: OwnerEditorStatus; error: string | null; receipt: Receipt | null
  editable: boolean; dirty: boolean; busy: boolean; canSave: boolean
  detailCurrent: boolean
  detail: OwnerContactDetail | null
  canSubmit: boolean; canReconcile: boolean; canInspectQueue: boolean; canRetryQueue: boolean; canReload: boolean
}

/** Owned by the caller outside ContactCard/read-key remounts. Construction and
 * inspection never write storage or dispatch outbox/HTTP calls. */
export function createOwnerContactEditor(options: {
  contactId: string; source: () => OwnerEditorSource | null; port: Port
  now?: () => number; requestId?: () => string
  /** Host-only local draft typing. Never grants enqueue/delivery authority. */
  localTyping?: boolean
}) {
  const now=options.now??(()=>performance.now()),newId=options.requestId??(()=>crypto.randomUUID())
  const load=loadOwnerEditorDraft(),listeners=new Set<()=>void>(),deadlines=new WeakMap<OwnerContactDetail,number>()
  let record=load.state==='current'?clone(load.value):null,raw=load.raw
  let blocked=load.state==='occupied'||load.state==='invalid'||load.state==='unavailable'?load.state:null
  if(record&&!sameId(record.base.contact_id,options.contactId)){record=null;blocked='occupied'}
  let version=0,busy=false,disposed=false,error:string|null=null,entry:OutboxEntry|null=null,receipt:Receipt|null=null
  let clockBad=false,lastClock:number|null=null
  let typingSeed:{scope:string;workspace:string;revision:number;base:QsoDraft}|null=null
  let operation:OwnerEditorStatus|null=record?.frozen?'frozen':null
  const emit=()=>{version++;for(const listener of listeners)listener()}
  function readClock():number|null{
    if(clockBad)return null
    let current:number
    try{current=now()}catch{clockBad=true;return null}
    if(!Number.isFinite(current)||current<0||lastClock!==null&&current<lastClock){clockBad=true;return null}
    lastClock=current;return current
  }
  function connection(): OwnerEditorSource | null {
    const source=options.source()
    if (!source || !source.connection.token || !source.connection.serverInstance || !source.connection.workspaceId
      || !Number.isSafeInteger(source.connection.generation)||source.connection.generation<0) return null
    return source
  }
  function currentRead(): OwnerEditorSource | null {
    const source=connection(),read=source?.read
    if (!source||!read) return null
    if(read.binding.token!==source.connection.token||read.binding.serverInstance!==source.connection.serverInstance
      ||read.binding.workspaceId!==source.connection.workspaceId||read.binding.generation!==source.connection.generation)return null
    const first=deadlines.get(read.detail)??read.deadlineMono
    deadlines.set(read.detail,Math.min(first,read.deadlineMono))
    const current=readClock()
    if (current===null||!Number.isFinite(read.deadlineMono)||!Number.isFinite(first)||current>=Math.min(first,read.deadlineMono)) return null
    return source
  }
  function admitted(requireWrite = true): OwnerEditorSource | null {
    const source=currentRead(),read=source?.read
    if (!source||!read||requireWrite&&!source.connection.writeGranted) return null
    const d=read.detail,b=d.editability.base
    if (d.schema_version!==1||d.status!=='found'||d.contact_id!==options.contactId||!d.fields)return null
    if(requireWrite&&(!d.editability.allowed||!allowed.every(k=>d.editability.allowed_fields.includes(k))
      || !Number.isSafeInteger(d.contact_revision)||d.contact_revision===null||d.contact_revision<1
      || !b||!wireQso(b)||!sameId(b.contact_id,options.contactId))) return null
    return source
  }
  function matched(source: OwnerEditorSource | null): boolean {
    return !!source&&!!record&&record.workspace_id===source.connection.workspaceId
      && source.read?.detail.contact_revision===record.base_revision
      && !!source.read.detail.editability.base&&sameQso(record.base,source.read.detail.editability.base)
  }
  function persist(next: OwnerEditorDraft): boolean {
    record=clone(next)
    try {raw=saveOwnerEditorDraft(record,raw);error=null;return true}
    catch(e){error=e instanceof Error?e.message:String(e);if(error==='draft_occupied')blocked='occupied';return false}
  }
  function inspect(): OwnerEditorView {
    const source=admitted(),facts=admitted(false),connected=connection(),matching=record?matched(source):!!source
    if(options.localTyping){
      if(readClock()===null||!connected?.connection.writeGranted||typingSeed&&typingSeed.scope!==scope(connected))typingSeed=null
      const observed=currentRead()?.read?.detail
      // Refresh/expiry can withhold evidence; a known contradictory answer is
      // different. Preserve the draft but stop typing until explicit resolution.
      if(typingSeed&&observed&&(!source||observed.contact_revision!==typingSeed.revision
        ||!observed.editability.base||!sameQso(observed.editability.base,typingSeed.base)))typingSeed=null
      if(!typingSeed&&source&&matching&&source.read?.detail.editability.base)typingSeed={
        scope:scope(source),workspace:source.connection.workspaceId,revision:source.read.detail.contact_revision!,
        base:clone(source.read.detail.editability.base)}
    }
    const candidate=facts?.read?.detail.editability.base
    const base=record?.base??typingSeed?.base??(candidate&&wireQso(candidate)?candidate:null)
    const fields=record?.fields??(base?ownerEditFields(base):null)
    const q=base&&fields?ownerEditQso(base,fields):null
    const dirty=!!base&&!!fields&&JSON.stringify(fields)!==JSON.stringify(ownerEditFields(base))
    let status:OwnerEditorStatus=blocked==='unavailable'?'storage_unavailable':blocked??operation??(dirty?'dirty':'ready')
    if (!blocked&&!operation) {
      if (record&&connected&&record.workspace_id!==connected.connection.workspaceId)status='foreign'
      else if (record&&source&&!matching)status='conflict'
      else if (!source)status='unavailable'
      else if (dirty&&!q)status='invalid_input'
      else if(error)status='storage_unavailable'
    }
    const frozen=!!record?.frozen
    const pending=entry?.state==='Pending'
    const terminal=!!entry&&typeof entry.state==='object'||!!receipt&&['applied','observed','rejected'].includes(receipt.status)
    const reloadCurrent=!!source&&(!record||record.workspace_id===source.connection.workspaceId)
      &&(receipt?.status!=='applied'||source?.read?.detail.contact_revision!=null
        &&source.read.detail.contact_revision>=receipt.contact_revision!)
    return {base:base?clone(base):null,fields:fields?{...fields}:null,requestId:record?.request_id??null,status,error,
      receipt:receipt?{...receipt}:null,detailCurrent:record?matched(facts):!!facts,
      detail:(!record||matched(facts))?facts?.read?.detail??null:null,
      editable:!disposed&&!busy&&!blocked&&!frozen&&(matching||!!typingSeed),
      dirty,busy,canSave:!disposed&&!busy&&!blocked&&!frozen&&matching&&dirty&&!!q,
      canSubmit:!disposed&&!busy&&!blocked&&frozen&&pending&&matching,
      canReconcile:!disposed&&!busy&&frozen&&!!connected&&connected.connection.workspaceId===record?.workspace_id&&!terminal,
      canInspectQueue:!disposed&&!busy&&frozen,
      canRetryQueue:!disposed&&!busy&&!blocked&&frozen&&matching&&!entry&&!receipt
        &&['frozen','queue_unknown','source_changed'].includes(operation??'frozen'),
      canReload:!disposed&&!busy&&!blocked&&reloadCurrent&&(!frozen||!!terminal)}
  }
  function set<K extends keyof OwnerEditFields>(key: K,value: OwnerEditFields[K]): boolean {
    if (!inspect().editable)return false
    const source=admitted()
    const seed=typingSeed??(source?.read?.detail.editability.base?{workspace:source.connection.workspaceId,
      revision:source.read.detail.contact_revision!,base:source.read.detail.editability.base}:null)
    if (!record&&!seed)return false
    const next:OwnerEditorDraft=record??{version:2,kind:'owner_contact_edit',request_id:newId(),workspace_id:seed!.workspace,
      base_revision:seed!.revision,base:clone(seed!.base),fields:ownerEditFields(seed!.base),frozen:null}
    const result=persist({...next,fields:{...next.fields,[key]:value}})
    emit();return result
  }
  function acceptReceipt(r: Receipt): boolean {
    if (!record||r.request_id!==record.request_id||r.workspace_id!==record.workspace_id||!r.server_instance
      || !['accepted','applied','observed','rejected','uncertain'].includes(r.status)
      || r.status==='applied'&&(!r.contact_id||!sameId(r.contact_id,record.base.contact_id)||!Number.isSafeInteger(r.contact_revision)||r.contact_revision==null||r.contact_revision<=record.base_revision)) {
      receipt=null;operation='receipt_invalid';return false
    }
    receipt={...r};operation=r.status==='uncertain'?'needs_receipt':r.status;return true
  }
  function acceptEntries(entries: OutboxEntry[]): void {
    entry=null;receipt=null
    if (!record?.frozen||!Array.isArray(entries)||entries.length>10_000)throw Error('invalid_queue')
    const found=entries.filter(e=>e.queued.request_id===record!.request_id)
    if(found.length===0){entry=null;operation='queue_unknown';return}
    const item=found[0]
    if(found.length!==1||item.queued.workspace_id!==record.workspace_id||item.queued.mutation.kind!=='logbook.update'
      ||item.queued.mutation.expected_contact_revision!==record.base_revision||!sameQso(item.queued.mutation.qso,record.frozen))throw Error('queue_identity_mismatch')
    if(item.state==='Pending')operation='queued'
    else if(item.state==='NeedsReceipt')operation='needs_receipt'
    else if(!['applied','observed','rejected'].includes(item.state.Settled.status)||!acceptReceipt(item.state.Settled))throw Error('receipt_invalid')
    entry=clone(item)
  }
  async function enqueue(retry: boolean): Promise<void> {
    const view=inspect()
    if(!(retry?view.canRetryQueue:view.canSave)||!record)return
    busy=true;error=null;emit()
    const source=admitted(),captured=source?scope(source):null
    try {
      if(!source||!matched(source))throw Error('source_changed')
      const q=record.frozen??ownerEditQso(record.base,record.fields)
      if(!q||!persist({...record,frozen:q}))return
      operation='frozen'
      const entries=await options.port.enqueue({kind:'logbook.update',qso:clone(q),expected_contact_revision:record.base_revision},record.request_id,record.workspace_id)
      if(disposed)return
      if(!connection()||scope(connection()!)!==captured){operation='source_changed';return}
      acceptEntries(entries)
    }catch(e){if(!disposed){error=e instanceof Error?e.message:String(e);operation=record?.frozen?'queue_unknown':null}}
    finally{busy=false;emit()}
  }
  async function inspectQueue(): Promise<void> {
    if(!inspect().canInspectQueue)return
    busy=true;error=null;emit()
    try{const entries=await options.port.outbox();if(!disposed)acceptEntries(entries)}
    catch(e){if(!disposed){error=e instanceof Error?e.message:String(e);operation='queue_unknown'}}
    finally{busy=false;emit()}
  }
  async function deliver(action:'submit'|'reconcile'): Promise<void> {
    const view=inspect()
    if(!(action==='submit'?view.canSubmit:view.canReconcile)||!record)return
    busy=true;error=null;emit()
    const source=connection(),captured=source?scope(source):null
    try{
      if(!source||source.connection.workspaceId!==record.workspace_id)throw Error('source_changed')
      const result=await options.port.deliver(record.request_id,action)
      if(disposed)return
      if(!connection()||scope(connection()!)!==captured){operation='source_changed';return}
      if(result.server_instance!==source.connection.serverInstance){operation='receipt_invalid';return}
      acceptReceipt(result)
      // Durable local state remains the source for subsequent dispatch/restart.
      entry=null
    }catch(e){if(!disposed){error=e instanceof Error?e.message:String(e);operation='needs_receipt';entry=null}}
    finally{busy=false;emit()}
  }
  /** Caller must obtain an explicit discard/reload decision first. */
  function reload(): boolean {
    if(!inspect().canReload)return false
    if(record){try{clearOwnerEditorDraft(raw!)}catch(e){error=e instanceof Error?e.message:String(e);emit();return false}}
    record=null;raw=null;entry=null;receipt=null;operation=null;error=null;typingSeed=null;emit();return true
  }
  /** Explicit unqueued discard only; never deletes a possibly queued intent. */
  function discard(): boolean {
    if(busy||disposed||blocked||record?.frozen)return false
    if(record&&raw!==null){try{clearOwnerEditorDraft(raw)}catch(e){error=e instanceof Error?e.message:String(e);emit();return false}}
    record=null;raw=null;error=null;typingSeed=null;emit();return true
  }
  return {contactId:options.contactId,inspect,set,save:()=>enqueue(false),retryQueue:()=>enqueue(true),inspectQueue,
    submit:()=>deliver('submit'),reconcile:()=>deliver('reconcile'),reload,discard,
    closePolicy:(): 'blocked'|'confirm-discard'|'retain'|'close' => busy?'blocked':record?.frozen?'retain':inspect().dirty?'confirm-discard':'close',
    subscribe:(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener)}},version:()=>version,
    dispose:()=>{disposed=true;typingSeed=null;listeners.clear()}}
}
export type OwnerContactEditor = ReturnType<typeof createOwnerContactEditor>
