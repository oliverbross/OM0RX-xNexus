// SPDX-License-Identifier: GPL-3.0-only
import { expect, it, vi } from 'vitest'
import corpus from '../../../integrations/xlog/fixtures/contact-detail-swift.json'
import { createOwnerContactReader } from './ownerContactReader'
import type { ContactDetailEnvelope, HistoryReadView } from './api'
import type { OwnerReadBinding } from './ownerHistoryReader'

const ID = corpus.found.contact_id, OTHER = '00000000-0000-4000-8000-000000000999'
const clone = <T,>(v: T): T => JSON.parse(JSON.stringify(v)) as T
function binding(): OwnerReadBinding { return { sessionGeneration: 1, connectionToken: 'synthetic-connection', owner: {
  paired: true, connected: true, hello: { application: 'OM0RX-xLog', protocol_version: 1,
    server_instance: 'synthetic-server', workspace_id: 'synthetic-workspace', capabilities: ['logbook.contact.read'] },
} } }
function wire(name: keyof typeof corpus = 'found'): HistoryReadView<ContactDetailEnvelope> {
  return { connection_token: 'synthetic-connection', remaining_valid_ms: 500,
    payload: { protocol_version: 1, server_instance: 'synthetic-server', workspace_id: 'synthetic-workspace',
      generation: 'synthetic-workspace', worker_generation: 1, generated_at_unix_ms: 10_000, max_age_ms: 5000,
      detail: clone(corpus[name]) as ContactDetailEnvelope['detail'] } }
}
function setup() {
  let time = 100
  const transport = { read: vi.fn(async (_token: string, _id: string) => wire()) }, b = binding()
  const reader = createOwnerContactReader({ transport, now: () => time })
  return { reader, transport, b, time: (n: number) => { time = n } }
}
it.each(['found', 'deleted', 'not_found'] as const)('replays actual Swift %s DTO without filling absent facts', async name => {
  const t = setup(), response = wire(name)
  t.transport.read.mockResolvedValue(response)
  const result = await t.reader.read(t.b, response.payload.detail.contact_id)
  expect(result.state).toBe('current')
  if (result.state !== 'current') throw Error('Expected current')
  expect(result.payload.detail).toEqual(corpus[name])
  expect(t.transport.read).toHaveBeenCalledExactlyOnceWith('synthetic-connection', response.payload.detail.contact_id)
  expect(result.deadline).toBe(600)
})
it('construction and inspection do not invoke a read', () => {
  const t = setup(); expect(t.reader.inspect(t.b, ID).state).toBe('unavailable')
  expect(t.transport.read).not.toHaveBeenCalled()
})
it('preserves revision-zero retained facts as read-only rather than hiding the contact', async () => {
  const t = setup(), v = wire()
  v.payload.detail.contact_revision = 0
  v.payload.detail.editability = { allowed: false, reason: 'revision_unavailable', allowed_fields: [], base: null }
  t.transport.read.mockResolvedValue(v)
  const current = await t.reader.read(t.b, ID)
  expect(current.state).toBe('current')
  if (current.state === 'current') {
    expect(current.payload.detail.contact_revision).toBe(0)
    expect(current.payload.detail.editability.allowed).toBe(false)
    expect(current.payload.detail.fields).toEqual(corpus.found.fields)
  }
})
it.each(['missing_capability','disconnected','unpaired','missing_token','uppercase_id','missing_id'] as const)('refuses %s without IPC', async kind => {
  const t = setup()
  if (kind === 'missing_capability') t.b.owner.hello!.capabilities = ['logbook.read']
  if (kind === 'disconnected') t.b.owner.connected = false
  if (kind === 'unpaired') t.b.owner.paired = false
  if (kind === 'missing_token') t.b.connectionToken = null
  expect((await t.reader.read(t.b, kind === 'uppercase_id' ? ID.toUpperCase() : kind === 'missing_id' ? '' : ID)).state).toBe('unavailable')
  expect(t.transport.read).not.toHaveBeenCalled()
})
it.each(['connection_token','server_instance','workspace_id','generation','worker_generation','max_age_ms','contact_id','missing_group','missing_null','remaining','extra_wrapper'] as const)('refuses invalid IPC %s', async kind => {
  const t = setup(), v = wire()
  if (kind === 'connection_token') v.connection_token = 'foreign'
  if (kind === 'server_instance' || kind === 'workspace_id' || kind === 'generation') v.payload[kind] = 'foreign'
  if (kind === 'worker_generation') v.payload.worker_generation = 0
  if (kind === 'max_age_ms') (v.payload as unknown as { max_age_ms: number }).max_age_ms = 5001
  if (kind === 'contact_id') v.payload.detail.contact_id = OTHER
  if (kind === 'missing_group') delete (v.payload.detail.fields as unknown as Record<string, unknown>).contact
  if (kind === 'missing_null') delete (v.payload.detail as unknown as Record<string, unknown>).contact_revision
  if (kind === 'remaining') v.remaining_valid_ms = 5001
  if (kind === 'extra_wrapper') Object.assign(v, { cached: true })
  t.transport.read.mockResolvedValue(v)
  expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
})
it('expires at the pre-invoke monotonic anchor, including in-transit expiry', async () => {
  const t = setup(), current = await t.reader.read(t.b, ID)
  t.time(599); expect(t.reader.isCurrent(current, t.b, ID)).toBe(true)
  t.time(600); expect(t.reader.isCurrent(current, t.b, ID)).toBe(false)
  t.transport.read.mockImplementation(async () => { t.time(1200); return wire() })
  expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
})
it('cannot renew a repeated producer stamp by refresh, closing, or inspection', async () => {
  const t = setup(); await t.reader.read(t.b, ID)
  t.reader.invalidate(); t.time(400)
  const repeated = await t.reader.read(t.b, ID)
  expect(repeated.state === 'current' && repeated.deadline).toBe(600)
  t.time(700); expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
})
it.each(['timestamp','revision'] as const)('retires retained facts on older %s', async kind => {
  const t = setup(), first = await t.reader.read(t.b, ID), v = wire()
  if (kind === 'timestamp') v.payload.generated_at_unix_ms--
  else v.payload.detail.logbook_revision--
  t.transport.read.mockResolvedValue(v)
  expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
  expect(t.reader.isCurrent(first, t.b, ID)).toBe(false)
})
it('worker replacement poisons the same connection even after an explicit refresh', async () => {
  const t = setup(); await t.reader.read(t.b, ID)
  const v = wire(); v.payload.worker_generation = 2; v.payload.generated_at_unix_ms++
  t.transport.read.mockResolvedValue(v)
  expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
  t.reader.invalidate()
  expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
  expect(t.transport.read).toHaveBeenCalledTimes(2)
  t.b.connectionToken = 'new-connection'; v.connection_token = 'new-connection'
  expect((await t.reader.read(t.b, ID)).state).toBe('current')
})
it.each(['paired','connected','capability','token','hello'] as const)('availability-only %s loss cannot renew repeated evidence or clear worker poison', async kind => {
  const t = setup(); await t.reader.read(t.b, ID)
  const original = clone(t.b)
  function lose() {
    if (kind === 'paired') t.b.owner.paired = false
    if (kind === 'connected') t.b.owner.connected = false
    if (kind === 'capability') t.b.owner.hello!.capabilities = []
    if (kind === 'token') t.b.connectionToken = null
    if (kind === 'hello') t.b.owner.hello = null
    expect(t.reader.inspect(t.b, ID).state).toBe('unavailable')
  }
  function restore() { Object.assign(t.b, clone(original)) }
  lose(); t.time(400); restore()
  const repeated = await t.reader.read(t.b, ID)
  expect(repeated.state === 'current' && repeated.deadline).toBe(600)
  const v = wire(); v.payload.worker_generation = 2; v.payload.generated_at_unix_ms++
  t.transport.read.mockResolvedValue(v)
  expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
  lose(); restore(); expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
  expect(t.transport.read).toHaveBeenCalledTimes(3)
})
it.each(['session','token','server','workspace','capability','disconnect','selection','close'] as const)('retires a delayed reply on %s change without fallback', async kind => {
  const t = setup(); let resolve!: (v: HistoryReadView<ContactDetailEnvelope>) => void
  t.transport.read.mockReturnValue(new Promise(r => { resolve = r }))
  const pending = t.reader.read(t.b, ID)
  if (kind === 'session') t.b.sessionGeneration++
  if (kind === 'token') t.b.connectionToken = 'new-token'
  if (kind === 'server') t.b.owner.hello!.server_instance = 'new-server'
  if (kind === 'workspace') t.b.owner.hello!.workspace_id = 'new-workspace'
  if (kind === 'capability') t.b.owner.hello!.capabilities = []
  if (kind === 'disconnect') t.b.owner.connected = false
  if (kind === 'close') t.reader.invalidate()
  else t.reader.synchronize(t.b, kind === 'selection' ? OTHER : ID)
  resolve(wire()); expect((await pending).state).toBe('unavailable')
  expect(t.reader.inspect(t.b, kind === 'selection' ? OTHER : ID).state).toBe('unavailable')
  expect(t.transport.read).toHaveBeenCalledTimes(1)
})
it('busy selection A→B→A supersedes the first reply with no queued IPC', async () => {
  const t = setup(); let resolve!: (v: HistoryReadView<ContactDetailEnvelope>) => void
  t.transport.read.mockReturnValueOnce(new Promise(r => { resolve = r }))
  const first = t.reader.read(t.b, ID)
  expect((await t.reader.read(t.b, OTHER)).state).toBe('unavailable')
  t.reader.synchronize(t.b, ID); resolve(wire())
  expect((await first).state).toBe('unavailable')
  expect(t.transport.read).toHaveBeenCalledTimes(1)
  expect((await t.reader.read(t.b, ID)).state).toBe('current')
})
it.each([NaN, -1, 99] as const)('latches a failed monotonic clock %s instead of reviving facts', async bad => {
  const t = setup(), first = await t.reader.read(t.b, ID)
  t.time(bad); expect(t.reader.isCurrent(first, t.b, ID)).toBe(false)
  t.time(101); expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
  expect(t.transport.read).toHaveBeenCalledTimes(1)
})
it('a thrown clock retires facts and makes no IPC request', async () => {
  const transport = { read: vi.fn(async () => wire()) }
  const reader = createOwnerContactReader({ transport, now: () => { throw Error('clock') } })
  expect((await reader.read(binding(), ID)).state).toBe('unavailable'); expect(transport.read).not.toHaveBeenCalled()
})
it('copies and freezes admitted facts, including nested edit base and lease', async () => {
  const t = setup(), v = wire(); t.transport.read.mockResolvedValue(v)
  const current = await t.reader.read(t.b, ID)
  if (current.state !== 'current') throw Error('Expected current')
  v.payload.detail.contact_id = OTHER
  expect(current.payload.detail.contact_id).toBe(ID)
  expect(Object.isFrozen(current)).toBe(true)
  expect(Object.isFrozen(current.payload.detail.editability.base)).toBe(true)
})
it('an IPC failure retires facts and never becomes an empty successful contact', async () => {
  const t = setup(), first = await t.reader.read(t.b, ID)
  t.transport.read.mockRejectedValue(Error('unavailable'))
  expect((await t.reader.read(t.b, ID)).state).toBe('unavailable')
  expect(t.reader.isCurrent(first, t.b, ID)).toBe(false)
  expect(t.transport.read).toHaveBeenCalledTimes(2)
})
