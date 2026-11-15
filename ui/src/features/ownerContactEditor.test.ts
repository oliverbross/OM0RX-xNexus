// @vitest-environment jsdom
import { beforeEach, afterEach, expect, it, vi } from 'vitest'
import producer from '../xlog/fixtures/contact-detail-producer.json'
import type { OwnerContactDetail } from './ownerContactDetail'
import { createOwnerContactEditor, ownerEditFields, ownerEditQso, type OwnerEditorSource } from './ownerContactEditor'
import { DRAFT_KEY, loadOwnerEditorDraft } from '../xlog/draftStore'
import type { OutboxEntry, Receipt } from '../xlog/api'

const requestId='11111111-1111-4111-8111-111111111111'
function fixture(){return JSON.parse(JSON.stringify(producer.found)) as OwnerContactDetail}
function setup(){
  let clock=100
  const detail=fixture(),binding={token:'memory-only-token',serverInstance:'owner-A',workspaceId:'workspace-A',generation:1}
  let source:OwnerEditorSource|null={connection:{...binding,writeGranted:true},read:{detail,deadlineMono:1000,binding:{...binding}}}
  let entries:OutboxEntry[]=[]
  const port={enqueue:vi.fn(async (mutation:OutboxEntry['queued']['mutation'],id:string,workspace:string)=>{
    entries=[{queued:{version:1,request_id:id,workspace_id:workspace,mutation},state:'Pending'}];return entries
  }),outbox:vi.fn(async()=>entries),deliver:vi.fn(async():Promise<Receipt>=>({request_id:requestId,workspace_id:'workspace-A',server_instance:'owner-A',status:'uncertain',message:null}))}
  const make=()=>createOwnerContactEditor({contactId:detail.contact_id,source:()=>source,port,now:()=>clock,requestId:()=>requestId})
  return {detail,port,make,editor:make(),time:(v:number)=>{clock=v},source:()=>source!,replace:(s:OwnerEditorSource|null)=>{source=s},entries:()=>entries}
}
beforeEach(()=>localStorage.clear())
afterEach(()=>vi.restoreAllMocks())

it('viewing the actual producer base never stores or dispatches; preserves null Hz and stored MFSK/uppercase UUID',()=>{
  const s=setup(),v=s.editor.inspect()
  expect(v.editable).toBe(true);expect(v.base).toEqual(s.detail.editability.base)
  expect(v.fields?.frequency).toBe('');expect(localStorage.length).toBe(0)
  expect(s.port.enqueue).not.toHaveBeenCalled();expect(s.port.outbox).not.toHaveBeenCalled()
})
it('requires write grant, original lease and matching connection-bound read, not business permission',()=>{
  const s=setup();s.source().connection.writeGranted=false
  expect(s.editor.set('comment','not admitted')).toBe(false)
  s.source().connection.writeGranted=true;s.source().read!.binding.token='retired'
  expect(s.editor.inspect().editable).toBe(false)
  s.source().read!.binding.token=s.source().connection.token;s.time(1000)
  expect(s.editor.inspect().canSave).toBe(false)
  expect(localStorage.length).toBe(0)
})
it('same payload reference cannot renew its original lease',()=>{
  const s=setup();expect(s.editor.inspect().editable).toBe(true)
  s.source().read!.deadlineMono=5000;s.time(1000)
  expect(s.editor.inspect().editable).toBe(false)
})
it.each([NaN,Infinity,-Infinity,-1])('invalid clock %s latches editing off without dispatch or draft loss',async invalid=>{
  const s=setup();s.editor.set('comment','retained');const saved=localStorage.getItem(DRAFT_KEY)
  s.time(invalid);expect(s.editor.inspect().editable).toBe(false)
  s.time(200);expect(s.editor.inspect().canSave).toBe(false)
  await s.editor.save();await s.editor.submit();await s.editor.retryQueue()
  expect(s.port.enqueue).not.toHaveBeenCalled();expect(s.port.deliver).not.toHaveBeenCalled()
  expect(localStorage.getItem(DRAFT_KEY)).toBe(saved);expect(s.editor.inspect().fields?.comment).toBe('retained')
})
it('a throwing clock cannot escape inspection and permanently retires write admission',async()=>{
  const s=setup();s.editor.set('comment','retained');let throws=true
  const restored=createOwnerContactEditor({contactId:s.detail.contact_id,source:s.source,port:s.port,
    now:()=>{if(throws)throw Error('clock unavailable');return 100}})
  expect(()=>restored.inspect()).not.toThrow();throws=false
  expect(restored.inspect().editable).toBe(false);await restored.save();await restored.submit()
  expect(s.port.enqueue).not.toHaveBeenCalled();expect(s.port.deliver).not.toHaveBeenCalled()
  expect(restored.inspect().fields?.comment).toBe('retained')
})
it('a backward clock, including rollback after expiry, cannot revive editing even with a newer read',async()=>{
  const s=setup();s.editor.set('comment','retained');s.time(1000)
  expect(s.editor.inspect().editable).toBe(false)
  s.time(500);expect(s.editor.inspect().editable).toBe(false)
  s.time(1200);s.replace({...s.source(),read:{...s.source().read!,detail:fixture(),deadlineMono:5000}})
  expect(s.editor.inspect().editable).toBe(false);await s.editor.save();await s.editor.submit()
  expect(s.port.enqueue).not.toHaveBeenCalled();expect(s.port.deliver).not.toHaveBeenCalled()
  const fresh=s.make();expect(fresh.inspect().editable).toBe(true);expect(fresh.inspect().requestId).toBe(requestId)
})
it('backward time before expiry also latches, rather than merely comparing with the deadline',()=>{
  const s=setup();expect(s.editor.inspect().editable).toBe(true)
  s.time(99);expect(s.editor.inspect().editable).toBe(false)
  s.time(101);expect(s.editor.inspect().editable).toBe(false)
})
it('clock failure does not prevent explicit local queue inspection or same-ID receipt reconciliation',async()=>{
  const s=setup();s.editor.set('comment','retained');await s.editor.save();s.time(NaN)
  expect(s.editor.inspect().canSubmit).toBe(false)
  await s.editor.inspectQueue();expect(s.port.outbox).toHaveBeenCalledTimes(1)
  await s.editor.reconcile();expect(s.port.deliver).toHaveBeenCalledWith(requestId,'reconcile')
  expect(s.editor.inspect().canSave).toBe(false)
})
it('rejects wrong outer ID and invalid/zero revision despite allowed business projection',()=>{
  const s=setup();s.detail.contact_id=s.detail.contact_id.toUpperCase()
  expect(s.editor.inspect().editable).toBe(false)
  s.detail.contact_id=producer.found.contact_id;s.detail.contact_revision=0
  expect(s.editor.inspect().editable).toBe(false)
})
it('preserves exact untouched values and multiline edits without UTC/frequency normalization',()=>{
  const base=fixture().editability.base!,fields=ownerEditFields(base)
  expect(ownerEditQso(base,fields)).toEqual(base)
  expect(ownerEditQso(base,{...fields,comment:' first\r\nsecond\t ',grid:'fn42'})).toEqual({...base,comment:' first\r\nsecond\t ',grid:'fn42'})
  expect(ownerEditQso(base,{...fields,frequency:'14.074001'})?.frequency_hz).toBe(14074001)
  expect(ownerEditQso(base,{...fields,frequency:'0'})).toBeNull()
  expect(ownerEditQso(base,{...fields,frequency:'14.0740001'})).toBeNull()
  expect(ownerEditQso(base,{...fields,date:'2025-02-30'})).toBeNull()
})
it('retains typed fields and same request after expiry, conflict, foreign workspace and reconnect',()=>{
  const s=setup();s.editor.set('comment','my draft');const before=localStorage.getItem(DRAFT_KEY)
  s.time(1000);expect(s.editor.inspect().status).toBe('unavailable')
  s.replace({...s.source(),read:{...s.source().read!,detail:{...s.detail,contact_revision:2},deadlineMono:5000}})
  expect(s.editor.inspect().status).toBe('conflict')
  s.source().connection.workspaceId='workspace-B';expect(s.editor.inspect().status).toBe('foreign')
  expect(s.editor.inspect().fields?.comment).toBe('my draft');expect(localStorage.getItem(DRAFT_KEY)).toBe(before)
  expect(s.editor.inspect().requestId).toBe(requestId)
})
it('latches same-turn Save and freezes a complete exact mutation before enqueue; close cannot bypass pending',async()=>{
  const s=setup();let resolve!:(entries:OutboxEntry[])=>void
  s.port.enqueue.mockImplementation(()=>new Promise(r=>{resolve=r}))
  s.editor.set('comment','queued\ntext')
  const first=s.editor.save(),second=s.editor.save()
  expect(s.port.enqueue).toHaveBeenCalledTimes(1);expect(s.editor.closePolicy()).toBe('blocked')
  const stored=loadOwnerEditorDraft();expect(stored.state).toBe('current')
  if(stored.state==='current')expect(stored.value.frozen?.comment).toBe('queued\ntext')
  expect(s.editor.set('grid','XX00')).toBe(false)
  resolve([]);await Promise.all([first,second])
  expect(s.editor.inspect().status).toBe('queue_unknown');expect(s.editor.closePolicy()).toBe('retain')
})
it('recovers enqueue reply loss from exact local outbox, without new UUID or dispatch on restart',async()=>{
  const s=setup(),enqueue=s.port.enqueue.getMockImplementation()!
  s.port.enqueue.mockImplementation(async(...args)=>{await enqueue(...args);throw Error('reply lost')})
  s.editor.set('comment','queued');await s.editor.save()
  const restored=s.make();expect(restored.inspect().requestId).toBe(requestId)
  expect(s.port.enqueue).toHaveBeenCalledTimes(1)
  await restored.inspectQueue();expect(restored.inspect().status).toBe('queued')
  expect(s.port.enqueue).toHaveBeenCalledTimes(1);expect(s.port.deliver).not.toHaveBeenCalled()
})
it('local queue recovery refuses same ID with a different immutable payload',async()=>{
  const s=setup();s.editor.set('comment','queued');await s.editor.save()
  s.entries()[0].queued.mutation.qso.comment='different'
  await s.editor.inspectQueue();expect(s.editor.inspect().error).toBe('queue_identity_mismatch')
  expect(s.editor.inspect().canSubmit).toBe(false)
})
it('a malformed frozen/display pair remains preserved and cannot be retried or submitted after restart',async()=>{
  const s=setup();s.editor.set('comment','visible');await s.editor.save()
  const value=JSON.parse(localStorage.getItem(DRAFT_KEY)!)
  value.frozen.comment='hidden'
  const raw=JSON.stringify(value);localStorage.setItem(DRAFT_KEY,raw)
  const recovered=s.make();expect(recovered.inspect().status).toBe('invalid')
  await recovered.save();await recovered.retryQueue();await recovered.submit()
  expect(s.port.enqueue).toHaveBeenCalledTimes(1);expect(s.port.deliver).not.toHaveBeenCalled()
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
it('connection/workspace change during enqueue preserves the frozen operation and ignores its late reply without auto-resend',async()=>{
  const s=setup();let resolve!:(entries:OutboxEntry[])=>void
  s.port.enqueue.mockImplementation(()=>new Promise(r=>{resolve=r}))
  s.editor.set('comment','one immutable correction');const pending=s.editor.save()
  const saved=localStorage.getItem(DRAFT_KEY),call=s.port.enqueue.mock.calls[0]
  s.source().connection={...s.source().connection,token:'new-token',workspaceId:'workspace-B',generation:2}
  resolve([{queued:{version:1,request_id:call[1],workspace_id:call[2],mutation:call[0]},state:'Pending'}])
  await pending
  expect(s.editor.inspect().status).toBe('source_changed');expect(s.editor.inspect().requestId).toBe(requestId)
  expect(s.editor.inspect().canSubmit).toBe(false);expect(s.editor.inspect().canRetryQueue).toBe(false)
  await s.editor.save();await s.editor.retryQueue();await s.editor.submit()
  expect(s.port.enqueue).toHaveBeenCalledTimes(1);expect(s.port.deliver).not.toHaveBeenCalled()
  expect(localStorage.getItem(DRAFT_KEY)).toBe(saved)
})
it('explicit queue retry retains identical ID/payload and requires fresh authority',async()=>{
  const s=setup();s.port.enqueue.mockRejectedValueOnce(Error('local enqueue unknown'))
  s.editor.set('comment','one');await s.editor.save();s.time(1000)
  await s.editor.retryQueue();expect(s.port.enqueue).toHaveBeenCalledTimes(1)
  s.replace({...s.source(),read:{...s.source().read!,detail:fixture(),deadlineMono:5000}})
  await s.editor.retryQueue();expect(s.port.enqueue).toHaveBeenCalledTimes(2)
  expect(s.port.enqueue.mock.calls[1]).toEqual(s.port.enqueue.mock.calls[0])
})
it('requires explicit Submit and same-ID reconciliation; uncertain is never applied',async()=>{
  const s=setup();s.editor.set('comment','one');await s.editor.save()
  expect(s.editor.inspect().status).toBe('queued');expect(s.port.deliver).not.toHaveBeenCalled()
  await s.editor.submit();expect(s.editor.inspect().status).toBe('needs_receipt')
  await s.editor.reconcile();expect(s.port.deliver.mock.calls).toEqual([[requestId,'submit'],[requestId,'reconcile']])
})
it.each(['accepted','observed','rejected'] as const)('%s remains its actual status, never applied',async status=>{
  const s=setup();s.editor.set('comment','one');await s.editor.save()
  s.port.deliver.mockResolvedValue({request_id:requestId,workspace_id:'workspace-A',server_instance:'owner-A',status,message:null})
  await s.editor.submit();expect(s.editor.inspect().status).toBe(status)
})
it('accepts only a matching applied receipt and waits for its new revision before explicit reload',async()=>{
  const s=setup();s.editor.set('comment','one');await s.editor.save()
  s.port.deliver.mockResolvedValue({request_id:requestId,workspace_id:'workspace-A',server_instance:'owner-A',status:'applied',message:null,contact_id:producer.found.contact_id,contact_revision:2})
  await s.editor.submit();expect(s.editor.inspect().status).toBe('applied');expect(s.editor.inspect().canReconcile).toBe(false)
  expect(s.editor.reload()).toBe(false)
  s.replace({...s.source(),read:{...s.source().read!,detail:{...fixture(),contact_revision:2}}})
  expect(s.editor.reload()).toBe(true);expect(localStorage.getItem(DRAFT_KEY)).toBeNull()
})
it('wrong receipt contact/revision cannot display applied',async()=>{
  const s=setup();s.editor.set('comment','one');await s.editor.save()
  s.port.deliver.mockResolvedValue({request_id:requestId,workspace_id:'workspace-A',server_instance:'owner-A',status:'applied',message:null,contact_id:'ffffffff-ffff-4fff-8fff-ffffffffffff',contact_revision:2})
  await s.editor.submit();expect(s.editor.inspect().status).toBe('receipt_invalid')
})
it('late submit from old connection cannot publish under a replacement session',async()=>{
  const s=setup();s.editor.set('comment','one');await s.editor.save()
  let resolve!:(r:Receipt)=>void;s.port.deliver.mockImplementation(()=>new Promise(r=>{resolve=r}))
  const pending=s.editor.submit();s.source().connection.token='new-session'
  resolve({request_id:requestId,workspace_id:'workspace-A',server_instance:'owner-A',status:'applied',message:null,contact_id:producer.found.contact_id,contact_revision:2})
  await pending;expect(s.editor.inspect().status).toBe('source_changed');expect(s.editor.inspect().receipt).toBeNull()
})
it('preserves draft on failed storage and dispatches nothing until exact intent can be persisted',async()=>{
  const s=setup();s.editor.set('comment','before')
  const saved=localStorage.getItem(DRAFT_KEY),set=vi.spyOn(Storage.prototype,'setItem').mockImplementation(()=>{throw Error('disk full')})
  s.editor.set('comment','after');await s.editor.save()
  expect(localStorage.getItem(DRAFT_KEY)).toBe(saved);expect(s.editor.inspect().fields?.comment).toBe('after')
  expect(s.port.enqueue).not.toHaveBeenCalled();set.mockRestore()
  const frozen=s.editor.inspect()
  await s.editor.retryQueue();expect(s.port.enqueue).toHaveBeenCalledTimes(1)
  const retained=loadOwnerEditorDraft();expect(retained.state).toBe('current')
  if(retained.state==='current'){
    expect(retained.value.request_id).toBe(frozen.requestId)
    expect(retained.value.frozen).toEqual(s.port.enqueue.mock.calls[0][0].qso)
    expect(retained.value.frozen?.comment).toBe('after')
  }
})
it('another contact editor cannot claim an occupied v2 draft or show its base',()=>{
  const s=setup();s.editor.set('comment','original')
  const other=createOwnerContactEditor({contactId:'ffffffff-ffff-4fff-8fff-ffffffffffff',source:()=>s.source(),port:s.port})
  expect(other.inspect().status).toBe('occupied');expect(other.inspect().base).toBeNull()
  expect(other.reload()).toBe(false);expect(other.discard()).toBe(false)
})
it('explicit unqueued discard works offline, while frozen intents can only be retained',async()=>{
  const s=setup();s.editor.set('comment','draft');s.replace(null)
  expect(s.editor.closePolicy()).toBe('confirm-discard');expect(s.editor.discard()).toBe(true)
  const next=setup();next.editor.set('comment','frozen');await next.editor.save()
  expect(next.editor.discard()).toBe(false);expect(next.editor.closePolicy()).toBe('retain')
})

it('opt-in local typing never seeds from a restored record without fresh read/write authority',async()=>{
  const s=setup();s.editor.set('comment','saved');const raw=localStorage.getItem(DRAFT_KEY)
  s.source().read=null
  const restored=createOwnerContactEditor({contactId:s.detail.contact_id,source:s.source,port:s.port,localTyping:true,now:()=>100})
  expect(restored.inspect().fields?.comment).toBe('saved');expect(restored.set('comment','unadmitted')).toBe(false)
  await restored.save();expect(s.port.enqueue).not.toHaveBeenCalled();expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
it('opt-in local typing latches bad clocks and cannot seed again from an apparently fresh read',()=>{
  const s=setup();let now=100
  const editor=createOwnerContactEditor({contactId:s.detail.contact_id,source:s.source,port:s.port,localTyping:true,now:()=>now})
  expect(editor.set('comment','saved')).toBe(true);const raw=localStorage.getItem(DRAFT_KEY)
  now=NaN;expect(editor.set('comment','lost')).toBe(false)
  now=100;expect(editor.inspect().editable).toBe(false);expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
it('a fresh changed base before the first keystroke replaces only the unpersisted seed',()=>{
  const s=setup(),editor=createOwnerContactEditor({contactId:s.detail.contact_id,source:s.source,port:s.port,localTyping:true,now:()=>100})
  expect(editor.inspect().editable).toBe(true);expect(localStorage.length).toBe(0)
  const newer=fixture();newer.contact_revision!++;newer.editability.base!.comment='new owner base'
  s.source().read={...s.source().read!,detail:newer}
  expect(editor.set('comment','typed from new base')).toBe(true)
  const saved=loadOwnerEditorDraft();expect(saved.state).toBe('current')
  if(saved.state==='current'){expect(saved.value.base_revision).toBe(newer.contact_revision);expect(saved.value.base.comment).toBe('new owner base')}
})
it.each(['revision','base','deleted','not_found','readonly'] as const)('known %s retires local typing without changing saved fields',kind=>{
  const s=setup(),editor=createOwnerContactEditor({contactId:s.detail.contact_id,source:s.source,port:s.port,localTyping:true,now:()=>100})
  editor.set('comment','retained');const raw=localStorage.getItem(DRAFT_KEY),next=fixture()
  if(kind==='revision')next.contact_revision!++
  if(kind==='base')next.editability.base!.grid='AA00'
  if(kind==='deleted'||kind==='not_found'){next.status=kind;next.fields=null;next.editability={allowed:false,reason:kind,allowed_fields:[],base:null}}
  if(kind==='readonly')next.editability.allowed=false
  s.source().read={...s.source().read!,detail:next}
  expect(editor.set('comment','refused')).toBe(false)
  s.source().read=null
  expect(editor.set('comment','still refused')).toBe(false)
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw);expect(editor.inspect().fields?.comment).toBe('retained')
})
