// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import type { Bridge, History, HistoryContact } from './api'

export function HistoryPanel({ api, callsign, workspace, enabled, canEdit, onEdit }: { api: Bridge; callsign?: string; workspace: string | null; enabled: boolean; canEdit: boolean; onEdit: (contact: HistoryContact, workspace: string) => void }) {
  const [history, setHistory] = useState<History | null>(null)
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const generation = useRef(0)
  useEffect(() => { generation.current++; setHistory(null); setError(''); setBusy(false) }, [callsign, workspace, enabled])
  async function load() {
    if (!callsign || !enabled) return
    const epoch = ++generation.current
    setBusy(true); setError('')
    try {
      const response = await api.history(callsign)
      if (epoch !== generation.current) return
      if (response.workspace_id !== workspace || response.callsign !== callsign.toUpperCase()) throw Error('xLog history identity changed; request it again')
      setHistory(response)
    } catch (error) { if (epoch === generation.current) setError(error instanceof Error ? error.message : String(error)) }
    finally { if (epoch === generation.current) setBusy(false) }
  }
  return <section className="xn-history" aria-label="Selected station log history">
    <div className="xn-feed-tools"><h3>{callsign ? `${callsign} in your xLog logbook` : 'Select a station for its log history'}</h3><button disabled={!callsign || !enabled || busy} onClick={() => void load()}>Load history from xLog</button></div>
    {error && <p role="alert">{error}</p>}
    {history && <><p>Read at {new Date(history.generated_at_unix_ms).toISOString()} · {history.contacts.length} recent contacts. Refresh explicitly after a logbook change.</p>
      <div className="xn-history-matrix">{history.matrix.map(cell => <span key={`${cell.band}/${cell.mode}`}>{cell.band} / {cell.mode}: {cell.confirmed ? 'Confirmed' : cell.worked ? 'Worked' : 'Not worked'}</span>)}</div>
      <ul className="xn-queue">{history.contacts.map(contact => <li key={contact.contact_id}><div><strong>{contact.callsign}</strong> · {contact.band} · {contact.mode}<small>{new Date(contact.started_at_unix_ms).toISOString()} · {contact.rst_sent} / {contact.rst_received} · {contact.grid}</small><p>{contact.comment}</p></div><button disabled={!canEdit || !enabled || contact.contact_revision == null} onClick={() => onEdit(contact, history.workspace_id)}>Edit QSO</button>{contact.contact_revision == null && <small>xLog has not supplied an edit revision.</small>}</li>)}</ul>
    </>}
  </section>
}
