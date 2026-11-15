// SPDX-License-Identifier: GPL-3.0-only
// Only QSO data lives here, never pairing credentials. Native startup holds the
// outbox's single-writer lock before opening this private WebView profile.
import type { QsoDraft } from './api'
import { validQsoDraft, ownerEditQso, type OwnerEditFields } from './ownerEditFields'
export { validQsoDraft } from './ownerEditFields'
export type { OwnerEditFields } from './ownerEditFields'

export const DRAFT_KEY = 'om0rx-xnexus.qso-draft.v1'
export interface SavedDraft {
  version: 1
  request_id: string
  workspace_id: string | null
  edit_revision: number | null
  started: boolean
  qso: QsoDraft
}
export function newSavedDraft(): SavedDraft {
  return { version: 1, request_id: crypto.randomUUID(), workspace_id: null, edit_revision: null, started: false,
    qso: { contact_id: crypto.randomUUID(), callsign: '', band: '20m', mode: 'SSB', started_at_unix_ms: Date.now(), frequency_hz: null, rst_sent: '', rst_received: '', grid: '', comment: '' } }
}
const uuid = (value: unknown) => typeof value === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(value)
const string = (value: unknown, max: number, multiline = false) => typeof value === 'string' && value.length <= max && !(multiline ? /[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]/ : /[\x00-\x1f\x7f]/).test(value)
const object = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value)
const integer = (value: unknown, max: number) => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 && value <= max
export function validSavedDraft(value: unknown): value is SavedDraft {
  if (!object(value) || Object.keys(value).sort().join(',') !== 'edit_revision,qso,request_id,started,version,workspace_id') return false
  if (value.version !== 1 || !uuid(value.request_id) || typeof value.started !== 'boolean' || !(value.workspace_id === null || string(value.workspace_id,128) && value.workspace_id !== '') || !(value.edit_revision === null || integer(value.edit_revision,Number.MAX_SAFE_INTEGER - 1) && value.edit_revision !== 0 && value.workspace_id !== null)) return false
  return validQsoDraft(value.qso)
}
/** The same private draft owner/key. v1 readers deliberately preserve/lock v2. */
export interface OwnerEditorDraft {
  version: 2
  kind: 'owner_contact_edit'
  request_id: string
  workspace_id: string
  base_revision: number
  base: QsoDraft
  fields: OwnerEditFields
  /** Set before the first enqueue attempt; thereafter neither ID nor payload changes. */
  frozen: QsoDraft | null
}
export type OwnerDraftLoad =
  | { state: 'empty'; raw: null }
  | { state: 'current'; raw: string; value: OwnerEditorDraft }
  | { state: 'occupied' | 'invalid' | 'unavailable'; raw: string | null }

export function validOwnerEditorDraft(value: unknown): value is OwnerEditorDraft {
  if (!object(value) || Object.keys(value).sort().join(',') !== 'base,base_revision,fields,frozen,kind,request_id,version,workspace_id') return false
  if (value.version !== 2 || value.kind !== 'owner_contact_edit' || !uuid(value.request_id)
    || !string(value.workspace_id,128) || value.workspace_id === ''
    || !integer(value.base_revision,Number.MAX_SAFE_INTEGER - 1) || value.base_revision === 0
    || !validQsoDraft(value.base)) return false
  const fields = value.fields
  if (!object(fields) || Object.keys(fields).sort().join(',') !== 'comment,date,frequency,grid,rstReceived,rstSent,time'
    || !string(fields.date,10) || !string(fields.time,16) || !string(fields.frequency,32)
    || !string(fields.rstSent,16) || !string(fields.rstReceived,16) || !string(fields.grid,12)
    || !string(fields.comment,4096,true)) return false
  if (value.frozen === null) return true
  if (!validQsoDraft(value.frozen)) return false
  const displayed = ownerEditQso(value.base, fields as unknown as OwnerEditFields)
  return displayed !== null && Object.keys(displayed).every(key =>
    displayed[key as keyof QsoDraft] === (value.frozen as QsoDraft)[key as keyof QsoDraft])
}

export function loadOwnerEditorDraft(): OwnerDraftLoad {
  let raw: string | null
  try { raw = window.localStorage.getItem(DRAFT_KEY) } catch { return { state: 'unavailable', raw: null } }
  if (raw === null) return { state: 'empty', raw: null }
  try {
    if (raw.length > 32_768) return { state: 'invalid', raw }
    const value: unknown = JSON.parse(raw)
    if (validSavedDraft(value)) return { state: 'occupied', raw }
    if (validOwnerEditorDraft(value)) return { state: 'current', raw, value }
    return { state: 'invalid', raw }
  } catch { return { state: 'invalid', raw } }
}

/** Compare the exact prior bytes synchronously; another draft is never replaced. */
export function saveOwnerEditorDraft(value: OwnerEditorDraft, expectedRaw: string | null): string {
  if (!validOwnerEditorDraft(value)) throw Error('invalid_draft')
  const raw = JSON.stringify(value)
  if (raw.length > 32_768) throw Error('invalid_draft')
  let before: string | null
  try { before = window.localStorage.getItem(DRAFT_KEY) } catch { throw Error('storage_unavailable') }
  if (before !== expectedRaw) throw Error('draft_occupied')
  if (before !== null) {
    let previous: unknown
    try { previous = JSON.parse(before) } catch { throw Error('draft_occupied') }
    if (!validOwnerEditorDraft(previous)
      || previous.request_id !== value.request_id || previous.workspace_id !== value.workspace_id
      || previous.base_revision !== value.base_revision || JSON.stringify(previous.base) !== JSON.stringify(value.base)
      || previous.frozen !== null && (JSON.stringify(previous.frozen) !== JSON.stringify(value.frozen)
        || JSON.stringify(previous.fields) !== JSON.stringify(value.fields))) throw Error('draft_occupied')
  }
  try { window.localStorage.setItem(DRAFT_KEY,raw) } catch { throw Error('storage_unavailable') }
  return raw
}

/** Only an explicit discard/reload of this exact retained record may remove it. */
export function clearOwnerEditorDraft(expectedRaw: string): void {
  const current = loadOwnerEditorDraft()
  if (current.state !== 'current' || current.raw !== expectedRaw) throw Error('draft_occupied')
  try { window.localStorage.removeItem(DRAFT_KEY) } catch { throw Error('storage_unavailable') }
}
export function loadDraft(): { value: SavedDraft; error: string; locked: boolean; raw: string | null | undefined } {
  const fresh = newSavedDraft()
  let raw: string | null
  try { raw = window.localStorage.getItem(DRAFT_KEY) } catch { return { value: fresh, error: 'Cannot read draft storage. Keep this window open; submission waits until the draft can be saved.', locked: false, raw: undefined } }
  if (raw === null) return { value: fresh, error: '', locked: false, raw }
  try {
    if (raw.length > 32_768) throw Error('Oversized draft')
    const value: unknown = JSON.parse(raw)
    if (!validSavedDraft(value)) throw Error('Invalid draft')
    return { value, error: '', locked: false, raw }
  } catch { return { value: fresh, error: 'The saved draft needs recovery and has been preserved. Clear draft explicitly to start a new contact.', locked: true, raw } }
}
/** Existing legacy Clear button only. Compare its last observed/saved bytes;
 * never migrate or clear another editor/version, and never remove before write. */
export function explicitlyClearLegacyDraft(value:SavedDraft,expectedRaw:string|null|undefined):string{
  if(!validSavedDraft(value)||expectedRaw===undefined)throw Error('The existing draft cannot be verified and has been preserved.')
  let current:string|null
  try{current=window.localStorage.getItem(DRAFT_KEY)}catch{throw Error('Cannot read draft storage. The existing draft is preserved.')}
  if(current!==expectedRaw)throw Error('The saved draft changed and has been preserved. Reopen it before recovery.')
  if(current!==null){
    if(current.length>32_768)throw Error('The oversized draft requires recovery and has been preserved.')
    let previous:unknown
    try{previous=JSON.parse(current)}catch{
      // Corrupt data that still names a version/editor is not a legacy claim.
      if(/"(?:version|kind|frozen|base_revision|base)"\s*:/.test(current))throw Error('Another draft format requires recovery and has been preserved.')
    }
    if(object(previous)&&(('version' in previous&&previous.version!==1)
      ||'kind' in previous||'frozen' in previous||'base_revision' in previous||'base' in previous)){
      throw Error('Another draft format requires recovery and has been preserved.')
    }
  }
  const raw=JSON.stringify(value)
  try{window.localStorage.setItem(DRAFT_KEY,raw)}catch{throw Error('Cannot save the cleared draft. The existing draft is preserved.')}
  return raw
}
export function saveDraft(value: SavedDraft): void {
  if (!validSavedDraft(value)) throw Error('Draft values exceed the supported bounds. The previous saved draft is preserved.')
  // A legacy component may have loaded before an owner editor claimed this
  // same key. Its stale save must not erase a v2 operation or recovery record.
  let current: string | null
  try { current = window.localStorage.getItem(DRAFT_KEY) } catch { throw Error('Cannot read draft storage. The previous draft is preserved.') }
  if (current !== null) {
    let previous: unknown
    try { previous = current.length <= 32_768 ? JSON.parse(current) : null } catch { previous = null }
    if (!validSavedDraft(previous)) throw Error('The saved draft needs recovery and has been preserved. Close this editor before recovering it.')
  }
  try { window.localStorage.setItem(DRAFT_KEY,JSON.stringify(value)) } catch { throw Error('Cannot save the draft. Keep this window open; nothing will be submitted until storage is available.') }
}
