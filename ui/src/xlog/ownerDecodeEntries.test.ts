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
    expect(rows[0].worked).toBe(false)
    expect(rows[0].directedToMe).toBeNull()
    expect(rows[1].external.status).toBe('unknown')
    expect(rows[1].external.score).toBeNull()
  })
  it.each(['New Call', 'New Band', 'New Mode', 'New Band+Mode', 'Worked', 'Confirmed'])('keeps authoritative %s call history separately from DXCC', callStatus => {
    const copy = structuredClone(view)
    copy.rows[0].needs.owner!.call_status = callStatus
    const rows = ownerDecodeEntries({ view: copy, owner, now: 101, leases: receiveNeeds(new Map(), copy, 100, owner), needsAvailable: true, mycall: null })
    expect(rows[0].external.callStatus).toBe(callStatus)
    expect(rows[0].external.dxccStatus).toBe('ATNO')
    expect(rows[0].external.classificationMode).toBe('DIGI')
    expect(rows[0].worked).toBe(callStatus !== 'New Call')
  })
  it('preserves known call history when the DXCC need is unknown', () => {
    const copy = structuredClone(view), facts = copy.rows[0].needs
    facts.status = facts.owner!.need.status = 'unknown'
    facts.score = facts.owner!.need.score = null
    facts.reason_code = facts.owner!.need.reason_code = 'country_unknown'
    facts.owner!.call_status = 'New Band'
    facts.owner!.dxcc_status = 'Unknown'
    const rows = ownerDecodeEntries({ view: copy, owner, now: 101, leases: receiveNeeds(new Map(), copy, 100, owner), needsAvailable: true, mycall: null })
    expect(rows[0].external.status).toBe('unknown')
    expect(rows[0].external.callStatus).toBe('New Band')
    expect(rows[0].worked).toBe(true)
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
