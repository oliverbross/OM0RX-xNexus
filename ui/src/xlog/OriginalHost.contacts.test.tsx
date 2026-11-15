// @vitest-environment jsdom
// Real host connection/history/contact hooks and original Recent/Recall/Card.
// Unrelated cockpit rendering/RX/device services are inert fixture boundaries.
import type { ReactNode } from 'react'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { OriginalHost } from './OriginalHost'
import { RecentQsos, type ExternalRecentRead } from '../components/RecentQsosPane'
import { RecallPanel, type ExternalRecallRead } from '../components/RecallPanel'
import { rxOff } from './rxApi'
import { disconnected, type Bridge, type ContactDetailEnvelope, type History, type HistoryReadView, type RecentContacts, type View } from './api'
import type { OwnerContactDetail } from '../features/ownerContactDetail'
import producer from './fixtures/contact-detail-producer.json'
import { t } from '../i18n'

const native = vi.hoisted(() => vi.fn(() => { throw Error('Independent native contact service forbidden') }))
vi.mock('../components/TopBar', () => ({ TopBar: ({ nowBar }: { nowBar: ReactNode }) => <header>{nowBar}</header> }))
vi.mock('../components/RotorStrip', () => ({ RotorStrip: () => null }))
vi.mock('../components/StationList', () => ({ StationList: () => null }))
vi.mock('../components/SpotsPanel', () => ({ SpotsPanel: () => null }))
vi.mock('./useOwnerReceiver', () => ({ useOwnerReceiver: () => ({ view: rxOff, mode: 'FT8', now: 100,
  pending: false, observation: 0, leases: new Map(), receiving: native, selectMode: native, pollError: '', actionError: '' }) }))
vi.mock('./useOwnerSpectrum', () => ({ useOwnerSpectrum: () => ({ getSpectrumRow: async () => null, resetKey: 'none', suspend: native }) }))
vi.mock('../components/OperateCockpit', () => ({ OperateCockpit: (props: {
  external: { recent: ExternalRecentRead; history: ExternalRecallRead };
  onOpenLogbook: (call: string, id?: string | null) => void; onSelect: (call: string) => void;
}) => <>
  <button onClick={() => props.onSelect('K1ABC')}>Select recall</button>
  <button onClick={() => props.onOpenLogbook('K1ABC')}>Heading without ID</button>
  <RecentQsos external={props.external.recent} onOpen={props.onOpenLogbook} />
  <RecallPanel call="K1ABC" hist={null} externalHistory={props.external.history} onOpenLog={props.onOpenLogbook} />
</> }))
vi.mock('../api', async original => ({ ...await original<typeof import('../api')>(), qrzLookup: native, getCallStats: native,
  getDxccEntityLocations: native, getCallBearings: native }))
const A = producer.found.contact_id, B = '00000000-0000-4000-8000-000000000999'
const workspace = '00000000-0000-4000-8000-000000000100'
const row = (id: string, call: string, at: number) => ({ contact_id: id, contact_revision: null, callsign: call,
  band: '20m', mode: 'MFSK', exact_mode: 'FT8', submode: 'FT8', details_complete: false,
  started_at_unix_ms: at, frequency_hz: null, rst_sent: '-10', rst_received: '-11', grid: '', comment: '' })
function setup() {
  let stamp = 1000
  const view: View = { ...disconnected, paired: true, connected: true, connection_token: 'token', hello: {
    application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'server', workspace_id: workspace,
    capabilities: ['logbook.read', 'logbook.contact.read', 'logbook.write'] } }
  const wire = <T,>(payload: T): HistoryReadView<T> => ({ payload, connection_token: 'token', remaining_valid_ms: 5000 })
  const common = () => ({ protocol_version: 1 as const, server_instance: 'server', workspace_id: workspace,
    generation: workspace, logbook_revision: 1, generated_at_unix_ms: ++stamp, max_age_ms: 5000 })
  const contacts = [row(A, 'W1AAA', 10000), row(B, 'K1ABC/P', 9000)]
  const historyTransport = {
    recent: vi.fn(async () => wire<RecentContacts>({ ...common(), schema_version: 1, contacts, limit: 5,
      total: 2, complete: true, ordering: 'qso_start_desc_contact_id_asc' })),
    history: vi.fn(async (_token: string, call: string) => wire<History>({ ...common(), callsign: call,
      contacts, matrix: [], contacts_limit: 25, contacts_complete: true, matrix_complete: true,
      history_summary: { schema_version: 1, scope: 'normal_log_base_callsign', complete: true, total: 2,
        worked_before: true, confirmed_count: 0, confirmation_policy: 'xlog_cluster_paper_or_lotw_v1',
        confirmation_sources: ['paper_qsl', 'lotw'], last_started_at_unix_ms: 10000,
        bands: ['20m'], modes: ['FT8'], bands_complete: true, modes_complete: true,
        requested_band: null, requested_exact_mode: null, requested_band_worked: null, requested_band_exact_mode_worked: null } })),
  }
  const contactTransport = { read: vi.fn(async (_token: string, id: string) => {
    const detail = JSON.parse(JSON.stringify(producer.found)) as OwnerContactDetail
    detail.contact_id = id; detail.editability.base!.contact_id = id.toUpperCase()
    return wire<ContactDetailEnvelope>({ protocol_version: 1, server_instance: 'server', workspace_id: workspace,
      generation: workspace, worker_generation: 1, generated_at_unix_ms: ++stamp, max_age_ms: 5000, detail })
  }) }
  const api: Bridge = { status: vi.fn(async () => view), poll: vi.fn(async () => view), importPairing: native, connect: native,
    disconnect: native, command: native, enqueue: native, outbox: native, deliver: native, history: native }
  return { api, historyTransport, contactTransport }
}
beforeEach(() => {
  localStorage.clear(); native.mockClear()
  // Gesture handoff is the scope here; separate hook tests advance real cadence
  // and lease clocks. Avoid unrelated periodic renders while async act settles.
  vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout', 'setInterval', 'clearInterval'] })
  vi.spyOn(performance, 'now').mockReturnValue(100)
})

it('the original owner footer reconnects above a dirty card, retaining its fields and request until explicit Refresh',async()=>{
  const s=setup();s.api.connect=vi.fn(async()=>await s.api.status())
  render(<OriginalHost {...s}/>);await flushImmediate()
  await act(async()=>fireEvent.click(screen.getByRole('button',{name:t('logbook.row.edit',{call:'K1ABC/P'})})))
  await flushImmediate()
  const comment=screen.getByLabelText(t('logbook.field.comment.label')) as HTMLTextAreaElement
  fireEvent.change(comment,{target:{value:'dirty before disconnect\nretained'}})
  const raw=localStorage.getItem('om0rx-xnexus.qso-draft.v1')
  vi.mocked(s.api.poll).mockRejectedValueOnce(Error('Synthetic connection lost'))
  await act(async()=>{await vi.advanceTimersByTimeAsync(500)})
  expect(comment.readOnly).toBe(true)
  const reads=s.contactTransport.read.mock.calls.length
  await act(async()=>fireEvent.click(screen.getByRole('button',{name:t('contactCard.editor.connection')})))
  expect(screen.getByRole('dialog',{name:'xLog connection'})).toBeTruthy()
  expect(s.api.connect).not.toHaveBeenCalled()
  await act(async()=>fireEvent.click(screen.getByRole('button',{name:'Reconnect'})));await flushImmediate()
  expect(s.api.connect).toHaveBeenCalledTimes(1)
  expect(s.contactTransport.read).toHaveBeenCalledTimes(reads)
  await act(async()=>fireEvent.click(screen.getByRole('button',{name:'Close'})));await flushImmediate()
  expect(screen.getByLabelText(t('logbook.field.comment.label'))).toBe(comment)
  expect(comment.value).toBe('dirty before disconnect\nretained')
  expect(localStorage.getItem('om0rx-xnexus.qso-draft.v1')).toBe(raw)
  expect((screen.getByRole('button',{name:t('contactCard.editor.queue')}) as HTMLButtonElement).disabled).toBe(true)
  await act(async()=>fireEvent.click(screen.getByRole('button',{name:t('contactCard.editor.refresh')})));await flushImmediate()
  expect((screen.getByRole('button',{name:t('contactCard.editor.queue')}) as HTMLButtonElement).disabled).toBe(false)
  expect(s.contactTransport.read).toHaveBeenCalledTimes(reads+1)
  expect(s.api.enqueue).not.toHaveBeenCalled();expect(s.api.deliver).not.toHaveBeenCalled();expect(native).not.toHaveBeenCalled()
})
afterEach(() => { cleanup(); vi.restoreAllMocks(); vi.useRealTimers() })
const flushImmediate = () => act(async () => { await vi.advanceTimersByTimeAsync(0) })
function phase(label: string, s: ReturnType<typeof setup>) {
  console.info('CONTACT_HOST_PHASE', JSON.stringify({ label, utc: new Date().toISOString(),
    history: s.historyTransport.history.mock.calls.length, recent: s.historyTransport.recent.mock.calls.length,
    detail: s.contactTransport.read.mock.calls.length, native: native.mock.calls.length,
    card: !!document.querySelector('form.contact-card') }))
}

it('the actual second Recent row opens its UUID in the original ContactCard without any write/native fallback', async () => {
  const s = setup(); render(<OriginalHost {...s} />)
  phase('recent-mounted', s); await flushImmediate(); phase('recent-ready', s)
  expect(s.contactTransport.read).not.toHaveBeenCalled()
  await act(async () => fireEvent.click(screen.getByRole('button', { name: t('logbook.row.edit', { call: 'K1ABC/P' }) })))
  await flushImmediate(); phase('recent-opened', s)
  expect(s.contactTransport.read.mock.calls).toEqual([['token', B]])
  expect(document.querySelector('form.contact-card .contact-card-actions')).toBeTruthy()
  expect(native).not.toHaveBeenCalled()
})
it('the actual Recall row uses the queried call for admission and its own ID; no-ID gestures never search', async () => {
  const s = setup(); const shown = render(<OriginalHost {...s} />)
  phase('recall-mounted', s); await flushImmediate(); phase('recall-ready', s)
  await act(async () => fireEvent.click(screen.getByRole('button', { name: 'Heading without ID' })))
  await flushImmediate(); phase('no-id-refused', s)
  expect(s.contactTransport.read).not.toHaveBeenCalled()
  await act(async () => fireEvent.click(screen.getByRole('button', { name: 'Select recall' })))
  await flushImmediate(); phase('recall-selected', s)
  const rows = shown.container.querySelectorAll<HTMLElement>('.recall-log-row')
  expect(rows).toHaveLength(2)
  await act(async () => fireEvent.click(rows[1]))
  await flushImmediate(); phase('recall-opened', s)
  expect(s.contactTransport.read.mock.calls).toEqual([['token', B]])
  expect(native).not.toHaveBeenCalled()
})
