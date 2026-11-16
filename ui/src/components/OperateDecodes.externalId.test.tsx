// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render } from '@testing-library/react'
import { OperateDecodes } from './OperateDecodes'
import type { ExternalDecodeEntry } from '../decodeHistory'
vi.mock('../api', () => ({ openQrzPage: vi.fn() }))
beforeEach(() => localStorage.clear())
afterEach(cleanup)
const entry: ExternalDecodeEntry = { id: 'immutable-owner-row', slot: 4, at: 60_000,
  from: 'K1ABC', snr: -10, dtSec: .1, freqHz: 1200, message: 'CQ K1ABC FN42',
  isCq: true, directedToMe: false, worked: null, tier: 'FT8', rv: null,
  external: { periodStartMs: 60_000, status: 'unknown', score: null, reasons: [], reason: null } }
function setup(exact = true) {
  const onCall = vi.fn(), onCallExternal = vi.fn(), onSelectDecode = vi.fn(), onSetRx = vi.fn()
  const result = render(<OperateDecodes decodes={[]} slot={4} rxOffsetHz={1500} band="20m" tier="FT8" harqRescues={null}
    external={{ contextKey: 'owner-receiver', entries: [entry] }} onCall={onCall}
    onCallExternal={exact ? onCallExternal : undefined} onSelectDecode={onSelectDecode} onSetRx={onSetRx} />)
  const row = result.container.querySelector('.decode-row[data-pin-key="immutable-owner-row"]')!
  expect(row).not.toBeNull()
  return { row, onCall, onCallExternal, onSelectDecode, onSetRx }
}
describe('external Work preserves native decode identity', () => {
  it('passes only the immutable producer row ID when an exact coordinator is present', () => {
    const test = setup(); fireEvent.doubleClick(test.row)
    expect(test.onCallExternal).toHaveBeenCalledExactlyOnceWith(entry.id)
    expect(test.onCall).not.toHaveBeenCalled()
  })
  it('preserves the existing call signature for other hosts', () => {
    const test = setup(false); fireEvent.doubleClick(test.row)
    expect(test.onCall).toHaveBeenCalledExactlyOnceWith('K1ABC', undefined, entry.message, -10, 1200)
    expect(test.onCallExternal).not.toHaveBeenCalled()
  })
  it('keeps Ctrl selection and RX marking separate from a transmit gesture', () => {
    const test = setup(); fireEvent.doubleClick(test.row, { ctrlKey: true })
    expect(test.onSelectDecode).toHaveBeenCalledOnce(); expect(test.onSetRx).toHaveBeenCalledExactlyOnceWith(1200)
    expect(test.onCallExternal).not.toHaveBeenCalled(); expect(test.onCall).not.toHaveBeenCalled()
  })
})
