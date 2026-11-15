// Pure shared display-to-mutation projection. No storage, clock, network or normalization.
import type { QsoDraft } from './api'

const uuid = (value: unknown) => typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
const string = (value: unknown, max: number, multiline = false) => typeof value === 'string' && value.length <= max && !(multiline ? /[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/ : /[\x00-\x1f\x7f]/).test(value)
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value)
const integer = (value: unknown, max: number) => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 && value <= max
/** Same bounds used by the existing v1 draft. No normalization is performed. */
export function validQsoDraft(qso: unknown): qso is QsoDraft {
  return object(qso) && Object.keys(qso).sort().join(',') === 'band,callsign,comment,contact_id,frequency_hz,grid,mode,rst_received,rst_sent,started_at_unix_ms'
    && uuid(qso.contact_id) && string(qso.callsign,32) && string(qso.band,16) && string(qso.mode,16) && integer(qso.started_at_unix_ms,4_102_444_800_000)
    && (qso.frequency_hz === null || integer(qso.frequency_hz,1_000_000_000_000) && qso.frequency_hz !== 0)
    && string(qso.rst_sent,16) && string(qso.rst_received,16) && string(qso.grid,12) && string(qso.comment,4096,true)
}

export interface OwnerEditFields {
  date: string; time: string; frequency: string
  rstSent: string; rstReceived: string; grid: string; comment: string
}
const bytes = (s: string) => new TextEncoder().encode(s).length
export function wireQso(q: QsoDraft): boolean {
  if (!validQsoDraft(q) || !/^[A-Za-z0-9/\-]{1,32}$/.test(q.callsign)) return false
  for (const [v,max,optional] of [[q.band,16,false],[q.mode,16,false],[q.rst_sent,16,true],[q.rst_received,16,true],[q.grid,12,true]] as const) {
    if (bytes(v)>max || /\p{Cc}/u.test(v) || (v!==''||!optional)&&v.trim()==='') return false
  }
  return bytes(q.comment)<=4096
    && ![...q.comment].some(c=>/\p{Cc}/u.test(c)&&!['\r','\n','\t'].includes(c))
}
function hzText(hz: number | null): string {
  if (hz===null) return ''
  const whole=Math.floor(hz/1_000_000),fraction=String(hz%1_000_000).padStart(6,'0').replace(/0+$/,'')
  return fraction?`${whole}.${fraction}`:String(whole)
}
export function ownerEditFields(q: QsoDraft): OwnerEditFields {
  const [date,time]=new Date(q.started_at_unix_ms).toISOString().split('T')
  return {date,time:time.slice(0,-1),frequency:hzText(q.frequency_hz),rstSent:q.rst_sent,rstReceived:q.rst_received,grid:q.grid,comment:q.comment}
}
export function ownerEditQso(base: QsoDraft, f: OwnerEditFields): QsoDraft | null {
  if (!wireQso(base)) return null
  const initial=ownerEditFields(base)
  let start=base.started_at_unix_ms,frequency=base.frequency_hz
  if (f.date!==initial.date || f.time!==initial.time) {
    const d=/^(\d{4})-(\d{2})-(\d{2})$/.exec(f.date),t=/^(\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?$/.exec(f.time)
    if (!d||!t) return null
    const [year,month,day]=d.slice(1).map(Number),hour=Number(t[1]),minute=Number(t[2]),second=Number(t[3]??0),ms=Number((t[4]??'').padEnd(3,'0'))
    start=Date.UTC(year,month-1,day,hour,minute,second,ms)
    const check=new Date(start)
    if (check.getUTCFullYear()!==year||check.getUTCMonth()!==month-1||check.getUTCDate()!==day||check.getUTCHours()!==hour||check.getUTCMinutes()!==minute||check.getUTCSeconds()!==second) return null
  }
  if (f.frequency!==initial.frequency) {
    if (f.frequency==='') frequency=null
    else {
      const parts=/^(\d{1,7})(?:\.(\d{1,6}))?$/.exec(f.frequency)
      if (!parts) return null
      frequency=Number(parts[1])*1_000_000+Number((parts[2]??'').padEnd(6,'0'))
    }
  }
  const q={...base,started_at_unix_ms:start,frequency_hz:frequency,rst_sent:f.rstSent,rst_received:f.rstReceived,grid:f.grid,comment:f.comment}
  return wireQso(q)?q:null
}
