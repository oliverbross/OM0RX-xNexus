// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import { bandLabelForMhz } from '../band'
import type { Bridge, DeliveryAction, OutboxEntry, QsoDraft, Spot } from './api'
import { HistoryPanel } from './HistoryPanel'
import { explicitlyClearLegacyDraft, loadDraft, newSavedDraft, saveDraft, type SavedDraft } from './draftStore'

const text = (error: unknown) => error instanceof Error ? error.message : String(error)

export function QsoPanel({ api, enabled, connected, workspace, selected, readEnabled = false }: { api: Bridge; enabled: boolean; connected: boolean; workspace: string | null; selected?: Spot; readEnabled?: boolean }) {
  const [initial] = useState(loadDraft)
  const retainedRaw = useRef(initial.raw)
  const [record, setRecord] = useState(initial.value)
  const [storageError, setStorageError] = useState(initial.error)
  const [locked, setLocked] = useState(initial.locked)
  const [queue, setQueue] = useState<OutboxEntry[]>([])
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const mounted = useRef(true)
  const currentWorkspace = useRef(workspace); currentWorkspace.current = workspace
  const draft = record.qso
  const edit = record.edit_revision == null ? null : { workspace: record.workspace_id, revision: record.edit_revision }
  const foreign = record.workspace_id !== null && record.workspace_id !== workspace
  const untouched = record.workspace_id === null && !record.started && !edit
    && draft.callsign === '' && draft.rst_sent === '' && draft.rst_received === '' && draft.grid === '' && draft.comment === ''
    && draft.frequency_hz === null && draft.band === '20m' && draft.mode === 'SSB'
  useEffect(() => { mounted.current = true; return () => { mounted.current = false } }, [])
  function persist(next: SavedDraft, clear = false): boolean {
    if (locked && !clear) return false
    if (!clear) setRecord(next)
    try {
      if (clear) retainedRaw.current = explicitlyClearLegacyDraft(next, retainedRaw.current)
      else { saveDraft(next); retainedRaw.current = JSON.stringify(next) }
      setRecord(next); setStorageError(''); setLocked(false); return true
    }
    catch (error) { setStorageError(text(error)); return false }
  }
  useEffect(() => { let active = true; api.outbox().then(entries => { if (active) setQueue(entries) }).catch(error => { if (active) setMessage(text(error)) }); return () => { active = false } }, [api])
  const field = <K extends keyof QsoDraft>(key: K, value: QsoDraft[K]) => {
    const firstCall = key === 'callsign' && typeof value === 'string' && value.trim() !== '' && !record.started && !edit
    const startTime = firstCall ? Date.now() : draft.started_at_unix_ms
    const meaningful = typeof value === 'string' ? value.trim() !== '' : value !== null
    persist({ ...record, workspace_id: record.workspace_id ?? (meaningful ? workspace : null), started: record.started || firstCall || key === 'started_at_unix_ms', qso: { ...draft, [key]: value, ...(firstCall ? { started_at_unix_ms: startTime } : {}) } })
  }
  function useSelectedStation(spot: Spot) {
    const startTime = record.started ? draft.started_at_unix_ms : Date.now()
    persist({ ...record, workspace_id: record.workspace_id ?? workspace, started: true, qso: { ...draft, callsign: spot.callsign, mode: spot.mode, band: spot.band?.trim() || bandLabelForMhz(spot.frequency_hz / 1e6), frequency_hz: spot.frequency_hz, started_at_unix_ms: startTime } })
  }
  async function enqueue() {
    setBusy(true); setMessage('')
    try {
      const qso = { ...draft, callsign: draft.callsign.trim().toUpperCase(), grid: draft.grid.trim().toUpperCase() }
      if (!workspace || foreign) throw Error('Reconnect to this draft’s xLog workspace')
      // Persist the exact operation and stable ID BEFORE native enqueue. A crash
      // before acknowledgement can only repeat this same immutable operation.
      if (!persist({ ...record, workspace_id: workspace, qso })) return
      const entries = await api.enqueue(edit ? { kind: 'logbook.update', qso, expected_contact_revision: edit.revision } : { kind: 'logbook.create', qso }, record.request_id, workspace)
      if (!mounted.current) return
      setQueue(entries)
      persist(newSavedDraft())
      setMessage('QSO saved in the native submission queue. Select Submit to xLog when ready.')
    } catch (error) { if (mounted.current) setMessage(text(error)) }
    finally { if (mounted.current) setBusy(false) }
  }
  async function deliver(id: string, action: DeliveryAction) {
    const owner = workspace
    setBusy(true); setMessage('')
    try {
      const receipt = await api.deliver(id, action)
      if (mounted.current && currentWorkspace.current === owner) setMessage(`xLog receipt: ${receipt.status}${receipt.message ? ` — ${receipt.message}` : ''}`)
    } catch (error) { if (mounted.current && currentWorkspace.current === owner) setMessage(`${text(error)}. The retained queue entry has not been discarded.`) }
    finally {
      try { const entries = await api.outbox(); if (mounted.current) setQueue(entries) } catch (error) { if (mounted.current) setMessage(text(error)) }
      if (mounted.current) setBusy(false)
    }
  }
  return <section className="xn-qso" aria-label="QSO submission queue">
    <HistoryPanel api={api} callsign={selected?.callsign} workspace={workspace} enabled={readEnabled} canEdit={enabled && !busy && !locked && untouched} onEdit={(contact, owner) => { const { contact_revision, ...qso } = contact; if (contact_revision == null) return; persist({ ...newSavedDraft(), qso, workspace_id: owner, edit_revision: contact_revision, started: true }) }} />
    <div><h2>Log through xLog</h2><p>QSO drafts are queued on this computer. Only an applied xLog receipt confirms a logbook write. Nothing is sent automatically.</p></div>
    {!enabled && <p className="xn-hint">A fresh connection with xLog’s logbook.write capability is required to queue or submit a contact.</p>}
    {storageError && <p role="alert" className="xn-error">{storageError}</p>}
    {foreign && <p role="status">This draft belongs to xLog workspace {record.workspace_id}. Reconnect there or clear the draft explicitly.</p>}
    <form onSubmit={event => { event.preventDefault(); if (enabled && !busy && !locked && !foreign) void enqueue() }}><fieldset className="xn-qso-form" disabled={busy || locked}>
      {edit && <p>Editing contact {draft.contact_id} at xLog revision {edit.revision}. Change callsign, band or mode in xLog to preserve award and confirmation evidence. Clear this draft to start a different contact.</p>}
      {selected && !edit && <button type="button" disabled={foreign} onClick={() => useSelectedStation(selected)}>Use selected station</button>}
      <label>Callsign<input required readOnly={!!edit} maxLength={32} value={draft.callsign} onChange={event => field('callsign', event.target.value)} /></label>
      <label>Band<input required readOnly={!!edit} maxLength={16} value={draft.band} onChange={event => field('band', event.target.value)} /></label>
      <label>Mode<input required readOnly={!!edit} maxLength={16} value={draft.mode} onChange={event => field('mode', event.target.value)} /></label>
      <label>Started UTC<input required type="datetime-local" value={new Date(draft.started_at_unix_ms).toISOString().slice(0,16)} onChange={event => { const value = Date.parse(`${event.target.value}Z`); if (Number.isFinite(value)) field('started_at_unix_ms', value) }} /></label>
      <label>Frequency MHz<input type="number" min="0.000001" step="0.000001" value={draft.frequency_hz == null ? '' : draft.frequency_hz / 1e6} onChange={event => field('frequency_hz', event.target.value ? Math.round(Number(event.target.value) * 1e6) : null)} /></label>
      <label>RST sent<input maxLength={16} value={draft.rst_sent} onChange={event => field('rst_sent', event.target.value)} /></label>
      <label>RST received<input maxLength={16} value={draft.rst_received} onChange={event => field('rst_received', event.target.value)} /></label>
      <label>Grid<input maxLength={12} value={draft.grid} onChange={event => field('grid', event.target.value)} /></label>
      <label className="xn-qso-comment">Comment<textarea maxLength={4096} value={draft.comment} onChange={event => field('comment', event.target.value)} /></label>
      <button type="submit" disabled={!enabled || busy || locked || foreign}>{edit ? 'Queue QSO correction' : 'Queue QSO'}</button>
    </fieldset></form>
    <button type="button" disabled={busy} onClick={() => persist(newSavedDraft(), true)}>Clear draft</button>
    {message && <p role="status" className="xn-result">{message}</p>}
    {queue.length === 0 ? <p>No queued QSO submissions.</p> : <ul className="xn-queue">{queue.map(entry => {
      const { queued, state } = entry
      const matching = queued.workspace_id === workspace
      const receipt = typeof state === 'object' ? state.Settled : null
      return <li key={queued.request_id}><div><strong>{queued.mutation.qso.callsign}</strong> · {queued.mutation.qso.band} · {queued.mutation.qso.mode} · {receipt ? receipt.status : state === 'Pending' ? 'Pending submission' : 'Receipt required'}<small>Workspace {queued.workspace_id} · request {queued.request_id}</small>{receipt?.message && <p>{receipt.message}</p>}</div>
        {!matching && <span>Reconnect to this entry’s xLog workspace.</span>}
        {state === 'Pending' && <button disabled={!enabled || !matching || busy} onClick={() => void deliver(queued.request_id, 'submit')}>Submit to xLog</button>}
        {state === 'NeedsReceipt' && <><button disabled={!connected || !matching || busy} onClick={() => void deliver(queued.request_id, 'reconcile')}>Check receipt</button><button disabled={!enabled || !matching || busy} onClick={() => void deliver(queued.request_id, 'retry_missing')}>Retry only if receipt missing</button></>}
      </li>
    })}</ul>}
  </section>
}
