import { describe,it,expect } from 'vitest'
import { DRAFT_KEY } from './draftStore'
import { MANUAL_CREATE_KEY,loadCreateDraft,saveCreateDraft,discardCreateDraft,validCreateRecord,retireAppliedCreateDraft,type CreateStorage,type ManualCreateRecord } from './manualCreateStore'
import { blankManualFields,manualCreateQso } from './manualCreateFields'
function setup(){const values=new Map<string,string>();const store:CreateStorage={getItem:k=>values.get(k)??null,setItem:(k,v)=>{values.set(k,v)},removeItem:k=>{values.delete(k)}};return{values,store}}
function record():ManualCreateRecord{return{version:1,kind:'owner_manual_create',request_id:'aaaaaaaa-1234-4234-8234-123456789abc',contact_id:'bbbbbbbb-1234-4234-8234-123456789abc',workspace_id:'workspace',fields:{...blankManualFields(Date.UTC(2024,0,1)),call:'K1ABC',band:'20m',mode:'FT8'},frozen:null}}
describe('independent manual draft preservation',()=>{
  it('opening is read-only, preserves correction bytes, and roundtrips partial fields',()=>{
    const {store,values}=setup();values.set(DRAFT_KEY,'untouched-v2-correction')
    expect(loadCreateDraft(store).state).toBe('empty');expect(values.size).toBe(1)
    const r=record();r.fields.whenDate='2024-';saveCreateDraft(store,r,null)
    expect(loadCreateDraft(store)).toMatchObject({state:'current',value:r});expect(values.get(DRAFT_KEY)).toBe('untouched-v2-correction')
  })
  it('does not replace a competing writer or an unknown version',()=>{
    const {store,values}=setup(),r=record();const raw=saveCreateDraft(store,r,null)
    for(const foreign of ['{"version":9}',JSON.stringify({...r,request_id:'cccccccc-1234-4234-8234-123456789abc'})]){
      values.set(MANUAL_CREATE_KEY,foreign);expect(()=>saveCreateDraft(store,r,raw)).toThrow('occupied');expect(()=>discardCreateDraft(store,raw)).toThrow('occupied');expect(values.get(MANUAL_CREATE_KEY)).toBe(foreign)
    }
  })
  it('rejects a changed body under an already frozen ID, including displayed/frozen disagreement',()=>{
    const {store}=setup(),r=record();r.frozen=manualCreateQso(r.contact_id,r.fields);const raw=saveCreateDraft(store,r,null)
    const bad={...r,frozen:{...r.frozen!,comment:'hidden'}};expect(validCreateRecord(bad)).toBe(false)
    const changed={...r,fields:{...r.fields,comment:'other'},frozen:{...r.frozen!,comment:'other'}}
    expect(()=>saveCreateDraft(store,changed,raw)).toThrow('occupied');expect(()=>discardCreateDraft(store,raw)).toThrow('occupied')
  })
  it('preserves corrupt/oversized and unavailable storage',()=>{
    const {store,values}=setup();for(const raw of ['{bad','x'.repeat(32769)]){values.set(MANUAL_CREATE_KEY,raw);expect(loadCreateDraft(store)).toEqual({state:'invalid',raw})}
    expect(loadCreateDraft({...store,getItem:()=>{throw Error('denied')}}).state).toBe('unavailable')
  })
  it('a failed write or clear retains prior bytes',()=>{
    const {store,values}=setup(),r=record();const raw=saveCreateDraft(store,r,null)
    expect(()=>saveCreateDraft({...store,setItem:()=>{throw Error('disk')}},{...r,fields:{...r.fields,comment:'new'}},raw)).toThrow('storage_unavailable')
    expect(()=>discardCreateDraft({...store,removeItem:()=>{throw Error('disk')}},raw)).toThrow('storage_unavailable');expect(values.get(MANUAL_CREATE_KEY)).toBe(raw)
  })
  it('only explicit exact Applied acknowledgement retires local form, not correction or outbox',()=>{
    const {store,values}=setup(),r=record();r.frozen=manualCreateQso(r.contact_id,r.fields);const raw=saveCreateDraft(store,r,null);values.set(DRAFT_KEY,'retained')
    const receipt={request_id:r.request_id,workspace_id:r.workspace_id,server_instance:'owner',status:'applied' as const,message:null,contact_id:r.contact_id,contact_revision:1}
    expect(()=>retireAppliedCreateDraft(store,raw,{...receipt,status:'uncertain'})).toThrow('receipt_invalid')
    retireAppliedCreateDraft(store,raw,receipt);expect(values.get(MANUAL_CREATE_KEY)).toBeUndefined();expect(values.get(DRAFT_KEY)).toBe('retained')
  })
})
