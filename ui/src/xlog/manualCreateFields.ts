// Pure owner-create projection. Raw typing and frozen wire payload are different states.
import type { QsoDraft } from './api'
export interface ManualCreateFields {
  call:string; band:string; mode:string; whenDate:string; whenTime:string; freq:string
  rstSent:string; rstRcvd:string; grid:string; comment:string
}
export const manualCreateKeys = ['call','band','mode','whenDate','whenTime','freq','rstSent','rstRcvd','grid','comment'] as const
export const uuid = (v:unknown):v is string => typeof v==='string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(v)
const bytes=(s:string)=>new TextEncoder().encode(s).length
export const wellFormed=(s:string)=>![...s].some(c=>c.length===1&&c.charCodeAt(0)>=0xd800&&c.charCodeAt(0)<=0xdfff)
const controls=(s:string)=>/\p{Cc}/u.test(s)
export function validManualFields(value:unknown):value is ManualCreateFields {
  if(!value||typeof value!=='object'||Array.isArray(value))return false
  const f=value as Record<string,unknown>
  const limits={call:32,band:16,mode:16,whenDate:10,whenTime:16,freq:32,rstSent:16,rstRcvd:16,grid:12,comment:4096}
  return Object.keys(f).sort().join(',')===[...manualCreateKeys].sort().join(',') && manualCreateKeys.every(k=>
    typeof f[k]==='string' && wellFormed(f[k] as string) && (f[k] as string).length<=limits[k]
    && (k==='comment'?!/[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/.test(f[k] as string):!controls(f[k] as string)))
}
export function manualCreateQso(contactId:string,f:ManualCreateFields):QsoDraft|null {
  if(!uuid(contactId)||!validManualFields(f)||!/^[A-Z0-9/\-]{3,32}$/.test(f.call)
    ||!/^[a-zA-Z0-9.]{1,16}$/.test(f.band)||!/^[A-Z0-9\-]{1,16}$/.test(f.mode)
    ||bytes(f.rstSent)>16||bytes(f.rstRcvd)>16||controls(f.rstSent)||controls(f.rstRcvd)
    ||f.grid!==''&&!/^[A-Ra-r]{2}[0-9]{2}([A-Xa-x]{2}([0-9]{2})?)?$/.test(f.grid)
    ||bytes(f.comment)>4096||[...f.comment].some(c=>controls(c)&&!['\r','\n','\t'].includes(c)))return null
  const d=/^(\d{4})-(\d{2})-(\d{2})$/.exec(f.whenDate)
  const t=/^(\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?$/.exec(f.whenTime)
  if(!d||!t)return null
  const [year,month,day]=d.slice(1).map(Number),hour=Number(t[1]),minute=Number(t[2]),second=Number(t[3]??0),ms=Number((t[4]??'').padEnd(3,'0'))
  const start=Date.UTC(year,month-1,day,hour,minute,second,ms),date=new Date(start)
  if(!Number.isSafeInteger(start)||start<0||start>4_102_444_800_000||date.getUTCFullYear()!==year
    ||date.getUTCMonth()!==month-1||date.getUTCDate()!==day||date.getUTCHours()!==hour
    ||date.getUTCMinutes()!==minute||date.getUTCSeconds()!==second)return null
  let hz:number|null=null
  if(f.freq!==''){
    const match=/^(\d{1,7})(?:\.(\d{1,6}))?$/.exec(f.freq)
    if(!match)return null
    hz=Number(match[1])*1_000_000+Number((match[2]??'').padEnd(6,'0'))
    if(!Number.isSafeInteger(hz)||hz<1||hz>1_000_000_000_000)return null
  }
  return {contact_id:contactId,callsign:f.call,band:f.band,mode:f.mode,started_at_unix_ms:start,
    frequency_hz:hz,rst_sent:f.rstSent,rst_received:f.rstRcvd,grid:f.grid,comment:f.comment}
}
export function sameCreateQso(a:QsoDraft,b:QsoDraft):boolean {
  const keys=['contact_id','callsign','band','mode','started_at_unix_ms','frequency_hz','rst_sent','rst_received','grid','comment'] as const
  return !!a&&!!b&&typeof a==='object'&&typeof b==='object'&&!Array.isArray(a)&&!Array.isArray(b)&&Object.keys(a).length===keys.length&&Object.keys(b).length===keys.length&&keys.every(k=>a[k]===b[k])
}
export function blankManualFields(utc:number):ManualCreateFields {
  const d=new Date(utc),valid=Number.isFinite(utc)&&utc>=0&&utc<=4_102_444_800_000
  const stamp=valid?d.toISOString():null
  return {call:'',band:'',mode:'',whenDate:stamp?.slice(0,10)??'',whenTime:stamp?.slice(11,23)??'',freq:'',rstSent:'',rstRcvd:'',grid:'',comment:''}
}
