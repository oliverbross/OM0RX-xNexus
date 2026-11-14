// SPDX-License-Identifier: GPL-3.0-only
// Only QSO data lives here, never pairing credentials. Native startup holds the
// outbox's single-writer lock before opening this private WebView profile.
import type { QsoDraft } from './api'

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
  const qso = value.qso
  return object(qso) && Object.keys(qso).sort().join(',') === 'band,callsign,comment,contact_id,frequency_hz,grid,mode,rst_received,rst_sent,started_at_unix_ms'
    && uuid(qso.contact_id) && string(qso.callsign,32) && string(qso.band,16) && string(qso.mode,16) && integer(qso.started_at_unix_ms,4_102_444_800_000)
    && (qso.frequency_hz === null || integer(qso.frequency_hz,1_000_000_000_000) && qso.frequency_hz !== 0)
    && string(qso.rst_sent,16) && string(qso.rst_received,16) && string(qso.grid,12) && string(qso.comment,4096,true)
}
export function loadDraft(): { value: SavedDraft; error: string; locked: boolean } {
  const fresh = newSavedDraft()
  let raw: string | null
  try { raw = window.localStorage.getItem(DRAFT_KEY) } catch { return { value: fresh, error: 'Cannot read draft storage. Keep this window open; submission waits until the draft can be saved.', locked: false } }
  if (raw === null) return { value: fresh, error: '', locked: false }
  try {
    if (raw.length > 32_768) throw Error('Oversized draft')
    const value: unknown = JSON.parse(raw)
    if (!validSavedDraft(value)) throw Error('Invalid draft')
    return { value, error: '', locked: false }
  } catch { return { value: fresh, error: 'The saved draft needs recovery and has been preserved. Clear draft explicitly to start a new contact.', locked: true } }
}
export function saveDraft(value: SavedDraft): void {
  if (!validSavedDraft(value)) throw Error('Draft values exceed the supported bounds. The previous saved draft is preserved.')
  try { window.localStorage.setItem(DRAFT_KEY,JSON.stringify(value)) } catch { throw Error('Cannot save the draft. Keep this window open; nothing will be submitted until storage is available.') }
}
