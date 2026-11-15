// @vitest-environment jsdom
// Actual Swift/SQLite serializer corpus, SHA B53A2BE6…6443043F.
// Projector presentation replay only; no live transport/lease is invented.
import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, render, screen } from '@testing-library/react'
import { ContactCard } from './ContactCard'
import { t } from '../i18n'
import type { OwnerContactDetail } from '../features/ownerContactDetail'
import producer from '../xlog/fixtures/contact-detail-producer.json'

const forbidden = vi.hoisted(() => vi.fn(() => { throw new Error('Native service during owner producer replay') }))
vi.mock('../api', () => ({ qrzLookup: forbidden }))
vi.mock('../features/callbookAnswers', () => ({ subscribeCallbookAnswers: forbidden, callbookAnswer: forbidden }))
vi.mock('../features/logSource', () => ({ useLogAnswer: forbidden }))
vi.mock('./QsoHistory', () => ({ QsoHistory: forbidden }))
vi.mock('./AwardStatus', () => ({ CardAwardStatus: forbidden }))
vi.mock('./Iota', () => ({ IotaIslandPick: forbidden, IotaReferenceNote: forbidden }))
vi.mock('./CallCard', () => ({ CallbookPhoto: forbidden, qrzPageOf: forbidden }))
vi.mock('./prop/ContactPath', () => ({ ContactPath: forbidden }))
vi.mock('./ui/Dialog', () => ({ Dialog: ({ children }: { children: React.ReactNode }) => <div role="dialog">{children}</div> }))
vi.mock('../focusReturn', () => ({ useFocusReturn: () => () => {} }))
afterEach(() => { cleanup(); forbidden.mockClear() })

// This is a raw projector corpus. No authenticated live envelope or read lease
// is invented; current here is solely the component's presentation-test input.
const corpus = producer as { found: OwnerContactDetail; deleted: OwnerContactDetail; not_found: OwnerContactDetail }
function show(detail: OwnerContactDetail) {
  return render(<ContactCard external={{ contactId: detail.contact_id, scopeKey: 'producer-replay-only', read: { state: 'current', detail } }} onClose={() => {}} />)
}

it('replays exact corrected owner fields without substituting native contact semantics', () => {
  const detail = corpus.found
  expect(detail.status).toBe('found')
  expect(detail.contact_revision).toBe(1)
  expect(detail.editability.allowed).toBe(true)
  expect(detail.editability.base?.contact_id).toBe('ABCDEF12-3456-4ABC-8DEF-123456ABCDEF')
  expect(Object.prototype.hasOwnProperty.call(detail.editability.base, 'frequency_hz')).toBe(true)
  expect(detail.editability.base?.frequency_hz).toBeNull()
  expect(detail.contact_id).toBe('abcdef12-3456-4abc-8def-123456abcdef')
  expect(detail.fields?.contact.mode.value).toBe('MFSK')
  expect(detail.fields?.contact.submode.value).toBe('FT8')
  expect(detail.fields?.contact.comment.value).toBe('line one\nline two')
  expect(detail.fields?.confirmations.paper.received_status.value).toBe('Y')
  expect(detail.fields?.confirmations.paper.sent_via.value).toBe('D')
  expect(detail.fields?.radio.frequency_hz).toEqual({ state: 'empty', value: null, reason: null })
  show(detail)
  expect((screen.getByLabelText(t('logbook.field.mode.label')) as HTMLInputElement).value).toBe('MFSK')
  expect((screen.getByLabelText(t('logbook.field.comment.label')) as HTMLTextAreaElement).value).toBe('line one\nline two')
  expect((screen.getByLabelText(t('logbook.field.freq.label')) as HTMLInputElement).value).toBe('')
  expect(screen.queryByRole('checkbox')).toBeNull()
  expect((screen.getByRole('button', { name: t('logbook.form.save') }) as HTMLButtonElement).disabled).toBe(true)
  expect(forbidden).not.toHaveBeenCalled()
})

it.each(['deleted', 'not_found'] as const)('replays actual %s without restoring contact fields or native reads', (state) => {
  const detail = corpus[state]
  expect(detail.status).toBe(state)
  expect(detail.fields).toBeNull()
  expect(detail.editability.base).toBeNull()
  show(detail)
  expect((screen.getByLabelText(t('logbook.field.call.label')) as HTMLInputElement).value).toBe(t('contactCard.owner.unavailable'))
  expect(forbidden).not.toHaveBeenCalled()
})
