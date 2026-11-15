// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { RecentQsos, RECENT_QSOS } from './RecentQsosPane'
import { createAskingLogSource } from '../features/askingLogSource'
import { __resetLogSourceForTests, setLogSource } from '../features/logSource'
import { t } from '../i18n'

const nativeAsk = vi.fn(async () => { throw new Error('Independent log must not be read') })
beforeEach(() => { nativeAsk.mockClear(); setLogSource(createAskingLogSource(nativeAsk)) })
afterEach(() => { cleanup(); __resetLogSourceForTests() })
const row = (id: string, call: string, canOpen: boolean) => ({ id, call, canOpen, band: '20m', mode: 'FT8', whenUnix: 1_700_000_000 })

it('renders actual narrow owner rows in the existing five-line pane without querying the native log', async () => {
  const onOpen = vi.fn()
  render(<RecentQsos external={{ state: 'current', total: 2, rows: [row('a', 'K1ABC', true), row('b', 'K2ABC', false)] }} onOpen={onOpen} />)
  expect([...document.querySelectorAll('.recent-qso-call')].map(node => node.textContent)).toEqual(['K1ABC', 'K2ABC'])
  expect(screen.getAllByRole('listitem')).toHaveLength(RECENT_QSOS)
  expect(screen.getAllByRole('button')).toHaveLength(1)
  fireEvent.click(screen.getByRole('button', { name: t('logbook.row.edit', { call: 'K1ABC' }) }))
  expect(onOpen).toHaveBeenCalledWith('K1ABC', 'a')
  await Promise.resolve()
  expect(nativeAsk).not.toHaveBeenCalled()
})

it('distinguishes owner loading and unavailable from an authoritative empty log without changing row count', async () => {
  const { rerender } = render(<RecentQsos external={{ state: 'loading', message: 'Reading xLog contacts…' }} />)
  expect(screen.getByText('Reading xLog contacts…')).toBeTruthy()
  expect(screen.queryByText(t('recentQsos.empty'))).toBeNull()
  expect(screen.getAllByRole('listitem')).toHaveLength(RECENT_QSOS)
  rerender(<RecentQsos external={{ state: 'unavailable', message: 'xLog contact history unavailable' }} />)
  expect(screen.getByText('xLog contact history unavailable')).toBeTruthy()
  expect(screen.queryByText(t('recentQsos.empty'))).toBeNull()
  rerender(<RecentQsos external={{ state: 'current', total: 0, rows: [] }} />)
  expect(screen.getByText(t('recentQsos.empty'))).toBeTruthy()
  expect(screen.getAllByRole('listitem')).toHaveLength(RECENT_QSOS)
  await Promise.resolve()
  expect(nativeAsk).not.toHaveBeenCalled()
})

it('retires displayed owner rows and edit controls immediately when their source becomes unavailable', () => {
  const onOpen = vi.fn()
  const { rerender } = render(<RecentQsos external={{ state: 'current', total: 1, rows: [row('a', 'K1ABC', true)] }} onOpen={onOpen} />)
  expect(screen.getAllByRole('button')).toHaveLength(1)
  rerender(<RecentQsos external={{ state: 'unavailable', message: 'Connection changed' }} onOpen={onOpen} />)
  expect(document.querySelectorAll('.recent-qso-call')).toHaveLength(0)
  expect(screen.queryAllByRole('button')).toHaveLength(0)
  expect(screen.getByText('Connection changed')).toBeTruthy()
  expect(onOpen).not.toHaveBeenCalled()
})

it('does not use an independent-log response that arrives after switching to an external owner', async () => {
  let settle!: (value: unknown) => void
  const pending = new Promise(resolve => { settle = resolve })
  setLogSource(createAskingLogSource(async () => pending as never))
  const { rerender } = render(<RecentQsos />)
  rerender(<RecentQsos external={{ state: 'current', total: 1, rows: [row('owner-id', 'OM0RX', false)] }} />)
  settle({ rows: [{ id: 'native-id', call: 'W1AW', band: '40m', mode: 'CW', whenUnix: 1_700_000_000 }] })
  await Promise.resolve()
  expect([...document.querySelectorAll('.recent-qso-call')].map(node => node.textContent)).toEqual(['OM0RX'])
  expect(screen.queryAllByRole('button')).toHaveLength(0)
})
