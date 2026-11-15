// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import producer from './fixtures/contact-detail-producer.json'
import { ownerEditFields } from '../features/ownerContactEditor'
import { DRAFT_KEY, clearOwnerEditorDraft, explicitlyClearLegacyDraft, loadDraft, loadOwnerEditorDraft, newSavedDraft,
  saveDraft, saveOwnerEditorDraft, validOwnerEditorDraft, type OwnerEditorDraft } from './draftStore'
import type { QsoDraft } from './api'

function record():OwnerEditorDraft{
  const base=JSON.parse(JSON.stringify(producer.found.editability.base)) as QsoDraft
  return {version:2,kind:'owner_contact_edit',request_id:'11111111-1111-4111-8111-111111111111',
    workspace_id:'workspace-A',base_revision:1,base,fields:ownerEditFields(base),frozen:null}
}
beforeEach(()=>localStorage.clear())
afterEach(()=>vi.restoreAllMocks())
it('v1 save/load behavior is unchanged and v2 loader preserves it as occupied',()=>{
  const old=newSavedDraft();saveDraft(old);const bytes=localStorage.getItem(DRAFT_KEY)
  expect(loadDraft().value).toEqual(old);expect(loadOwnerEditorDraft().state).toBe('occupied')
  expect(()=>saveOwnerEditorDraft(record(),bytes)).toThrow('draft_occupied')
  expect(localStorage.getItem(DRAFT_KEY)).toBe(bytes)
})
it('round-trips v2 in the SAME key and old v1 reader locks it without deleting it',()=>{
  const next=record();next.fields.comment='line one\r\nline two\t';const raw=saveOwnerEditorDraft(next,null)
  expect(localStorage.length).toBe(1);expect(loadOwnerEditorDraft()).toEqual({state:'current',raw,value:next})
  expect(loadDraft().locked).toBe(true);expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
  expect(raw.includes('token')).toBe(false)
})
it('a legacy reader opened before v2 publication cannot overwrite the new frozen intent',()=>{
  const stale=loadDraft().value,next=record();next.frozen={...next.base}
  const raw=saveOwnerEditorDraft(next,null)
  stale.qso.comment='late old typing'
  expect(()=>saveDraft(stale)).toThrow('preserved')
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
it.each(['comment','frequency_hz','started_at_unix_ms','rst_sent','rst_received','grid'] as const)(
  'preserves and rejects a restored frozen %s that differs from the displayed correction',field=>{
    const next=record();next.frozen={...next.base}
    const changed=next.frozen as unknown as Record<string,unknown>
    changed[field]=field==='frequency_hz'?14074000:field==='started_at_unix_ms'?next.base.started_at_unix_ms+1000:'hidden different value'
    const raw=JSON.stringify(next);localStorage.setItem(DRAFT_KEY,raw)
    expect(loadOwnerEditorDraft().state).toBe('invalid')
    expect(()=>saveDraft(newSavedDraft())).toThrow('preserved')
    expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
  })
it('accepts an exactly matching mutation with equivalent displayed numeric spelling without normalizing stored text',()=>{
  const next=record();next.fields.frequency='14.074000';next.frozen={...next.base,frequency_hz:14074000}
  const raw=saveOwnerEditorDraft(next,null)
  expect(loadOwnerEditorDraft()).toEqual({state:'current',raw,value:next})
})
it.each(['{','x'.repeat(32769),'{}'])('preserves malformed/oversized records without fallback or migration (%#)',raw=>{
  localStorage.setItem(DRAFT_KEY,raw)
  expect(loadOwnerEditorDraft().state).toBe('invalid')
  expect(()=>saveOwnerEditorDraft(record(),raw)).toThrow('draft_occupied')
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
it('rejects extra secret/unknown fields and unavailable storage',()=>{
  expect(validOwnerEditorDraft({...record(),connection_token:'never store'})).toBe(false)
  vi.spyOn(Storage.prototype,'getItem').mockImplementation(()=>{throw Error('unavailable')})
  expect(loadOwnerEditorDraft()).toEqual({state:'unavailable',raw:null})
  expect(()=>saveOwnerEditorDraft(record(),null)).toThrow('storage_unavailable')
})
it('compare-before-write refuses another draft even if this caller retained old bytes',()=>{
  const first=record(),raw=saveOwnerEditorDraft(first,null),replacement={...first,request_id:'22222222-2222-4222-8222-222222222222'}
  localStorage.setItem(DRAFT_KEY,JSON.stringify(replacement))
  expect(()=>saveOwnerEditorDraft({...first,fields:{...first.fields,comment:'changed'}},raw)).toThrow('draft_occupied')
  expect(()=>clearOwnerEditorDraft(raw)).toThrow('draft_occupied')
  expect(loadOwnerEditorDraft()).toMatchObject({value:replacement})
})
it('even exact current bytes cannot authorize changing immutable request/base/workspace',()=>{
  const first=record(),raw=saveOwnerEditorDraft(first,null)
  for(const next of [{...first,request_id:'22222222-2222-4222-8222-222222222222'},
    {...first,workspace_id:'different'}, {...first,base_revision:2}, {...first,base:{...first.base,comment:'changed base'}}]){
    expect(()=>saveOwnerEditorDraft(next,raw)).toThrow('draft_occupied')
  }
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
it('frozen queue intent cannot be thawed, rewritten or visually changed by later saves',()=>{
  const first=record();first.frozen={...first.base,comment:'fixed operation'};first.fields.comment='fixed operation'
  const raw=saveOwnerEditorDraft(first,null)
  expect(saveOwnerEditorDraft(first,raw)).toBe(raw)
  const cases:[OwnerEditorDraft,string][]=[
    [{...first,frozen:null},'draft_occupied'],
    [{...first,frozen:{...first.frozen,comment:'other'}},'invalid_draft'],
    [{...first,fields:{...first.fields,comment:'other'}},'invalid_draft'],
  ]
  for(const [next,error] of cases)expect(()=>saveOwnerEditorDraft(next,raw)).toThrow(error)
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
it.each([null,'{"invalid":"legacy recovery"}','{"version":1,"invalid":true}','{'])('explicit legacy Clear safely replaces its exact observed legacy bytes (%#)',raw=>{
  if(raw!==null)localStorage.setItem(DRAFT_KEY,raw)
  const initial=loadDraft(),next=newSavedDraft()
  expect(explicitlyClearLegacyDraft(next,initial.raw)).toBe(JSON.stringify(next))
  expect(loadDraft().value).toEqual(next)
})
it('explicit legacy Clear preserves ordinary valid v1 behavior',()=>{
  const old=newSavedDraft();old.qso.comment='old draft';saveDraft(old)
  const initial=loadDraft(),next=newSavedDraft();explicitlyClearLegacyDraft(next,initial.raw)
  expect(loadDraft().value).toEqual(next)
})
it.each(['{"version":3,"qso":{}}','{"version":2','{"kind":"owner_contact_edit"}','{"version":null}'])(
  'explicit legacy Clear refuses recognizable nonlegacy or unknown records (%#)',raw=>{
    localStorage.setItem(DRAFT_KEY,raw)
    expect(()=>explicitlyClearLegacyDraft(newSavedDraft(),loadDraft().raw)).toThrow('preserved')
    expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
  })
it('stale explicit Clear cannot overwrite a new frozen v2 or a changed unknown record',()=>{
  const initial=loadDraft(),next=record();next.frozen={...next.base}
  const raw=saveOwnerEditorDraft(next,null)
  expect(()=>explicitlyClearLegacyDraft(newSavedDraft(),initial.raw)).toThrow('changed')
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
  const loaded=loadDraft();localStorage.setItem(DRAFT_KEY,'{"version":99}')
  expect(()=>explicitlyClearLegacyDraft(newSavedDraft(),loaded.raw)).toThrow('changed')
  expect(localStorage.getItem(DRAFT_KEY)).toBe('{"version":99}')
})
it('explicit Clear storage failure never erases the observed bytes',()=>{
  const raw='{"invalid":"retained"}';localStorage.setItem(DRAFT_KEY,raw)
  vi.spyOn(Storage.prototype,'setItem').mockImplementation(()=>{throw Error('quota')})
  expect(()=>explicitlyClearLegacyDraft(newSavedDraft(),raw)).toThrow('preserved')
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
it('failed publication preserves the old saved bytes',()=>{
  const first=record(),raw=saveOwnerEditorDraft(first,null)
  vi.spyOn(Storage.prototype,'setItem').mockImplementation(()=>{throw Error('quota')})
  expect(()=>saveOwnerEditorDraft({...first,fields:{...first.fields,grid:'FN42'}},raw)).toThrow('storage_unavailable')
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
