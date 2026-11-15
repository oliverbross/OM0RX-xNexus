// SPDX-License-Identifier: GPL-3.0-only
// Actual synthetic Swift projector bytes, unchanged. Only the explicitly
// synthetic Rust-identity/IPC wrapper and monotonic test lease are supplied.
// Historical contract replay is not a current native session or live lease.
import { expect, it } from 'vitest'
import main from './fixtures/owner-history-main.json'
import lifecycle from './fixtures/owner-history-lifecycle.json'
import { createOwnerHistoryReader, type OwnerReadBinding, type OwnerHistoryQuery } from './ownerHistoryReader'
import { ownerRecallRead, ownerRecentRead } from './ownerHistoryPresentation'
import type { History, HistoryReadView, RecentContacts } from './api'

type Raw = Record<string, unknown>
function wrap<T>(raw: Raw): { binding: OwnerReadBinding; response: HistoryReadView<T> } {
  expect(main.synthetic).toBe(true); expect(main.fixture_schema).toBe(1)
  expect(lifecycle.synthetic).toBe(true); expect(lifecycle.fixture_schema).toBe(1)
  for (const field of ['server_instance', 'workspace_id', 'protocol_version']) expect(raw).not.toHaveProperty(field)
  expect(typeof raw.generation).toBe('string')
  const binding: OwnerReadBinding = { sessionGeneration: 1, connectionToken: 'synthetic-producer-replay',
    owner: { paired: true, connected: true, hello: { application: 'OM0RX-xLog', protocol_version: 1,
      server_instance: 'synthetic-producer-replay', workspace_id: raw.generation as string, capabilities: ['logbook.read'] } } }
  const response = { connection_token: binding.connectionToken!, remaining_valid_ms: 1000,
    payload: { ...raw, protocol_version: 1, server_instance: 'synthetic-producer-replay', workspace_id: raw.generation } as T }
  return { binding, response }
}
const histories: { name: string; raw: Raw; query: OwnerHistoryQuery }[] = [
  { name: '30-contact full summary with 25-row page', raw: main.history, query: { callsign: 'VK8RX', band: '20m', exactMode: 'FT4' } },
  { name: 'UTF-8 truncated view-only comment', raw: main.combining_comment, query: { callsign: 'K1ABC' } },
  { name: 'band-only history', raw: lifecycle.band_only_history, query: { callsign: 'VK8RX', band: '20m' } },
  { name: 'edited history', raw: lifecycle.after_edit_history, query: { callsign: 'VK8RX', band: '20m' } },
]
it.each(histories)('accepts actual owner $name without rewriting facts', async ({ raw, query }) => {
  const before = JSON.stringify(raw), { binding, response } = wrap<History>(raw)
  let now = 100
  const reader = createOwnerHistoryReader({ now: () => now, transport: {
    history: async () => response, recent: async () => { throw new Error('Unexpected recent request') },
  } })
  const result = await reader.readHistory(binding, query)
  expect(result.state).toBe('current')
  if (result.state !== 'current') throw new Error('Actual producer history refused')
  expect(result.payload.generated_at_unix_ms).toBe(raw.generated_at_unix_ms)
  expect(result.payload.contacts).toBe(raw.contacts)
  const card = ownerRecallRead(result, query.exactMode !== undefined)
  expect(card.state).toBe('current')
  if (raw === main.history && card.state === 'current') {
    expect(card.history.count).toBe(30); expect(card.history.qsos).toHaveLength(25)
  }
  if (raw === main.combining_comment && card.state === 'current') {
    expect(card.history.qsos[0].canOpen).toBe(false)
    expect(card.history.qsos[0].comment).toBe(main.combining_comment.contacts[0].comment)
  }
  now = 1100; expect(reader.inspectHistory(binding, query).state).toBe('unavailable')
  expect(JSON.stringify(raw)).toBe(before)
})
it.each([
  { name: 'recent 31 contacts', raw: main.recent },
  { name: 'empty recent', raw: lifecycle.empty_recent },
  { name: 'after delete', raw: lifecycle.after_delete_recent },
])('accepts actual owner $name and preserves total', async ({ raw }) => {
  const before = JSON.stringify(raw), { binding, response } = wrap<RecentContacts>(raw)
  const reader = createOwnerHistoryReader({ now: () => 100, transport: {
    history: async () => { throw new Error('Unexpected history request') }, recent: async () => response,
  } })
  const result = await reader.readRecent(binding)
  expect(result.state).toBe('current')
  const pane = ownerRecentRead(result)
  expect(pane.state).toBe('current')
  if (pane.state === 'current') {
    expect(pane.total).toBe(raw.total); expect(pane.rows).toHaveLength(Math.min(raw.total, 5))
  }
  expect(JSON.stringify(raw)).toBe(before)
})
it('treats both actual retired-workspace refusals as unavailable, never empty', async () => {
  expect(lifecycle.wrong_generation).toHaveLength(2)
  const { binding } = wrap<History>(main.history)
  for (const raw of lifecycle.wrong_generation) {
    expect(raw.status).toBe('workspace_changed'); expect(raw).not.toHaveProperty('contacts')
    const reader = createOwnerHistoryReader({ now: () => 100, transport: {
      history: async () => ({ connection_token: binding.connectionToken!, remaining_valid_ms: 1000, payload: raw as unknown as History }),
      recent: async () => { throw new Error('Unexpected recent request') },
    } })
    expect((await reader.readHistory(binding, { callsign: 'VK8RX' })).state).toBe('unavailable')
  }
})
