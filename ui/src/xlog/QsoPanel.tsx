// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import { bandLabelForMhz } from '../band'
import type { Bridge, DeliveryAction, OutboxEntry, QsoDraft, Spot } from './api'
import { HistoryPanel } from './HistoryPanel'

const text = (error: unknown) => error instanceof Error ? error.message : String(error)
const newDraft = (): QsoDraft => ({ contact_id: crypto.randomUUID(), callsign: '', band: '20m', mode: 'SSB', started_at_unix_ms: Date.now(), frequency_hz: null, rst_sent: '', rst_received: '', grid: '', comment: '' })

export function QsoPanel({ api, enabled, connected, workspace, selected, readEnabled = false }: { api: Bridge; enabled: boolean; connected: boolean; workspace: string | null; selected?: Spot; readEnabled?: boolean }) {
  const [draft, setDraft] = useState(newDraft)
  const [queue, setQueue] = useState<OutboxEntry[]>([])
  const [message, setMessage] = useState('')
  const [busy, setBusy] = useState(false)
  const [edit, setEdit] = useState<{ workspace: string; revision: number } | null>(null)
  const request = useRef(crypto.randomUUID())
  useEffect(() => { let active = true; api.outbox().then(entries => { if (active) setQueue(entries) }).catch(error => { if (active) setMessage(text(error)) }); return () => { active = false } }, [api])
  const field = <K extends keyof QsoDraft>(key: K, value: QsoDraft[K]) => setDraft(old => ({ ...old, [key]: value }))
  async function enqueue() {
    setBusy(true); setMessage('')
    try {
      const qso = { ...draft, callsign: draft.callsign.trim().toUpperCase(), grid: draft.grid.trim().toUpperCase() }
      if (edit && edit.workspace !== workspace) throw Error('Reconnect to the edited QSO’s xLog workspace')
      setQueue(await api.enqueue(edit ? { kind: 'logbook.update', qso, expected_contact_revision: edit.revision } : { kind: 'logbook.create', qso }, request.current))
      setDraft(newDraft()); request.current = crypto.randomUUID()
      setEdit(null)
      setMessage('QSO saved in the native submission queue. Select Submit to xLog when ready.')
    } catch (error) { setMessage(text(error)) }
    finally { setBusy(false) }
  }
  async function deliver(id: string, action: DeliveryAction) {
    setBusy(true); setMessage('')
    try {
      const receipt = await api.deliver(id, action)
      setMessage(`xLog receipt: ${receipt.status}${receipt.message ? ` — ${receipt.message}` : ''}`)
    } catch (error) { setMessage(`${text(error)}. The retained queue entry has not been discarded.`) }
    finally {
      try { setQueue(await api.outbox()) } catch (error) { setMessage(text(error)) }
      setBusy(false)
    }
  }
  return <section className="xn-qso" aria-label="QSO submission queue">
    <HistoryPanel api={api} callsign={selected?.callsign} workspace={workspace} enabled={readEnabled} canEdit={enabled && !busy && draft.callsign.trim() === ''} onEdit={(contact, owner) => { const { contact_revision, ...qso } = contact; if (contact_revision == null) return; setDraft(qso); setEdit({ workspace: owner, revision: contact_revision }); request.current = crypto.randomUUID() }} />
    <div><h2>Log through xLog</h2><p>QSO drafts are queued on this computer. Only an applied xLog receipt confirms a logbook write. Nothing is sent automatically.</p></div>
    {!enabled && <p className="xn-hint">A fresh connection with xLog’s logbook.write capability is required to queue or submit a contact.</p>}
    <form className="xn-qso-form" onSubmit={event => { event.preventDefault(); if (enabled && !busy) void enqueue() }}>
      {edit && <p>Editing contact {draft.contact_id} at xLog revision {edit.revision}. Clear this draft to start a different contact.</p>}
      {selected && !edit && <button type="button" onClick={() => setDraft(old => ({ ...old, callsign: selected.callsign, mode: selected.mode, band: bandLabelForMhz(selected.frequency_hz / 1e6) || old.band, frequency_hz: selected.frequency_hz }))}>Use selected station</button>}
      <label>Callsign<input required maxLength={32} value={draft.callsign} onChange={event => field('callsign', event.target.value)} /></label>
      <label>Band<input required maxLength={16} value={draft.band} onChange={event => field('band', event.target.value)} /></label>
      <label>Mode<input required maxLength={16} value={draft.mode} onChange={event => field('mode', event.target.value)} /></label>
      <label>Started UTC<input required type="datetime-local" value={new Date(draft.started_at_unix_ms).toISOString().slice(0,16)} onChange={event => { const value = Date.parse(`${event.target.value}Z`); if (Number.isFinite(value)) field('started_at_unix_ms', value) }} /></label>
      <label>Frequency MHz<input type="number" min="0.000001" step="0.000001" value={draft.frequency_hz == null ? '' : draft.frequency_hz / 1e6} onChange={event => field('frequency_hz', event.target.value ? Math.round(Number(event.target.value) * 1e6) : null)} /></label>
      <label>RST sent<input maxLength={16} value={draft.rst_sent} onChange={event => field('rst_sent', event.target.value)} /></label>
      <label>RST received<input maxLength={16} value={draft.rst_received} onChange={event => field('rst_received', event.target.value)} /></label>
      <label>Grid<input maxLength={12} value={draft.grid} onChange={event => field('grid', event.target.value)} /></label>
      <label className="xn-qso-comment">Comment<textarea maxLength={4096} value={draft.comment} onChange={event => field('comment', event.target.value)} /></label>
      <button type="submit" disabled={!enabled || busy || (!!edit && edit.workspace !== workspace)}>{edit ? 'Queue QSO correction' : 'Queue QSO'}</button>
      <button type="button" disabled={busy} onClick={() => { setDraft(newDraft()); setEdit(null); request.current = crypto.randomUUID() }}>Clear draft</button>
    </form>
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
