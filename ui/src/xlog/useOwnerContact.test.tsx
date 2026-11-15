// @vitest-environment jsdom
import { act, cleanup, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import producer from './fixtures/contact-detail-producer.json'
import { disconnected, type Bridge, type ContactDetailEnvelope, type HistoryReadView, type OutboxEntry, type View } from './api'
import { useOwnerContact } from './useOwnerContact'
import { DRAFT_KEY } from './draftStore'
import type { OwnerContactDetail } from '../features/ownerContactDetail'
import { confirmDialog } from '../confirm'

vi.mock('../confirm', () => ({ confirmDialog: vi.fn(async () => false) }))
const A = producer.found.contact_id, B = '00000000-0000-4000-8000-000000000999'
const workspace = '00000000-0000-4000-8000-000000000100'
function deferred<T>() { let resolve!: (v: T) => void; const promise = new Promise<T>(done => { resolve = done }); return { promise, resolve } }
function setup() {
  let clock = 100, stamp = 1000
  vi.useFakeTimers(); vi.spyOn(performance, 'now').mockImplementation(() => clock)
  vi.spyOn(document, 'hidden', 'get').mockReturnValue(false)
  let live: View = { ...disconnected, paired: true, connected: true, connection_token: 'token-A', hello: {
    application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'server-A', workspace_id: workspace,
    capabilities: ['logbook.read', 'logbook.contact.read', 'logbook.write'] } }
  let session = 1
  const connection = (): Parameters<typeof useOwnerContact>[0] => ({ view: live, session,
    isCurrentReadConnection: (scope, caps = []) => live.connected && live.paired && session === scope.session
      && scope.connectionToken === live.connection_token && scope.serverInstance === live.hello?.server_instance
      && scope.workspaceId === live.hello?.workspace_id && caps.every(cap => live.hello!.capabilities.includes(cap)) })
  const payload = (id = A, patch: Partial<OwnerContactDetail> = {}): HistoryReadView<ContactDetailEnvelope> => {
    const detail = JSON.parse(JSON.stringify(producer.found)) as OwnerContactDetail
    detail.contact_id = id
    if (detail.editability.base) detail.editability.base.contact_id = id.toUpperCase()
    Object.assign(detail, patch)
    return { connection_token: live.connection_token!, remaining_valid_ms: 5000, payload: {
      protocol_version: 1, server_instance: live.hello!.server_instance, workspace_id: live.hello!.workspace_id,
      generation: live.hello!.workspace_id, worker_generation: 1, generated_at_unix_ms: ++stamp,
      max_age_ms: 5000, detail } }
  }
  let entries: OutboxEntry[] = []
  const api: Bridge = { status: vi.fn(), poll: vi.fn(), connect: vi.fn(), disconnect: vi.fn(), importPairing: vi.fn(),
    command: vi.fn(), history: vi.fn(), enqueue: vi.fn(async (mutation, request_id, workspace_id) => {
      entries = [{ queued: { version: 1, request_id: request_id!, workspace_id, mutation }, state: 'Pending' }]; return entries
    }), outbox: vi.fn(async () => entries), deliver: vi.fn(async request_id => ({ request_id, server_instance: 'server-A',
      workspace_id: workspace, status: 'uncertain' as const, message: null })) }
  const transport = { read: vi.fn(async (_token: string, id: string) => payload(id)) }
  const row = vi.fn((call: string, id: string | null | undefined) => call === 'QUERIED' && (id === A || id === B))
  const mount = () => renderHook(({ owner }) => useOwnerContact(owner, api, row, transport), { initialProps: { owner: connection() } })
  const hook = mount()
  return { ...hook, api, row, transport, payload, mount,
    open: async (id = A) => { await act(async () => { await hook.result.current.open('QUERIED', id) }) },
    advance: async (ms: number) => { clock += ms; await act(async () => { await vi.advanceTimersByTimeAsync(ms) }) },
    replace: (fn: (v: View) => View, nextSession = session) => { live = fn(live); session = nextSession; hook.rerender({ owner: connection() }) },
    synchronousReplace: (fn: (v: View) => View) => { live = fn(live) },
    entries: () => entries }
}
beforeEach(() => { localStorage.clear(); vi.mocked(confirmDialog).mockResolvedValue(false) })
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.useRealTimers() })

it('does no read on mount and opens only the exact canonical current-row ID', async () => {
  const s = setup(); expect(s.transport.read).not.toHaveBeenCalled()
  await act(async () => {
    expect(await s.result.current.open('QUERIED')).toBe(false)
    expect(await s.result.current.open('QUERIED', A.toUpperCase())).toBe(false)
    expect(await s.result.current.open('OTHER', A)).toBe(false)
  })
  await s.open(B)
  expect(s.transport.read.mock.calls).toEqual([['token-A', B]])
  expect(s.result.current.card?.external.contactId).toBe(B)
  expect(s.api.history).not.toHaveBeenCalled(); expect(s.api.command).not.toHaveBeenCalled()
})
it('retained row handlers cannot open a replacement connection even with the same UUID', async () => {
  const s = setup(), old = s.result.current.open
  s.replace(v => ({ ...v, connection_token: 'token-B' }), 2)
  await act(async () => { expect(await old('QUERIED', A)).toBe(false) })
  expect(s.transport.read).not.toHaveBeenCalled()
  await s.open(); expect(s.transport.read).toHaveBeenCalledWith('token-B', A)
})
it('synchronous read capability/connection retirement blocks a retained event before rerender', async () => {
  const s = setup(), open = s.result.current.open
  s.synchronousReplace(v => ({ ...v, connected: false }))
  await act(async () => { expect(await open('QUERIED', A)).toBe(false) })
  expect(s.transport.read).not.toHaveBeenCalled()
})
it('keeps typing and the card key through a deferred refresh, but blocks every write until a current read', async () => {
  const s = setup(); await s.open()
  const editor = s.result.current.card!.editor, key = s.result.current.card!.external.scopeKey
  act(() => { editor.set('comment', 'first') })
  const late = deferred<HistoryReadView<ContactDetailEnvelope>>()
  s.transport.read.mockReturnValueOnce(late.promise)
  await s.advance(2000)
  expect(editor.set('comment', 'first\nsecond')).toBe(true)
  await s.advance(6000)
  expect(editor.set('comment', 'first\nsecond\nthird')).toBe(true)
  expect(editor.inspect().canSave).toBe(false)
  await act(async () => { await editor.save(); await editor.submit(); await editor.retryQueue() })
  expect(s.api.enqueue).not.toHaveBeenCalled(); expect(s.api.deliver).not.toHaveBeenCalled()
  expect(s.transport.read).toHaveBeenCalledTimes(2)
  expect(s.result.current.card?.external.scopeKey).toBe(key)
  expect(s.result.current.card?.editor).toBe(editor)
  await act(async () => { late.resolve(s.payload()) })
  // The delayed reply's original budget is expired; only explicit Refresh resumes.
  expect(editor.inspect().canSave).toBe(false)
  await act(async () => s.result.current.refresh())
  expect(editor.inspect().canSave).toBe(true)
  expect(editor.inspect().fields?.comment).toBe('first\nsecond\nthird')
})
it('a new revision reports conflict without replacing local text or base', async () => {
  const s = setup(); await s.open(); const editor = s.result.current.card!.editor
  act(() => { editor.set('comment', 'retained') }); const base = editor.inspect().base
  const newer = s.payload(); newer.payload.detail.contact_revision!++
  s.transport.read.mockResolvedValueOnce(newer); await s.advance(2000)
  expect(editor.inspect().status).toBe('conflict'); expect(editor.inspect().canSave).toBe(false)
  expect(editor.inspect().base).toEqual(base); expect(editor.set('comment', 'retained further')).toBe(false)
  expect(editor.inspect().fields?.comment).toBe('retained')
})
it('explicit reconnection preserves the dirty controller and key but only explicit Refresh restores write admission',async()=>{
  const s=setup();await s.open()
  const editor=s.result.current.card!.editor,key=s.result.current.card!.external.scopeKey
  act(()=>{editor.set('comment','dirty\nretained')})
  const raw=localStorage.getItem(DRAFT_KEY),id=editor.inspect().requestId,reads=s.transport.read.mock.calls.length
  s.replace(v=>({...v,connected:false}))
  expect(editor.inspect().editable).toBe(false);expect(editor.inspect().canSave).toBe(false)
  s.replace(v=>({...v,connected:true,connection_token:'token-B'}),2)
  await act(async()=>{await vi.advanceTimersByTimeAsync(0)})
  expect(s.result.current.card!.editor).toBe(editor);expect(s.result.current.card!.external.scopeKey).toBe(key)
  expect(localStorage.getItem(DRAFT_KEY)).toBe(raw);expect(editor.inspect().requestId).toBe(id)
  expect(s.transport.read).toHaveBeenCalledTimes(reads);expect(editor.inspect().canSave).toBe(false)
  await act(async()=>s.result.current.refresh())
  expect(s.transport.read).toHaveBeenLastCalledWith('token-B',A)
  expect(editor.inspect().canSave).toBe(true);expect(editor.inspect().fields?.comment).toBe('dirty\nretained')
  expect(s.api.enqueue).not.toHaveBeenCalled();expect(s.api.deliver).not.toHaveBeenCalled()
})
it('write capability loss blocks typing and enqueue synchronously, without deleting the draft', async () => {
  const s = setup(); await s.open(); const editor = s.result.current.card!.editor
  act(() => { editor.set('comment', 'retained') }); const raw = localStorage.getItem(DRAFT_KEY)
  s.synchronousReplace(v => ({ ...v, hello: { ...v.hello!, capabilities: ['logbook.contact.read'] } }))
  expect(editor.set('comment', 'lost')).toBe(false); await act(async () => editor.save())
  expect(s.api.enqueue).not.toHaveBeenCalled(); expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
})
it('read failures pause cadence until explicit Refresh; hiding never creates a second request', async () => {
  const s = setup(); await s.open()
  s.transport.read.mockRejectedValueOnce(Error('offline')); await s.advance(2000); await s.advance(10000)
  expect(s.transport.read).toHaveBeenCalledTimes(2)
  act(() => { vi.spyOn(document, 'hidden', 'get').mockReturnValue(true); document.dispatchEvent(new Event('visibilitychange')) })
  await act(async () => s.result.current.refresh()); expect(s.transport.read).toHaveBeenCalledTimes(2)
  await act(async () => { vi.spyOn(document, 'hidden', 'get').mockReturnValue(false); document.dispatchEvent(new Event('visibilitychange')) })
  expect(s.transport.read).toHaveBeenCalledTimes(3)
})
it('A to B to A coalesces only the last explicit selection and rejects the old A answer', async () => {
  const s = setup(), late = deferred<HistoryReadView<ContactDetailEnvelope>>()
  s.transport.read.mockReturnValueOnce(late.promise)
  await s.open(A); await s.open(B); await s.open(A)
  expect(s.transport.read).toHaveBeenCalledTimes(1)
  await act(async () => { late.resolve(s.payload(A)) })
  expect(s.transport.read.mock.calls.map(call => call[1])).toEqual([A, A])
  expect(s.result.current.card?.external.read.state).toBe('current')
})
it('dirty replacement requires the existing guard; a pending mutation blocks replacement and close', async () => {
  const s = setup(); await s.open(); const editor = s.result.current.card!.editor
  act(() => { editor.set('comment', 'retained') }); await s.open(B)
  expect(s.result.current.card?.external.contactId).toBe(A)
  expect(confirmDialog).toHaveBeenCalledOnce()
  const late = deferred<OutboxEntry[]>(); vi.mocked(s.api.enqueue).mockReturnValueOnce(late.promise)
  let operation!: Promise<void>; act(() => { operation = editor.save() })
  await s.open(B); await act(async () => s.result.current.close())
  expect(s.result.current.card?.editor).toBe(editor)
  await act(async () => { late.resolve([]); await operation })
})
it('explicit Resume recovers a nonvisible old contact and same frozen ID; startup never reads or sends', async () => {
  const s = setup(); await s.open(); const editor = s.result.current.card!.editor
  act(() => { editor.set('comment', 'retained') }); await act(async () => editor.save())
  const raw = localStorage.getItem(DRAFT_KEY), id = editor.inspect().requestId
  await act(async () => s.result.current.close())
  s.row.mockReturnValue(false)
  expect(s.result.current.savedAvailable).toBe(true)
  s.unmount(); const restarted = s.mount()
  const before = s.transport.read.mock.calls.length
  await s.advance(10000); expect(s.transport.read).toHaveBeenCalledTimes(before)
  await act(async () => { expect(await restarted.result.current.resume()).toBe(true) })
  const restored = restarted.result.current.card!.editor
  expect(restored.inspect().requestId).toBe(id); expect(localStorage.getItem(DRAFT_KEY)).toBe(raw)
  expect(s.api.enqueue).toHaveBeenCalledOnce(); expect(s.api.deliver).not.toHaveBeenCalled()
  await act(async () => restored.inspectQueue())
  expect(restored.inspect().canSubmit).toBe(true)
})
it('read-only, deleted and missing outcomes never grant an editable base', async () => {
  const s = setup(), readonly = s.payload()
  readonly.payload.detail.contact_revision = 0
  readonly.payload.detail.editability = { allowed: false, reason: 'legacy', allowed_fields: [], base: null }
  s.transport.read.mockResolvedValueOnce(readonly); await s.open()
  expect(s.result.current.card!.editor.inspect().editable).toBe(false)
  for (const status of ['deleted', 'not_found'] as const) {
    const value = s.payload(A, { status, fields: null, editability: { allowed: false, reason: status, allowed_fields: [], base: null } })
    s.transport.read.mockResolvedValueOnce(value); await act(async () => s.result.current.refresh())
    expect(s.result.current.card?.external.read).toMatchObject({ state: 'current', detail: { status } })
    expect(s.result.current.card!.editor.inspect().canSave).toBe(false)
  }
})
it('unmount retires a pending response and never starts a queued request', async () => {
  const s = setup(), late = deferred<HistoryReadView<ContactDetailEnvelope>>()
  s.transport.read.mockReturnValueOnce(late.promise); await s.open(); s.unmount()
  await act(async () => { late.resolve(s.payload()); await vi.advanceTimersByTimeAsync(10000) })
  expect(s.transport.read).toHaveBeenCalledOnce(); expect(s.api.enqueue).not.toHaveBeenCalled()
})
