// @vitest-environment jsdom
// Hand-authored presentation fixtures; actual Swift serialization replay is a separate gate.
import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { ContactCard, ContactCardSurface } from './ContactCard'
import type { CardDraft } from '../features/contactCard'
import { t } from '../i18n'
import type { DetailField, OwnerContactDetail, OwnerContactFields } from '../features/ownerContactDetail'

const forbidden = vi.hoisted(() => vi.fn(() => { throw new Error('Native dependency used by owner card') }))
const normalization = vi.hoisted(() => vi.fn((_element: HTMLTextAreaElement): string => { throw new Error('Native comment normalization used by owner card') }))
vi.mock('../api', () => ({ qrzLookup: forbidden }))
vi.mock('../typedInput', () => ({ oneLineTyped: normalization }))
vi.mock('../features/callbookAnswers', () => ({ subscribeCallbookAnswers: forbidden, callbookAnswer: forbidden }))
vi.mock('../features/logSource', () => ({ useLogAnswer: forbidden }))
vi.mock('./QsoHistory', () => ({ QsoHistory: forbidden }))
vi.mock('./AwardStatus', () => ({ CardAwardStatus: forbidden }))
vi.mock('./Iota', () => ({ IotaIslandPick: forbidden, IotaReferenceNote: forbidden }))
vi.mock('./CallCard', () => ({ CallbookPhoto: forbidden, qrzPageOf: forbidden }))
vi.mock('./prop/ContactPath', () => ({ ContactPath: forbidden }))
vi.mock('./ui/Dialog', () => ({ Dialog: ({ children }: { children: React.ReactNode }) => <div role="dialog">{children}</div> }))
vi.mock('../focusReturn', () => ({ useFocusReturn: () => () => {} }))
afterEach(() => { cleanup(); forbidden.mockClear(); normalization.mockClear() })

const empty = { state: 'empty', value: null, reason: null } as const
const unknown = { state: 'unavailable', value: null, reason: 'unsupported' } as const
const known = <T,>(value: T): DetailField<T> => ({ state: 'value', value, reason: null })
function texts<K extends string>(keys: readonly K[]): Record<K, DetailField<string>> {
  return Object.fromEntries(keys.map(key => [key, empty])) as Record<K, DetailField<string>>
}
function fixture(): OwnerContactDetail {
  const reference = () => texts(['pota', 'sota', 'wwff', 'iota', 'sig', 'sig_info'] as const)
  const confirmation = () => ({ ...texts(['source', 'evidence_kind', 'sent_status', 'sent_date', 'sent_via', 'received_status', 'received_date', 'received_via'] as const), match_age_seconds: unknown })
  const upload = () => ({ ...texts(['source', 'state'] as const), receipt_revision: unknown, last_attempt_at_unix_ms: unknown, is_current_revision: unknown })
  const fields: OwnerContactFields = {
    contact: { ...texts(['callsign', 'band', 'mode', 'submode', 'stored_time_unit', 'original_start_precision', 'original_end_precision', 'raw_qso_date', 'raw_time_on', 'raw_qso_date_off', 'raw_time_off', 'rst_sent', 'rst_received', 'grid', 'comment', 'notes'] as const),
      callsign: known('K1ABC'), band: known('20m'), mode: known('MFSK'), submode: known('FT8'),
      started_at_unix_ms: known(1_700_000_000_123), ended_at_unix_ms: empty, comment: known('first\nsecond'), notes: known('separate notes') },
    radio: { ...texts(['receive_band', 'tx_power_raw', 'propagation_mode', 'antenna_path', 'satellite_name'] as const), frequency_hz: empty, receive_frequency_hz: empty, tx_power_w: empty },
    station: { ...texts(['name', 'qth', 'state', 'county', 'country', 'continent', 'cq_zone', 'itu_zone'] as const), dxcc: empty },
    own_station: { ...texts(['operator_call', 'callsign', 'grid', 'recorded_rig', 'radio_label', 'station_location'] as const), recorded_rig: unknown, station_profile_id: empty },
    references: { their: { ...reference(), pota: known('US-0001 US-0002'), sota: known('W1/AA-001') }, my: reference(), dok: empty },
    confirmations: { paper: { ...confirmation(), sent_status: known('R'), received_status: unknown }, lotw: confirmation(), eqsl: confirmation(), qrz: confirmation() },
    awards: { credit_granted: known('DXCC'), credit_submitted: empty, authority: unknown },
    uploads: { lotw: upload(), eqsl: upload(), qrz: upload(), clublog: upload() },
    contest: texts(['contest_id', 'stx', 'srx', 'stx_string', 'srx_string'] as const),
  }
  return { schema_version: 1, contact_id: '11111111-1111-4111-8111-111111111111', status: 'found', reason: null,
    logbook_revision: 42, contact_revision: 7, fields,
    editability: { allowed: true, reason: 'allowed', allowed_fields: ['comment'], base: null }, locate: unknown }
}
function props(detail = fixture()) {
  return { external: { contactId: detail.contact_id, scopeKey: 'connection-A', read: { state: 'current' as const, detail } }, onClose: vi.fn() }
}
const input = (label: string) => screen.getByLabelText(label) as HTMLInputElement

it('uses the original form classes and distinct nullable facts without native reads or editable controls', () => {
  render(<ContactCard {...props()} />)
  expect(document.querySelector('form.contact-card .logbook-form-grid')).toBeTruthy()
  expect(input(t('logbook.field.freq.label')).value).toBe('')
  expect(input(t('logbook.field.myRig.label')).value).toBe(t('contactCard.owner.unavailable'))
  expect(input(t('logbook.field.mode.label')).value).toBe('MFSK')
  expect(input(t('logbook.field.qslSent.label')).value).toBe('R')
  expect(input(t('logbook.field.qslCard.label')).value).toBe(t('contactCard.owner.unavailable'))
  expect(screen.queryByRole('checkbox')).toBeNull()
  expect((screen.getByRole('button', { name: t('logbook.form.save') }) as HTMLButtonElement).disabled).toBe(true)
  fireEvent.submit(document.querySelector('form')!)
  expect(forbidden).not.toHaveBeenCalled()
})

it('preserves raw multiple references and multiline comment separately from notes', () => {
  render(<ContactCard {...props()} />)
  expect(input(t('logbook.field.parkTheirs.label')).value).toContain('POTA: US-0001 US-0002')
  expect(input(t('logbook.field.parkTheirs.label')).value).toContain('SOTA: W1/AA-001')
  expect((screen.getByLabelText(t('logbook.field.comment.label')) as HTMLTextAreaElement).value).toBe('first\nsecond')
  expect((screen.getByLabelText(t('logbook.field.notes.label')) as HTMLTextAreaElement).value).toBe('separate notes')
  expect(forbidden).not.toHaveBeenCalled()
})

it('keeps owner multiline text inert through Enter, paste and change without native normalization or form submission', () => {
  const p = props()
  const { rerender } = render(<ContactCard {...p} />)
  const textarea = screen.getByLabelText(t('logbook.field.comment.label')) as HTMLTextAreaElement
  const requestSubmit = vi.spyOn(textarea.form!, 'requestSubmit')
  const submit = vi.fn()
  textarea.form!.addEventListener('submit', submit)
  fireEvent.keyDown(textarea, { key: 'Enter' })
  expect(textarea.value).toBe('first\nsecond')
  fireEvent.paste(textarea, { clipboardData: { getData: () => 'pasted\nlines' } })
  fireEvent.change(textarea, { target: { value: 'pasted\nlines' } })
  // A synthetic DOM event cannot make a read-only projection into an edit.
  rerender(<ContactCard {...p} />)
  expect(textarea.value).toBe('first\nsecond')
  expect(requestSubmit).not.toHaveBeenCalled()
  expect(submit).not.toHaveBeenCalled()
  expect(forbidden).not.toHaveBeenCalled()
  expect(normalization).not.toHaveBeenCalled()
  requestSubmit.mockRestore()
})

it('keeps the shared surface native default normalization and Enter-submit positive control', () => {
  const draft: CardDraft = { call: 'K1ABC', date: '', timeOn: '', timeOff: '', band: '', freq: '', mode: '',
    rstSent: '', rstRcvd: '', txPower: '', name: '', qth: '', grid: '', state: '', theirProgram: '', theirRef: '',
    myProgram: '', myRef: '', iota: '', myGrid: '', myRig: '', qslSentVia: '', qslCard: '', comment: 'original', notes: '' }
  const set = vi.fn()
  normalization.mockImplementationOnce((element) => element.value.replace(/\n/g, ' '))
  render(<ContactCardSurface call="K1ABC" draft={draft} fields={[]} err={null} busy={false} dirty={false}
    canSave notResent={[]} facts={[]} box={() => null} program={() => null} set={set} setField={() => {}}
    setFields={() => {}} leave={async (then) => then()} onClose={() => {}} onSave={() => {}} returnFocus={() => {}}
    header={null} iota={null} confirmation={null} path={null} awards={null} recent={null} history={null} />)
  const textarea = screen.getByLabelText(t('logbook.field.comment.label')) as HTMLTextAreaElement
  const requestSubmit = vi.spyOn(textarea.form!, 'requestSubmit').mockImplementation(() => {})
  fireEvent.change(textarea, { target: { value: 'native\nline' } })
  expect(normalization).toHaveBeenCalledTimes(1)
  expect(set).toHaveBeenCalledWith('comment', 'native line')
  fireEvent.keyDown(textarea, { key: 'Enter' })
  expect(requestSubmit).toHaveBeenCalledTimes(1)
  requestSubmit.mockRestore()
})

it.each([Number.NaN, Number.POSITIVE_INFINITY, Number.NEGATIVE_INFINITY, 8_640_000_000_000_001, -8_640_000_000_000_001])('withholds unsupported timestamp %s instead of throwing or inventing a date', (ms) => {
  const d = fixture()
  d.fields!.contact.started_at_unix_ms = known(ms)
  d.fields!.contact.ended_at_unix_ms = known(ms)
  render(<ContactCard {...props(d)} />)
  for (const label of [t('logbook.field.date.label'), t('logbook.field.time.label'), t('logbook.field.end.label')]) {
    expect(input(label).value).toBe(t('contactCard.owner.unavailable'))
  }
  expect(forbidden).not.toHaveBeenCalled()
})

it('retires expired facts without remounting the card or treating a new revision as new identity', () => {
  const p = props()
  const { rerender } = render(<ContactCard {...p} />)
  const element = input(t('logbook.field.call.label'))
  element.focus()
  rerender(<ContactCard external={{ ...p.external, read: { state: 'unavailable', message: 'Lease expired' } }} onClose={p.onClose} />)
  expect(input(t('logbook.field.call.label'))).toBe(element)
  expect(element.value).toBe(t('contactCard.owner.unavailable'))
  expect(document.activeElement).toBe(element)
  const next = fixture(); next.contact_revision = 8
  rerender(<ContactCard {...props(next)} />)
  expect(input(t('logbook.field.call.label'))).toBe(element)
  expect(element.value).toBe('K1ABC')
  expect(forbidden).not.toHaveBeenCalled()
})

it.each(['deleted', 'not_found', 'unavailable'] as const)('never shows retained fields for %s', (status) => {
  const d = fixture(); d.status = status
  render(<ContactCard {...props(d)} />)
  expect(input(t('logbook.field.call.label')).value).toBe(t('contactCard.owner.unavailable'))
  expect(screen.queryByText('K1ABC')).toBeNull()
  expect(forbidden).not.toHaveBeenCalled()
})

it('refuses details for another selected contact and ordinary Close is the only dispatch', () => {
  const p = props(); p.external.contactId = '22222222-2222-4222-8222-222222222222'
  render(<ContactCard {...p} />)
  expect(screen.getByRole('status').textContent).toBe(t('contactCard.owner.mismatch'))
  fireEvent.click(screen.getByRole('button', { name: t('contactCard.cancel') }))
  expect(p.onClose).toHaveBeenCalledTimes(1)
  expect(forbidden).not.toHaveBeenCalled()
})
