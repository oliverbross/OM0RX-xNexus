// SPDX-License-Identifier: GPL-3.0-only
// Passive original-interface reads. No local log, browser HTTP or retry loop.
import { historyReads, type History, type HistoryReadView, type RecentContacts, type View } from './api'

export interface OwnerReadBinding {
  sessionGeneration: number
  connectionToken: string | null
  owner: Pick<View, 'paired' | 'connected' | 'hello'>
}
export interface OwnerHistoryQuery { callsign: string; band?: string; exactMode?: string }
type CanonicalQuery = { callsign: string; band?: string; exactMode?: string; key: string }
export type OwnerReadState<T> =
  | { state: 'current'; payload: T; deadline: number; contextKey: string; queryKey: string }
  | { state: 'loading'; contextKey: string; queryKey: string }
  | { state: 'unavailable'; reason: string; contextKey: string | null; queryKey: string }
type Payload = History | RecentContacts
const object = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v)
const integer = (v: unknown, min = 0, max = Number.MAX_SAFE_INTEGER): v is number =>
  typeof v === 'number' && Number.isSafeInteger(v) && v >= min && v <= max
const token = (v: unknown): v is string => typeof v === 'string' && v.length > 0 && v.length <= 128 && !/[\u0000-\u001f\u007f]/.test(v)

/** Echo the owner's documented exact-mode aliases; never merge FT8 with FT4. */
function query(value: OwnerHistoryQuery): CanonicalQuery | null {
  const rawCall = value.callsign.trim(), rawMode = value.exactMode?.trim()
  if (!/^[A-Za-z0-9/-]{1,32}$/.test(rawCall)
    || (rawMode !== undefined && !/^[A-Za-z0-9][A-Za-z0-9-]{0,19}$/.test(rawMode))) return null
  const callsign = rawCall.toUpperCase()
  const band = value.band?.trim().toLowerCase()
  let exactMode = value.exactMode?.trim().toUpperCase()
  if (exactMode && ['USB', 'LSB', 'PHONE', 'PH'].includes(exactMode)) exactMode = 'SSB'
  if (exactMode === 'CWR') exactMode = 'CW'
  if (exactMode === 'RTTYR') exactMode = 'RTTY'
  if ((exactMode !== undefined && band === undefined)
    || (band !== undefined && !/^\d{1,4}(?:\.\d{1,3})?(?:cm|mm|m)$/.test(band))
    || (exactMode !== undefined && !/^[A-Z0-9][A-Z0-9-]{0,19}$/.test(exactMode))) return null
  return { callsign, band, exactMode, key: JSON.stringify([callsign, band ?? null, exactMode ?? null]) }
}
function bindingKey(binding: OwnerReadBinding): string | null {
  const { owner } = binding
  if (!integer(binding.sessionGeneration) || !token(binding.connectionToken)
    || !owner.paired || !owner.connected || !owner.hello
    || !owner.hello.capabilities.includes('logbook.read')
    || !token(owner.hello.server_instance) || !token(owner.hello.workspace_id)) return null
  return JSON.stringify([binding.sessionGeneration, binding.connectionToken, owner.hello.server_instance, owner.hello.workspace_id])
}

/** The native SDK validates contact details and producer semantics. This second
 * boundary verifies IPC identity, requested scope, bounded counts and lease.
 * Incomplete/legacy history is unavailable here, never an empty full history. */
function valid(value: unknown, binding: OwnerReadBinding, q: CanonicalQuery | null): value is HistoryReadView<Payload> {
  if (!object(value) || value.connection_token !== binding.connectionToken || !integer(value.remaining_valid_ms, 1, 5000)
    || !object(value.payload)) return false
  const p = value.payload, hello = binding.owner.hello!
  if (p.protocol_version !== 1 || p.server_instance !== hello.server_instance || p.workspace_id !== hello.workspace_id
    || p.generation !== hello.workspace_id || !integer(p.logbook_revision) || !integer(p.generated_at_unix_ms, 1)
    || !integer(p.max_age_ms, 1, 5000) || value.remaining_valid_ms > p.max_age_ms || !Array.isArray(p.contacts)) return false
  if (q === null) return p.schema_version === 1 && p.limit === 5 && integer(p.total)
    && p.contacts.length === Math.min(p.total, 5) && p.complete === (p.total <= 5)
    && p.ordering === 'qso_start_desc_contact_id_asc'
  const s = p.history_summary
  return p.callsign === q.callsign && p.contacts_limit === 25 && typeof p.matrix_complete === 'boolean'
    && Array.isArray(p.matrix) && object(s) && s.schema_version === 1 && s.scope === 'normal_log_base_callsign'
    && s.complete === true && integer(s.total) && p.contacts.length === Math.min(s.total, 25)
    && p.contacts_complete === (s.total <= 25) && s.worked_before === (s.total > 0)
    && integer(s.confirmed_count, 0, s.total) && s.confirmation_policy === 'xlog_cluster_paper_or_lotw_v1'
    && Array.isArray(s.confirmation_sources) && s.confirmation_sources.join('|') === 'paper_qsl|lotw'
    && s.requested_band === (q.band ?? null) && s.requested_exact_mode === (q.exactMode ?? null)
    && (q.band === undefined ? s.requested_band_worked === null : typeof s.requested_band_worked === 'boolean')
    && (q.exactMode === undefined ? s.requested_band_exact_mode_worked === null : typeof s.requested_band_exact_mode_worked === 'boolean')
}

export function createOwnerHistoryReader(options: { transport?: typeof historyReads; now?: () => number } = {}) {
  const transport = options.transport ?? historyReads, clock = options.now ?? (() => performance.now())
  let contextKey: string | null = null, generation = 0, selected = '', selectionGeneration = 0, pending = false
  let clockBad = false, lastClock = 0, revision: number | null = null
  let stamp: { generated: number; deadline: number } | null = null
  function unavailable<T>(reason: string, queryKey: string): OwnerReadState<T> {
    return { state: 'unavailable', reason, contextKey, queryKey }
  }
  let history: OwnerReadState<History> = unavailable('No current xLog history.', '')
  let recent: OwnerReadState<RecentContacts> = unavailable('No current xLog recent contacts.', 'recent')
  const clear = (reason: string) => {
    history = unavailable(reason, selected); recent = unavailable(reason, 'recent')
  }
  const now = () => {
    const current = clock()
    if (!Number.isFinite(current) || current < 0 || current < lastClock) {
      clockBad = true; clear('Read clock unavailable.'); return null
    }
    lastClock = current
    if (history.state === 'current' && current >= history.deadline) history = unavailable('xLog history expired.', selected)
    if (recent.state === 'current' && current >= recent.deadline) recent = unavailable('xLog recent contacts expired.', 'recent')
    return current
  }
  const invalidate = () => {
    ++generation; contextKey = null; revision = null; stamp = null
    clear('xLog connection changed.')
  }
  const synchronize = (binding: OwnerReadBinding) => {
    const next = bindingKey(binding)
    if (next !== contextKey) {
      ++generation; contextKey = next; revision = null; stamp = null
      clear(next ? 'Waiting for xLog history.' : 'xLog history unavailable.')
    }
    now()
  }
  const select = (q: CanonicalQuery | null) => {
    const key = q?.key ?? ''
    if (selected !== key) { ++selectionGeneration; selected = key; history = unavailable('Waiting for this callsign history.', selected) }
  }
  async function read(binding: OwnerReadBinding, q: CanonicalQuery | null): Promise<OwnerReadState<Payload>> {
    synchronize(binding)
    const key = q?.key ?? 'recent', started = now()
    if (clockBad || started === null || contextKey === null) return unavailable('xLog history unavailable.', key)
    if (pending) return unavailable('xLog history is busy.', key)
    const admittedGeneration = generation, admittedContext = contextKey, admittedSelection = selectionGeneration
    const loading: OwnerReadState<never> = { state: 'loading', contextKey, queryKey: key }
    if (q) history = loading; else recent = loading
    pending = true
    try {
      const response = await (q ? transport.history(binding.connectionToken!, q.callsign, q.band, q.exactMode) : transport.recent(binding.connectionToken!))
      const finished = now()
      if (generation !== admittedGeneration || contextKey !== admittedContext) {
        return { state: 'unavailable', reason: 'Read superseded.', contextKey: admittedContext, queryKey: key }
      }
      if (q && (selected !== q.key || admittedSelection !== selectionGeneration)) {
        // The shared native handle may have advanced its revision even though
        // this callsign is no longer selected. Do not keep unrelated old facts.
        clear('Read superseded.')
        return unavailable('Read superseded.', key)
      }
      if (clockBad || finished === null) return unavailable('Read clock unavailable.', key)
      if (!valid(response, binding, q)) { clear('xLog read scope or payload unavailable.'); return unavailable('xLog read scope or payload unavailable.', key) }
      const payload = response.payload, generated = payload.generated_at_unix_ms, nextRevision = payload.logbook_revision!
      if ((revision !== null && nextRevision < revision) || (stamp && generated < stamp.generated)) {
        clear('Older xLog history refused.'); return unavailable('Older xLog history refused.', key)
      }
      let deadline = started + response.remaining_valid_ms
      if (stamp?.generated === generated) deadline = Math.min(deadline, stamp.deadline)
      if (revision !== null && nextRevision !== revision) clear('xLog logbook changed.')
      revision = nextRevision; stamp = { generated, deadline }
      if (finished >= deadline) {
        if (q) history = unavailable('xLog history expired.', key)
        else recent = unavailable('xLog recent contacts expired.', key)
        return unavailable('xLog read expired in transit.', key)
      }
      const result = { state: 'current' as const, payload, deadline, contextKey, queryKey: key }
      if (q) history = result as OwnerReadState<History>
      else recent = result as OwnerReadState<RecentContacts>
      return result
    } catch {
      if (generation === admittedGeneration && contextKey === admittedContext) clear('xLog read failed or unsupported.')
      return { state: 'unavailable', reason: 'xLog read failed or unsupported.', contextKey: admittedContext, queryKey: key }
    } finally { pending = false }
  }
  return {
    synchronize, invalidate,
    isCurrent(result: OwnerReadState<Payload>, binding: OwnerReadBinding): boolean {
      synchronize(binding)
      return !clockBad && result.state === 'current' && result.contextKey === contextKey
        && (result === history || result === recent)
    },
    inspectHistory(binding: OwnerReadBinding, value: OwnerHistoryQuery): OwnerReadState<History> {
      synchronize(binding); const q = query(value); select(q)
      return q ? history : unavailable('Invalid history query.', '')
    },
    inspectRecent(binding: OwnerReadBinding): OwnerReadState<RecentContacts> { synchronize(binding); return recent },
    async readHistory(binding: OwnerReadBinding, value: OwnerHistoryQuery): Promise<OwnerReadState<History>> {
      synchronize(binding); const q = query(value); select(q)
      return q ? await read(binding, q) as OwnerReadState<History> : unavailable('Invalid history query.', '')
    },
    async readRecent(binding: OwnerReadBinding): Promise<OwnerReadState<RecentContacts>> { return await read(binding, null) as OwnerReadState<RecentContacts> },
  }
}
