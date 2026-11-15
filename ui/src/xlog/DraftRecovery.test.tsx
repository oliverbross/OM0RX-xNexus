// @vitest-environment jsdom
import { fireEvent, render, screen, waitFor } from '@testing-library/react'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { QsoPanel } from './QsoPanel'
import { bridge, type Bridge, type OutboxEntry } from './api'
import { DRAFT_KEY, newSavedDraft, saveDraft, saveOwnerEditorDraft } from './draftStore'
import { ownerEditFields } from './ownerEditFields'
import producer from './fixtures/contact-detail-producer.json'

const api = (): Bridge => ({ ...bridge, outbox: vi.fn(async () => []), enqueue: vi.fn(async () => []) })
beforeEach(() => window.localStorage.clear())
describe('recoverable, workspace-bound QSO drafts', () => {
  it.each(['owner','unknown'] as const)('stale Clear preserves a newly occupied %s record and does not dispatch',kind=>{
    const native=api();render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    if(kind==='owner'){
      const base=producer.found.editability.base
      saveOwnerEditorDraft({version:2,kind:'owner_contact_edit',request_id:'11111111-1111-4111-8111-111111111111',
        workspace_id:'workspace-a',base_revision:1,base,fields:ownerEditFields(base),frozen:{...base}},null)
    }else window.localStorage.setItem(DRAFT_KEY,'{"version":99,"keep":"unknown owner"}')
    const before=window.localStorage.getItem(DRAFT_KEY)
    fireEvent.click(screen.getByRole('button',{name:'Clear draft'}))
    expect(window.localStorage.getItem(DRAFT_KEY)).toBe(before)
    expect(screen.getByRole('alert').textContent).toContain('preserved')
    expect(native.enqueue).not.toHaveBeenCalled()
  })
  it('restores unfinished fields and the same request ID without automatic submission', async () => {
    const native = api()
    const first = render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    fireEvent.change(screen.getByLabelText('Callsign'), { target: { value: 'OM0RX' } })
    fireEvent.change(screen.getByLabelText('Comment'), { target: { value: 'Keep this\nunfinished draft' } })
    const saved = JSON.parse(window.localStorage.getItem(DRAFT_KEY)!)
    first.unmount()
    render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    expect(screen.getByLabelText('Callsign')).toHaveProperty('value', 'OM0RX')
    expect(screen.getByLabelText('Comment')).toHaveProperty('value', 'Keep this\nunfinished draft')
    expect(native.enqueue).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Queue QSO' }))
    await waitFor(() => expect(native.enqueue).toHaveBeenCalledWith({ kind: 'logbook.create', qso: saved.qso }, saved.request_id, 'workspace-a'))
  })
  it('holds an unsent new draft when the active workspace changes', () => {
    const native = api()
    const { rerender } = render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    fireEvent.change(screen.getByLabelText('Callsign'), { target: { value: 'OM0RX' } })
    rerender(<QsoPanel api={native} enabled connected workspace="workspace-b" />)
    expect(screen.getByRole('button', { name: 'Queue QSO' })).toHaveProperty('disabled', true)
    fireEvent.submit(screen.getByRole('button', { name: 'Queue QSO' }).closest('form')!)
    expect(native.enqueue).not.toHaveBeenCalled()
    expect(JSON.parse(window.localStorage.getItem(DRAFT_KEY)!).workspace_id).toBe('workspace-a')
  })
  it('replays an unacknowledged enqueue using the identical immutable operation ID', async () => {
    const firstApi = api(); vi.mocked(firstApi.enqueue).mockReturnValue(new Promise<OutboxEntry[]>(() => {}))
    const first = render(<QsoPanel api={firstApi} enabled connected workspace="workspace-a" />)
    fireEvent.change(screen.getByLabelText('Callsign'), { target: { value: 'om0rx' } })
    fireEvent.click(screen.getByRole('button', { name: 'Queue QSO' }))
    const sent = vi.mocked(firstApi.enqueue).mock.calls[0]
    expect(sent).toBeTruthy()
    first.unmount()
    const resumed = api()
    render(<QsoPanel api={resumed} enabled connected workspace="workspace-a" />)
    expect(resumed.enqueue).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Queue QSO' }))
    await waitFor(() => expect(resumed.enqueue).toHaveBeenCalledWith(...sent))
  })
  it('keeps the in-memory draft and refuses enqueue when browser storage fails', async () => {
    const native = api()
    render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    const storage = vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('Quota') })
    try {
      fireEvent.change(screen.getByLabelText('Callsign'), { target: { value: 'OM0RX' } })
      fireEvent.click(screen.getByRole('button', { name: 'Queue QSO' }))
      expect(native.enqueue).not.toHaveBeenCalled()
      expect(screen.getByLabelText('Callsign')).toHaveProperty('value', 'OM0RX')
      expect(screen.getByRole('alert').textContent).toContain('Cannot save the draft')
    } finally { storage.mockRestore() }
    fireEvent.click(screen.getByRole('button', { name: 'Queue QSO' }))
    await waitFor(() => expect(native.enqueue).toHaveBeenCalledOnce())
  })
  it('preserves invalid saved data until an explicit Clear draft', () => {
    const corrupt = '{"invalid":"preserve-me"}'
    window.localStorage.setItem(DRAFT_KEY,corrupt)
    render(<QsoPanel api={api()} enabled connected workspace="workspace-a" />)
    expect(screen.getByRole('alert').textContent).toContain('needs recovery')
    expect(screen.getByLabelText('Callsign').closest('fieldset')).toHaveProperty('disabled', true)
    expect(window.localStorage.getItem(DRAFT_KEY)).toBe(corrupt)
    fireEvent.click(screen.getByRole('button', { name: 'Clear draft' }))
    expect(screen.getByLabelText('Callsign').closest('fieldset')).toHaveProperty('disabled', false)
    expect(JSON.parse(window.localStorage.getItem(DRAFT_KEY)!).version).toBe(1)
  })
  it('restores an exact edit revision and original UTC precision', async () => {
    const saved = newSavedDraft()
    saved.workspace_id = 'workspace-a'; saved.edit_revision = 12; saved.started = true
    saved.qso = { ...saved.qso, callsign: 'OM0RX', started_at_unix_ms: 1700000000123, comment: 'Existing comment' }
    saveDraft(saved)
    const native = api()
    render(<QsoPanel api={native} enabled connected workspace="workspace-a" />)
    fireEvent.click(screen.getByRole('button', { name: 'Queue QSO correction' }))
    await waitFor(() => expect(native.enqueue).toHaveBeenCalledWith({ kind: 'logbook.update', qso: saved.qso, expected_contact_revision: 12 }, saved.request_id, 'workspace-a'))
  })
})
