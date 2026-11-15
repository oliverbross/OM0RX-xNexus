// A separate bounded manual-create draft, never a migration of the correction key.
import type { QsoDraft } from './api'
import { uuid, wellFormed, validManualFields, manualCreateQso, sameCreateQso, type ManualCreateFields } from './manualCreateFields'
export const MANUAL_CREATE_KEY='om0rx-xnexus.manual-create.v1'
export interface ManualCreateRecord {
  version:1; kind:'owner_manual_create'; request_id:string; contact_id:string; workspace_id:string
  fields:ManualCreateFields; frozen:QsoDraft|null
}
export interface CreateStorage { getItem(key:string):string|null; setItem(key:string,value:string):void; removeItem(key:string):void }
export type CreateLoad={state:'empty';raw:null}|{state:'current';raw:string;value:ManualCreateRecord}|{state:'invalid'|'unavailable';raw:string|null}
export function validCreateRecord(v:unknown):v is ManualCreateRecord {
  if(!v||typeof v!=='object'||Array.isArray(v))return false
  const r=v as ManualCreateRecord
  if(Object.keys(r).sort().join(',')!=='contact_id,fields,frozen,kind,request_id,version,workspace_id'
    ||r.version!==1||r.kind!=='owner_manual_create'||!uuid(r.request_id)||!uuid(r.contact_id)
    ||typeof r.workspace_id!=='string'||!wellFormed(r.workspace_id)||!r.workspace_id.trim()||new TextEncoder().encode(r.workspace_id).length>128||/\p{Cc}/u.test(r.workspace_id)
    ||!validManualFields(r.fields))return false
  const q=manualCreateQso(r.contact_id,r.fields)
  return r.frozen===null||!!q&&!!r.frozen&&typeof r.frozen==='object'&&sameCreateQso(q,r.frozen)
}
export function loadCreateDraft(store:CreateStorage):CreateLoad {
  let raw:string|null
  try{raw=store.getItem(MANUAL_CREATE_KEY)}catch{return{state:'unavailable',raw:null}}
  if(raw===null)return{state:'empty',raw:null}
  try{const value:unknown=raw.length<=32768?JSON.parse(raw):null
    return validCreateRecord(value)?{state:'current',raw,value}:{state:'invalid',raw}
  }catch{return{state:'invalid',raw}}
}
/** Synchronous compare-before-set, NOT a cross-process atomic transaction. The
 * private profile single-writer owner is the outer fence; mismatches fail closed. */
export function saveCreateDraft(store:CreateStorage,next:ManualCreateRecord,expected:string|null):string {
  if(!validCreateRecord(next))throw Error('invalid_input')
  const raw=JSON.stringify(next)
  if(raw.length>32768)throw Error('invalid_input')
  let before:string|null
  try{before=store.getItem(MANUAL_CREATE_KEY)}catch{throw Error('storage_unavailable')}
  if(before!==expected)throw Error('occupied')
  if(before!==null){
    let prior:unknown;try{prior=JSON.parse(before)}catch{throw Error('occupied')}
    if(!validCreateRecord(prior)||prior.request_id!==next.request_id||prior.contact_id!==next.contact_id||prior.workspace_id!==next.workspace_id
      ||prior.frozen!==null&&(!next.frozen||!sameCreateQso(prior.frozen,next.frozen)||JSON.stringify(prior.fields)!==JSON.stringify(next.fields)))throw Error('occupied')
  }
  try{store.setItem(MANUAL_CREATE_KEY,raw)}catch{throw Error('storage_unavailable')}
  return raw
}
export function discardCreateDraft(store:CreateStorage,expected:string|null):void {
  // Unknown/changed records cannot be cleared through a stale form.
  let current:string|null;try{current=store.getItem(MANUAL_CREATE_KEY)}catch{throw Error('storage_unavailable')}
  if(current!==expected)throw Error('occupied')
  // Explicit discard of never-persisted, never-enqueued local typing.
  if(current===null)return
  let record:unknown;try{record=JSON.parse(current)}catch{throw Error('occupied')}
  if(!validCreateRecord(record)||record.frozen!==null)throw Error('occupied')
  try{store.removeItem(MANUAL_CREATE_KEY)}catch{throw Error('storage_unavailable')}
}
/** Explicit acknowledgement of a matching Applied receipt retires only this
 * local form. The native outbox and owner's contact/receipt remain untouched. */
export function retireAppliedCreateDraft(store:CreateStorage,expected:string,receipt:import('./api').Receipt):void {
  let current:string|null;try{current=store.getItem(MANUAL_CREATE_KEY)}catch{throw Error('storage_unavailable')}
  if(current===null||current!==expected)throw Error('occupied')
  let record:unknown;try{record=JSON.parse(current)}catch{throw Error('occupied')}
  if(!validCreateRecord(record)||!record.frozen||receipt.status!=='applied'||receipt.request_id!==record.request_id
    ||receipt.workspace_id!==record.workspace_id||receipt.contact_id?.toLowerCase()!==record.contact_id.toLowerCase()
    ||!Number.isSafeInteger(receipt.contact_revision)||receipt.contact_revision!<1)throw Error('receipt_invalid')
  try{store.removeItem(MANUAL_CREATE_KEY)}catch{throw Error('storage_unavailable')}
}
