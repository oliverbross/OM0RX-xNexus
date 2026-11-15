import { describe,it,expect,vi } from 'vitest'
import type { Bridge,OutboxEntry,Receipt } from './api'
import { createManualCreate,type ManualCreateAuthority } from './manualCreateController'
import { manualCreateQso } from './manualCreateFields'
import { MANUAL_CREATE_KEY,type CreateStorage,type ManualCreateRecord } from './manualCreateStore'
const req='aaaaaaaa-1234-4234-8234-123456789abc',contact='bbbbbbbb-1234-4234-8234-123456789abc'
function setup(){
 const values=new Map<string,string>();let fail=false,clock=1,source:ManualCreateAuthority|null={workspaceId:'workspace',serverInstance:'owner',connectionToken:'token',generation:1,deadline:5000,writeGranted:true}
 const store:CreateStorage={getItem:k=>values.get(k)??null,setItem:(k,v)=>{if(fail)throw Error('denied');values.set(k,v)},removeItem:k=>{values.delete(k)}}
 let rows:OutboxEntry[]=[],ids=0
 const applied:Receipt={request_id:req,workspace_id:'workspace',server_instance:'owner',status:'applied',message:null,contact_id:contact,contact_revision:1}
 const port:Pick<Bridge,'enqueue'|'deliver'|'outbox'>={enqueue:vi.fn(async(mutation,request_id,workspace_id)=>{
   rows=[{queued:{version:1,request_id,workspace_id,mutation},state:'Pending'}];return rows
 }),deliver:vi.fn(async()=>applied),outbox:vi.fn(async()=>rows)}
 const make=()=>createManualCreate({source:()=>source,port,store,now:()=>clock,utc:()=>Date.UTC(2024,0,1),id:()=>++ids%2?req:contact})
 const editor=make(),fill=()=>{editor.edit('call','K1ABC');editor.edit('band','20m');editor.edit('mode','FT8')}
 return{editor,make,fill,values,port,applied,store,setRows:(v:OutboxEntry[])=>{rows=v},setFail:(v:boolean)=>{fail=v},setClock:(v:number)=>{clock=v},setSource:(v:ManualCreateAuthority|null)=>{source=v},source:()=>source}
}
describe('manual create durable controller',()=>{
 it('view/open never writes/dispatches, first edit binds and preserves partial raw typing',()=>{
  const h=setup();expect(h.editor.view().fields.call).toBe('');expect(h.values.size).toBe(0)
  expect(h.port.enqueue).not.toHaveBeenCalled();expect(h.port.outbox).not.toHaveBeenCalled()
  h.fill();h.editor.edit('whenDate','2024-0');expect(JSON.parse(h.values.get(MANUAL_CREATE_KEY)!).fields.whenDate).toBe('2024-0');expect(h.editor.view().canQueue).toBe(false)
 })
 it('requires explicit queue then Submit and matches the exact Applied receipt before acknowledgement',async()=>{
  const h=setup();h.fill();await h.editor.queue();expect(h.editor.view().status).toBe('queued');expect(h.port.deliver).not.toHaveBeenCalled()
  expect(h.editor.finish()).toBe(false);await h.editor.submit();expect(h.editor.view().status).toBe('applied');expect(h.values.has(MANUAL_CREATE_KEY)).toBe(true)
  expect(h.editor.finish()).toBe(true);expect(h.values.has(MANUAL_CREATE_KEY)).toBe(false)
 })
 it('freezes before enqueue and same-turn double Queue dispatches once; dirty close retains',async()=>{
  const h=setup();h.fill();let release!:(v:OutboxEntry[])=>void
  vi.mocked(h.port.enqueue).mockImplementation(()=>new Promise(resolve=>{release=resolve}))
  const pending=h.editor.queue();void h.editor.queue();expect(h.port.enqueue).toHaveBeenCalledTimes(1)
  const saved=JSON.parse(h.values.get(MANUAL_CREATE_KEY)!) as ManualCreateRecord
  expect(saved.frozen?.callsign).toBe('K1ABC');expect(h.editor.closePolicy()).toBe('blocked')
  h.editor.edit('comment','ignored');release([]);await pending;expect(h.editor.view().fields.comment).toBe('');expect(h.editor.closePolicy()).toBe('retain')
 })
 it('freeze storage failure dispatches nothing, retains IDs and requires explicit durable save/inspection',async()=>{
  const h=setup();h.fill();h.setFail(true);await h.editor.queue();expect(h.port.enqueue).not.toHaveBeenCalled();expect(h.editor.closePolicy()).toBe('blocked')
  expect(h.editor.view().requestId).toBe(req);h.setFail(false);h.editor.retrySave();await h.editor.inspectQueue();await h.editor.retryQueue()
  expect(h.port.enqueue).toHaveBeenCalledTimes(1);expect(vi.mocked(h.port.enqueue).mock.calls[0][1]).toBe(req)
 })
 it('restored immutable intent requires full queue inspection before receipt/delivery',async()=>{
  const h=setup();h.fill();await h.editor.queue();const restored=h.make()
  expect(restored.view().canReconcile).toBe(false);expect(h.port.outbox).not.toHaveBeenCalled()
  await restored.inspectQueue();await restored.submit();expect(h.port.deliver).toHaveBeenCalledWith(req,'submit')
 })
 it.each(['body','workspace','contact','request'])('refuses conflicting retained queue %s after restart',async(change)=>{
  const h=setup();h.fill();await h.editor.queue();const r=JSON.parse(h.values.get(MANUAL_CREATE_KEY)!) as ManualCreateRecord
  const row:OutboxEntry={queued:{version:1,request_id:req,workspace_id:r.workspace_id,mutation:{kind:'logbook.create',qso:r.frozen!}},state:'NeedsReceipt'}
  if(change==='body')row.queued.mutation.qso.comment='hidden payload'
  if(change==='workspace')row.queued.workspace_id='foreign'
  if(change==='contact')row.queued.mutation.qso.contact_id='cccccccc-1234-4234-8234-123456789abc'
  if(change==='request')row.queued.request_id='cccccccc-1234-4234-8234-123456789abc'
  h.setRows([row]);const restored=h.make();await restored.inspectQueue();await restored.reconcile();await restored.submit()
  expect(h.port.deliver).not.toHaveBeenCalled();expect(restored.view().canFinish).toBe(false);expect(h.values.has(MANUAL_CREATE_KEY)).toBe(true)
 })
 it('loss/uncertainty reconciles same ID with no automatic send/new operation',async()=>{
  const h=setup();h.fill();await h.editor.queue();vi.mocked(h.port.deliver).mockRejectedValueOnce(Error('lost'))
  await h.editor.submit();expect(h.editor.view().status).toBe('uncertain');expect(h.port.deliver).toHaveBeenCalledTimes(1)
  await h.editor.reconcile();expect(h.port.deliver).toHaveBeenLastCalledWith(req,'reconcile');expect(h.port.enqueue).toHaveBeenCalledTimes(1)
 })
 it('workspace replacement during enqueue keeps exact operation but cannot Submit under the new owner',async()=>{
  const h=setup();h.fill();let release!:(rows:OutboxEntry[])=>void
  vi.mocked(h.port.enqueue).mockImplementation(()=>new Promise(resolve=>{release=resolve}));const pending=h.editor.queue()
  h.setSource({...h.source()!,workspaceId:'other',connectionToken:'new',generation:2})
  const record=JSON.parse(h.values.get(MANUAL_CREATE_KEY)!) as ManualCreateRecord
  release([{queued:{version:1,workspace_id:record.workspace_id,request_id:req,mutation:{kind:'logbook.create',qso:record.frozen!}},state:'Pending'}]);await pending
  await h.editor.submit();expect(h.port.deliver).not.toHaveBeenCalled();expect(h.editor.view().workspaceId).toBe('workspace')
 })
 it('capability loss/expiry forbids create, clock rollback never revives it',async()=>{
  const h=setup();h.fill();h.setSource({...h.source()!,writeGranted:false});await h.editor.queue();expect(h.port.enqueue).not.toHaveBeenCalled()
  h.setSource({...h.source()!,writeGranted:true});h.setClock(5001);expect(h.editor.view().canQueue).toBe(false)
  h.setClock(2);expect(h.editor.view().canQueue).toBe(false);expect(h.values.has(MANUAL_CREATE_KEY)).toBe(true)
 })
 it.each([NaN,Infinity,-1])('invalid clock %s permanently withholds write without losing raw text',async(value)=>{
  const h=setup();h.fill();h.setClock(value);await h.editor.queue();h.setClock(2);await h.editor.queue();expect(h.port.enqueue).not.toHaveBeenCalled();expect(h.editor.view().fields.call).toBe('K1ABC')
 })
 it('malformed Applied receipt cannot retire draft',async()=>{
  const h=setup();h.fill();await h.editor.queue();vi.mocked(h.port.deliver).mockResolvedValue({...h.applied,contact_id:'cccccccc-1234-4234-8234-123456789abc'})
  await h.editor.submit();expect(h.editor.view().status).toBe('receipt_invalid');expect(h.editor.finish()).toBe(false)
 })
})
it('empty inspection retires previous queue presence and never keeps Submit enabled',async()=>{
 const h=setup();h.fill();await h.editor.queue();expect(h.editor.view().canSubmit).toBe(true)
 h.setRows([]);await h.editor.inspectQueue();expect(h.editor.view().canSubmit).toBe(false);expect(h.editor.view().canReconcile).toBe(false)
 expect(h.editor.view().status).toBe('frozen');expect(h.editor.view().canRetryQueue).toBe(true);await h.editor.submit();expect(h.port.deliver).not.toHaveBeenCalled()
})
it('inspection failure withholds prior Pending presence, without discarding immutable intent',async()=>{
 const h=setup();h.fill();await h.editor.queue();vi.mocked(h.port.outbox).mockRejectedValue(Error('disk'))
 await h.editor.inspectQueue();expect(h.editor.view().canSubmit).toBe(false);expect(h.editor.view().canRetryQueue).toBe(false);expect(h.values.has(MANUAL_CREATE_KEY)).toBe(true)
})
it('unfrozen failed-first-write discard verifies empty storage; changed stored input is preserved',()=>{
 const h=setup();h.setFail(true);h.editor.edit('call','K1ABC');expect(h.editor.closePolicy()).toBe('blocked');h.editor.discard()
 expect(h.editor.closePolicy()).toBe('close');expect(h.values.has(MANUAL_CREATE_KEY)).toBe(false);expect(h.port.enqueue).not.toHaveBeenCalled()
 const other=setup();other.setFail(true);other.editor.edit('call','K1ABC');other.values.set(MANUAL_CREATE_KEY,'{"version":99}')
 other.editor.discard();expect(other.values.get(MANUAL_CREATE_KEY)).toBe('{"version":99}');expect(other.editor.view().status).toBe('occupied')
})
it('initial storage read failure reports storage recovery, with no connection or dispatch claim',()=>{
 const h=setup();const editor=createManualCreate({store:{...h.store,getItem:()=>{throw Error('denied')}},port:h.port,source:h.source,now:()=>1})
 expect(editor.view().status).toBe('storage_unavailable');expect(editor.view().editable).toBe(false);expect(editor.view().canQueue).toBe(false)
 expect(h.port.enqueue).not.toHaveBeenCalled();expect(h.port.deliver).not.toHaveBeenCalled()
})
it('failed local freeze can discard the exact older unfrozen draft only before any enqueue attempt',async()=>{
 const h=setup();h.fill();const old=h.values.get(MANUAL_CREATE_KEY);h.setFail(true);await h.editor.queue()
 expect(h.port.enqueue).not.toHaveBeenCalled();expect(h.values.get(MANUAL_CREATE_KEY)).toBe(old);expect(h.editor.view().canDiscard).toBe(true)
 h.editor.discard();expect(h.values.has(MANUAL_CREATE_KEY)).toBe(false);expect(h.editor.closePolicy()).toBe('close')
})
it('failed local freeze cannot discard a changed or frozen record appearing in storage',async()=>{
 const h=setup();h.fill();const saved=JSON.parse(h.values.get(MANUAL_CREATE_KEY)!) as ManualCreateRecord
 h.setFail(true);await h.editor.queue();const foreign=JSON.stringify({...saved,frozen:manualCreateQso(saved.contact_id,saved.fields)})
 h.values.set(MANUAL_CREATE_KEY,foreign);h.editor.discard();expect(h.values.get(MANUAL_CREATE_KEY)).toBe(foreign);expect(h.editor.view().status).toBe('occupied')
 expect(h.port.enqueue).not.toHaveBeenCalled()
})
it('restored frozen and uncertain attempted operations never gain local discard permission',async()=>{
 const h=setup();h.fill();vi.mocked(h.port.enqueue).mockRejectedValueOnce(Error('lost'))
 await h.editor.queue();expect(h.editor.view().status).toBe('queue_unknown');expect(h.editor.view().canDiscard).toBe(false)
 const raw=h.values.get(MANUAL_CREATE_KEY),restored=h.make();expect(restored.view().canDiscard).toBe(false);restored.discard()
 expect(h.values.get(MANUAL_CREATE_KEY)).toBe(raw);h.setRows([]);await h.editor.inspectQueue();h.editor.discard()
 expect(h.values.get(MANUAL_CREATE_KEY)).toBe(raw);expect(h.editor.view().canDiscard).toBe(false)
})
it('observing an exact native operation retires local-never-attempted discard even after failed freeze',async()=>{
 const h=setup();h.fill();h.setFail(true);await h.editor.queue();expect(h.editor.view().canDiscard).toBe(true)
 const fields=h.editor.view().fields,qso=manualCreateQso(contact,fields)!
 h.setRows([{queued:{version:1,request_id:req,workspace_id:'workspace',mutation:{kind:'logbook.create',qso}},state:'NeedsReceipt'}])
 await h.editor.inspectQueue();expect(h.editor.view().canDiscard).toBe(false);const raw=h.values.get(MANUAL_CREATE_KEY)
 h.editor.discard();expect(h.values.get(MANUAL_CREATE_KEY)).toBe(raw);expect(h.port.enqueue).not.toHaveBeenCalled()
})
