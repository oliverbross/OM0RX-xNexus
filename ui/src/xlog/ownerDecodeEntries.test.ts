import { describe, expect, it } from 'vitest'
import fixture from './fixtures/decoded-needs.json'
import type { RxView } from './rxApi'
import { receiveNeeds } from './rxNeeds'
import { ownerDecodeEntries } from './ownerDecodeEntries'
import { rowTimeMs } from '../decodeHistory'

describe('original decode presentation from native owner rows', () => {
  const view = fixture as RxView
  const owner = { server_instance: view.rows[0].context.server_instance, workspace_id: view.rows[0].context.workspace_id }
  it('retains actual Rust IDs, exact period and separate owner facts without HARQ or B4 inventions', () => {
    const rows = ownerDecodeEntries({ view, owner, now: 101, leases: receiveNeeds(new Map(), view, 100, owner), needsAvailable: true, mycall: null })
    expect(rows[0].id).toBe(view.rows[0].decode_id)
    expect(rowTimeMs(rows[0], 'FT8')).toBe(view.rows[0].host_period_start_unix_us / 1000)
    expect(rows[0].external.status).toBe('needed')
    expect(rows[0].external.score).toBe(view.rows[0].needs.score)
    expect(rows[0].rv).toBeNull()
    expect(rows[0].worked).toBeNull()
    expect(rows[0].directedToMe).toBeNull()
    expect(rows[1].external.status).toBe('unknown')
    expect(rows[1].external.score).toBeNull()
  })
  it('expires classification while retaining the exact raw signal measurements', () => {
    const rows = ownerDecodeEntries({ view, owner, now: 5000, leases: receiveNeeds(new Map(), view, 100, owner), needsAvailable: true, mycall: null })
    expect(rows[0].external.status).toBe('unknown')
    expect(rows[0].external.score).toBeNull()
    expect(rows[0].snr).toBe(view.rows[0].snr_db)
    expect(rows[0].dtSec).toBe(view.rows[0].time_offset_seconds)
    expect(rows[0].message).toBe(view.rows[0].message)
  })
  it('keeps two same-message/frequency rows with different immutable producer IDs', () => {
    const duplicate = structuredClone(view.rows[0])
    duplicate.decode_id += '-other'
    const sameSignal = { ...view, rows: [view.rows[0], duplicate] }
    const rows = ownerDecodeEntries({ view: sameSignal, owner, now: 100, leases: new Map(), needsAvailable: false, mycall: null })
    expect(rows.map(row => row.id)).toEqual([view.rows[0].decode_id, duplicate.decode_id])
    expect(rows.every(row => row.external.status === 'unknown')).toBe(true)
  })
})
