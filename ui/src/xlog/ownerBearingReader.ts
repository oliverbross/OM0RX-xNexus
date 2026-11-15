// SPDX-License-Identifier: GPL-3.0-only
import type { StationIdentityBinding, View } from './api'
import { bearingReads, type BearingEnvelope, type BearingReadView } from './bearingApi'
import type { SnapshotLease } from './ownerOperatingModel'
import type { OwnerConnectionScope } from './useOwnerConnection'

export interface BearingBinding {
  view: View
  session: number
  lease: SnapshotLease | null
  /** Checks actual synchronously accepted identity, not a stale React render. */
  current: (scope: OwnerConnectionScope, binding: StationIdentityBinding, grid: string) => boolean
}
export type BearingResult =
  | { state: 'current'; payload: BearingEnvelope; deadline: number; contextKey: string; callsign: string }
  | { state: 'loading'; callsign: string }
  | { state: 'unavailable'; callsign: string }
const SAFE = 9_007_199_254_740_990
const object = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === 'object' && !Array.isArray(v)
const keys = (v: Record<string, unknown>, names: string[]) => Object.keys(v).sort().join('|') === [...names].sort().join('|')
const integer = (v: unknown, min = 0): v is number => typeof v === 'number' && Number.isSafeInteger(v) && v >= min && v <= SAFE
const finite = (v: unknown, min: number, max: number): v is number => typeof v === 'number' && Number.isFinite(v) && v >= min && v <= max
const uuid = (v: unknown): v is string => typeof v === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(v)
const nonnil = (v: unknown): v is string => uuid(v) && v !== '00000000-0000-0000-0000-000000000000'
const text = (v: unknown, max: number, min = 0): v is string => typeof v === 'string' && new TextEncoder().encode(v).length <= max && v.length >= min
const optionalText = (v: unknown, max: number) => v === null || text(v, max)
export const literalBearingCall = (v: unknown): v is string => typeof v === 'string' && /^[A-Za-z0-9/]{3,32}$/.test(v)
  && /[A-Za-z]/.test(v) && /[0-9]/.test(v) && !v.startsWith('/') && !v.endsWith('/') && !v.includes('//')
const grid = (v: unknown): v is string => typeof v === 'string' && /^[A-R]{2}[0-9]{2}(?:[A-X]{2}(?:[0-9]{2})?)?$/.test(v)
export function bearingIdentityKey(v: unknown): string | null {
  if (!object(v) || !keys(v, ['workspace_generation','worker_generation','settings_revision','identity_epoch','identity_revision','operating_module'])
    || !nonnil(v.workspace_generation) || !nonnil(v.identity_epoch) || !integer(v.worker_generation, 1)
    || !integer(v.settings_revision) || !integer(v.identity_revision) || v.operating_module !== 'normal_logging') return null
  return JSON.stringify([v.workspace_generation,v.worker_generation,v.settings_revision,v.identity_epoch,v.identity_revision,v.operating_module])
}
const reasons = new Set(['origin_unavailable','record_too_large','invalid_record','logbook_unavailable','reference_unavailable',
  'country_ambiguous','country_known_no_entity','country_invalid','country_missing_date','country_unknown',
  'target_coordinates_unavailable','reference_metadata_invalid','coincident','antipodal','invalid_geometry'])
function body(v: unknown, call: string): boolean {
  if (!object(v) || !keys(v, ['schema_version','requested_callsign','normalized_callsign','status','reason','origin','target','geometry',
    'logbook_revision','reference_revision','motion_authority','transmit_authority']) || v.schema_version !== 1
    || v.requested_callsign !== call || v.normalized_callsign !== call.toUpperCase() || v.motion_authority !== false
    || v.transmit_authority !== false || !integer(v.logbook_revision)) return false
  const o = v.origin, t = v.target, g = v.geometry
  if (o !== null && (!object(o) || !keys(o, ['source','grid','latitude','longitude']) || o.source !== 'saved_station_settings'
    || !grid(o.grid) || !finite(o.latitude,-90,90) || !finite(o.longitude,-180,180))) return false
  if (t !== null) {
    if (!object(t) || !keys(t, ['source','grid','latitude','longitude','precision','contact_id','contact_revision','contact_started_at_unix_ms',
      'reference_revision','matched_reference_revision','matched_rule','country_status']) || !finite(t.latitude,-90,90) || !finite(t.longitude,-180,180)) return false
    if (t.source === 'exact_logbook_grid') {
      if (t.precision !== 'historical_grid_cell_center' || !grid(t.grid) || !uuid(t.contact_id) || !integer(t.contact_revision)
        || t.contact_revision > v.logbook_revision || !integer(t.contact_started_at_unix_ms,-SAFE)
        || t.reference_revision !== null || t.matched_reference_revision !== null || t.matched_rule !== null || t.country_status !== null) return false
    } else if (t.source === 'cty_entity_approximation') {
      if (t.precision !== 'entity_approximation' || t.grid !== null || t.contact_id !== null || t.contact_revision !== null
        || t.contact_started_at_unix_ms !== null || !text(t.reference_revision,2048,1) || !optionalText(t.matched_reference_revision,2048)
        || !optionalText(t.matched_rule,128) || t.country_status !== 'resolved') return false
    } else return false
  }
  if (v.reference_revision !== (object(t) ? t.reference_revision : null)) return false
  if (g !== null && (!object(g) || !keys(g,['short_path_deg','long_path_deg','distance_km'])
    || !finite(g.short_path_deg,0,360) || g.short_path_deg === 360 || !finite(g.long_path_deg,0,360) || g.long_path_deg === 360
    || !finite(g.distance_km,0,Number.MAX_VALUE))) return false
  if (v.status === 'available') return v.reason === null && o !== null && t !== null && g !== null
  if (v.status !== 'unavailable' || typeof v.reason !== 'string' || !reasons.has(v.reason) || g !== null) return false
  if (v.reason === 'origin_unavailable') return o === null && t === null
  return o !== null && (['coincident','antipodal','invalid_geometry'].includes(v.reason) ? t !== null : t === null)
}
function admissionValue(input: BearingBinding, at: number) {
  const { view, lease, session } = input, h = view.hello, s = lease?.snapshot, i = s?.station_identity
  if (!integer(session) || !view.paired || !view.connected || !view.connection_token || !h || h.application !== 'OM0RX-xLog'
    || h.protocol_version !== 1 || !h.capabilities.includes('bearing.read') || !nonnil(h.server_instance) || !nonnil(h.workspace_id)
    || !lease || !s || view.snapshot !== s || !Number.isFinite(lease.deadline) || lease.deadline <= at
    || s.server_instance !== h.server_instance || s.workspace_id !== h.workspace_id || !i || i.schema_version !== 1
    || i.source !== 'saved_station_settings' || !['current','incomplete'].includes(i.state) || i.transmit_authority !== false
    || !grid(i.mygrid) || !i.binding || !bearingIdentityKey(i.binding) || i.binding.workspace_generation !== h.workspace_id) return null
  const scope: OwnerConnectionScope = { session, connectionToken:view.connection_token,serverInstance:h.server_instance,workspaceId:h.workspace_id }
  if (!input.current(scope,i.binding,i.mygrid)) return null
  return { scope, binding: structuredClone(i.binding), grid:i.mygrid, deadline:lease.deadline,
    key:JSON.stringify([scope,bearingIdentityKey(i.binding),i.mygrid]) }
}
function admission(input:BearingBinding,at:number) {try{return admissionValue(input,at)}catch{return null}}
export const bearingBindingKey = (input: BearingBinding, at: number) => Number.isFinite(at) && at >= 0 ? admission(input,at)?.key ?? null : null
function valid(v: unknown, a: NonNullable<ReturnType<typeof admission>>, call: string): v is BearingReadView {
  if (!object(v) || !keys(v,['connection_token','payload','remaining_valid_ms']) || v.connection_token !== a.scope.connectionToken
    || !integer(v.remaining_valid_ms,1) || v.remaining_valid_ms > 5000 || !object(v.payload)) return false
  const p=v.payload
  return keys(p,['protocol_version','server_instance','workspace_id','generation','worker_generation','station_identity_binding','generated_at_unix_ms','max_age_ms','bearing'])
    && p.protocol_version === 1 && p.server_instance === a.scope.serverInstance && p.workspace_id === a.scope.workspaceId
    && p.generation === a.scope.workspaceId && p.worker_generation === a.binding.worker_generation
    && bearingIdentityKey(p.station_identity_binding) === bearingIdentityKey(a.binding) && integer(p.generated_at_unix_ms,1)
    && p.max_age_ms === 5000 && body(p.bearing,call) && object(p.bearing) && object(p.bearing.origin) && p.bearing.origin.grid === a.grid
    && new TextEncoder().encode(JSON.stringify(p)).length <= 20*1024
    && new TextEncoder().encode(JSON.stringify(p.bearing)).length <= 16*1024
}
function freeze<T>(v: T): T { if (v !== null && typeof v === 'object') { Object.values(v).forEach(freeze); Object.freeze(v) } return v }

/** Explicit IPC only. A render/selection intent must synchronize; actions and
 * replies independently recheck the synchronous owner identity predicate. */
export function createOwnerBearingReader(options: { transport?: typeof bearingReads; now?: () => number } = {}) {
  const transport=options.transport??bearingReads, clock=options.now??(()=>performance.now())
  let epoch:object={}, pending=false, last=0, clockBad=false, context:string|null=null, selected=''
  let evidenceContext:string|null=null, revision:number|null=null, stamp:{generated:number;deadline:number}|null=null
  let held:BearingResult={state:'unavailable',callsign:''}
  const retire=()=>{ epoch={}; held={state:'unavailable',callsign:selected} }
  function now() { let n:number; try { n=clock() } catch { n=NaN }
    if (!Number.isFinite(n)||n<0||n<last) {clockBad=true;retire();return null} last=n;return n }
  function synchronize(input:BearingBinding, call:string|null) {
    const at=now(), a=at===null||clockBad?null:admission(input,at), next=literalBearingCall(call)?call:''
    if (a && a.key!==evidenceContext) {evidenceContext=a.key;revision=null;stamp=null}
    if ((a?.key??null)!==context || next!==selected) {context=a?.key??null;selected=next;retire()}
    if (held.state==='current' && (at===null||at>=held.deadline)) retire()
    return a
  }
  return {
    invalidate:retire,
    inspect(input:BearingBinding,call:string|null) {synchronize(input,call);return held},
    isCurrent(result:BearingResult,input:BearingBinding,call:string|null) {
      synchronize(input,call);return held===result&&result.state==='current'&&!clockBad
    },
    async read(input:BearingBinding,call:string):Promise<BearingResult> {
      const a=synchronize(input,call);retire()
      const started=now(), version=epoch, exact=selected
      if (!a||!exact||started===null||clockBad||pending) return held
      pending=true;held={state:'loading',callsign:exact}
      const active=()=>epoch===version && context===a.key && selected===exact && input.current(a.scope,a.binding,a.grid)
      try {
        const response=await transport.read(a.scope.connectionToken,exact), finished=now()
        if (!active()) { if(epoch===version) retire();return {state:'unavailable',callsign:exact} }
        if (finished===null||!valid(response,a,exact)) {retire();return held}
        const p=response.payload
        if (revision!==null&&p.bearing.logbook_revision<revision || stamp&&p.generated_at_unix_ms<stamp.generated) {retire();return held}
        let deadline=Math.min(a.deadline,started+response.remaining_valid_ms)
        if(stamp?.generated===p.generated_at_unix_ms) deadline=Math.min(deadline,stamp.deadline)
        revision=p.bearing.logbook_revision;stamp={generated:p.generated_at_unix_ms,deadline}
        if(finished>=deadline) {retire();return held}
        held=freeze({state:'current',payload:structuredClone(p),deadline,contextKey:a.key,callsign:exact} as const)
        return held
      } catch { if(epoch===version) retire();return {state:'unavailable',callsign:exact} }
      finally {pending=false}
    },
  }
}
