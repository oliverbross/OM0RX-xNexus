// SPDX-License-Identifier: GPL-3.0-only
// Selected-contact IPC admission. Native SDK validates the complete producer DTO;
// this boundary checks IPC scope/selection and preserves its original deadline.
import { contactReads, type ContactDetailEnvelope, type HistoryReadView } from './api'
import type { OwnerReadBinding } from './ownerHistoryReader'

export type OwnerContactResult =
  | { state: 'current'; payload: ContactDetailEnvelope; deadline: number; contextKey: string; contactId: string }
  | { state: 'loading'; contextKey: string; contactId: string }
  | { state: 'unavailable'; contextKey: string | null; contactId: string; reason: string }

const object = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === 'object' && !Array.isArray(v)
const integer = (v: unknown, min = 0, max = 9_007_199_254_740_990): v is number =>
  typeof v === 'number' && Number.isSafeInteger(v) && v >= min && v <= max
const token = (v: unknown): v is string => typeof v === 'string' && v.length > 0 && v.length <= 128 && !/[\u0000-\u001f\u007f]/.test(v)
const id = (v: unknown): v is string => typeof v === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(v)
function identity(binding: OwnerReadBinding): string | null {
  const h = binding.owner.hello
  return integer(binding.sessionGeneration) && token(binding.connectionToken)
    && h?.protocol_version === 1 && h.application === 'OM0RX-xLog'
    && token(h.server_instance) && token(h.workspace_id)
    ? JSON.stringify([binding.sessionGeneration, binding.connectionToken, h.server_instance, h.workspace_id]) : null
}
function context(binding: OwnerReadBinding): string | null {
  return binding.owner.paired && binding.owner.connected && binding.owner.hello?.capabilities.includes('logbook.contact.read')
    ? identity(binding) : null
}
const sameKeys = (v: Record<string, unknown>, keys: string[]) => Object.keys(v).sort().join('|') === keys.sort().join('|')
/** This is not a substitute for the Rust DTO codec or endpoint validator. */
function valid(value: unknown, binding: OwnerReadBinding, contactId: string): value is HistoryReadView<ContactDetailEnvelope> {
  if (!object(value) || !sameKeys(value, ['payload', 'connection_token', 'remaining_valid_ms'])
    || value.connection_token !== binding.connectionToken || !integer(value.remaining_valid_ms, 1, 5000) || !object(value.payload)) return false
  const p = value.payload, d = p.detail, h = binding.owner.hello!
  if (!sameKeys(p, ['protocol_version','server_instance','workspace_id','worker_generation','generation','generated_at_unix_ms','max_age_ms','detail'])
    || p.protocol_version !== 1 || p.server_instance !== h.server_instance || p.workspace_id !== h.workspace_id
    || p.generation !== h.workspace_id || !integer(p.worker_generation, 1) || !integer(p.generated_at_unix_ms, 1)
    || p.max_age_ms !== 5000 || !object(d) || d.schema_version !== 1 || d.contact_id !== contactId
    || !integer(d.logbook_revision) || !(d.contact_revision === null || integer(d.contact_revision))
    || !object(d.editability) || !object(d.locate)) return false
  if (d.status === 'found') return object(d.fields)
    && ['contact','radio','station','own_station','references','confirmations','awards','uploads','contest'].every(k => object(d.fields) && object(d.fields[k]))
  return ['deleted','not_found','unavailable'].includes(String(d.status)) && d.fields === null
    && d.editability.allowed === false && d.editability.base === null
}
function freeze<T>(value: T): T {
  if (value !== null && typeof value === 'object') {
    for (const child of Object.values(value)) freeze(child)
    Object.freeze(value)
  }
  return value
}

/** Explicit reads only. No retry, HTTP fallback, history search or local log.
 * Call synchronize/inspect on every host render and invalidate synchronously on
 * connection intent/close. A new selection retires old facts even while busy. */
export function createOwnerContactReader(options: { transport?: typeof contactReads; now?: () => number } = {}) {
  const transport = options.transport ?? contactReads, clock = options.now ?? (() => performance.now())
  let contextKey: string | null = null, selected = '', epoch = 0, pending = false, clockBad = false, lastClock = 0
  let evidenceIdentity: string | null = null
  let worker: number | null = null, revision: number | null = null, poisonedContext: string | null = null
  let stamp: { generated: number; deadline: number } | null = null
  const unavailable = (reason: string): OwnerContactResult => ({ state: 'unavailable', contextKey, contactId: selected, reason })
  let held: OwnerContactResult = unavailable('No selected xLog contact.')
  function retire(reason: string) { ++epoch; held = unavailable(reason) }
  function now(): number | null {
    let n: number
    try { n = clock() } catch { n = NaN }
    if (!Number.isFinite(n) || n < 0 || n < lastClock) { clockBad = true; retire('Contact read clock unavailable.'); return null }
    lastClock = n
    if (held.state === 'current' && n >= held.deadline) held = unavailable('Contact details expired.')
    return n
  }
  function synchronize(binding: OwnerReadBinding, contactId: string | null) {
    const nextIdentity = identity(binding), next = context(binding), selection = id(contactId) ? contactId : ''
    // Missing availability/hello/token is not proof of a replacement identity.
    // A same-identity recovery keeps all evidence high-water and poison fences.
    if (nextIdentity !== null && nextIdentity !== evidenceIdentity) {
      evidenceIdentity = nextIdentity; worker = null; revision = null; stamp = null; poisonedContext = null
    }
    if (next !== contextKey) {
      contextKey = next
      retire('xLog connection changed.')
    }
    if (selection !== selected) { selected = selection; retire('Contact selection changed.') }
    now()
  }
  function inspect(binding: OwnerReadBinding, contactId: string | null): OwnerContactResult {
    synchronize(binding, contactId); return held
  }
  return {
    synchronize, inspect,
    // Keep high-water marks until the actual connection key changes. Closing or
    // refreshing a card must not renew repeated evidence or clear worker poison.
    invalidate() { retire('Contact read retired.'); selected = '' },
    isCurrent(result: OwnerContactResult, binding: OwnerReadBinding, contactId: string | null) {
      return inspect(binding, contactId) === result && result.state === 'current' && !clockBad && poisonedContext !== contextKey
    },
    async read(binding: OwnerReadBinding, contactId: string): Promise<OwnerContactResult> {
      synchronize(binding, contactId)
      // Retire before clock/admission failure, mirroring the native capture fence.
      retire('Waiting for selected contact.')
      const started = now(), admittedEpoch = epoch, admittedContext = contextKey, admittedId = selected
      if (clockBad || started === null || !admittedContext || !admittedId || poisonedContext === admittedContext) return held
      if (pending) { held = unavailable('Contact read is busy.'); return held }
      pending = true; held = { state: 'loading', contextKey: admittedContext, contactId: admittedId }
      const active = () => epoch === admittedEpoch && contextKey === admittedContext && selected === admittedId
      try {
        const response = await transport.read(binding.connectionToken!, admittedId), finished = now()
        if (!active()) return { state: 'unavailable', contextKey: admittedContext, contactId: admittedId, reason: 'Contact read superseded.' }
        if (clockBad || finished === null || !valid(response, binding, admittedId)) { retire('Contact details unavailable.'); return held }
        const p = response.payload
        if (worker !== null && p.worker_generation !== worker) { poisonedContext = admittedContext; retire('xLog worker changed.'); return held }
        if (revision !== null && p.detail.logbook_revision < revision || stamp && p.generated_at_unix_ms < stamp.generated) {
          retire('Older contact evidence refused.'); return held
        }
        let deadline = started + response.remaining_valid_ms
        if (stamp?.generated === p.generated_at_unix_ms) deadline = Math.min(deadline, stamp.deadline)
        worker = p.worker_generation; revision = p.detail.logbook_revision; stamp = { generated: p.generated_at_unix_ms, deadline }
        if (finished >= deadline) { retire('Contact details expired in transit.'); return held }
        // Keep caller mutation from changing an admitted snapshot or edit base.
        const payload = freeze(JSON.parse(JSON.stringify(p)) as ContactDetailEnvelope)
        held = Object.freeze({ state: 'current', payload, deadline, contextKey: admittedContext, contactId: admittedId })
        return held
      } catch {
        if (active()) retire('Contact read failed or unsupported.')
        return { state: 'unavailable', contextKey: admittedContext, contactId: admittedId, reason: 'Contact read failed or unsupported.' }
      } finally { pending = false }
    },
  }
}
