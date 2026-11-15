// @vitest-environment jsdom
// Actual host, navigation and SpotsPanel. Unrelated asynchronous services and
// heavyweight cockpit rendering are controlled here; browser geometry is separate.
import { useState, type ReactNode } from 'react'
import { cleanup, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { OriginalHost } from './OriginalHost'
import { rxOff } from './rxApi'
import type { View } from './api'

const control = vi.hoisted(() => ({ receive: vi.fn(), suspend: vi.fn(), command: vi.fn(), connection: vi.fn() }))
const view: View = {
  paired: true, connected: true, connection_token: 'connection-1', endpoint: null,
  hello: { application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'server', workspace_id: 'workspace', capabilities: ['audio.rx', 'cluster.read', 'rotor.stop'] },
  snapshot: { protocol_version: 1, server_instance: 'server', workspace_id: 'workspace', revision: 3,
    generated_at_unix_ms: 10_000, max_age_ms: 3000,
    station: { connected: false, radio_id: null, frequency_hz: null, mode: null, transmitting: null },
    rotor: { connected: true, rotor_id: 'exact-owner-route', azimuth_deg: 171, moving: null },
    spots: [{ id: 'cluster-1', callsign: 'K1ABC', frequency_hz: 14_074_123, mode: 'FT8',
      source: 'xLog cluster', observed_at_unix_ms: 9500, need: { status: 'needed', score: 71, reasons: ['New Call'] } }] },
}
const lease = { snapshot: view.snapshot!, deadline: 1000 }
vi.mock('./useOwnerConnection', () => ({ useOwnerConnection: () => ({ view, lease, session: 1, now: 100,
  busy: false, pollError: '', actionError: '', receipt: null, command: control.command, connection: control.connection,
  isCurrentConnection: () => true, isCurrentReadConnection: () => false }) }))
vi.mock('./useOwnerReceiver', () => ({ useOwnerReceiver: () => ({ view: { ...rxOff, enabled: true }, pending: false,
  observation: 1, leases: new Map(), mode: 'FT8', now: 100, receiving: control.receive,
  selectMode: vi.fn(), pollError: '', actionError: '' }) }))
vi.mock('./useOwnerSpectrum', () => ({ useOwnerSpectrum: () => ({ getSpectrumRow: async () => null,
  resetKey: 'spectrum-1', suspend: control.suspend }) }))
vi.mock('./useOwnerHistory', () => ({ useOwnerHistory: () => ({ recent: null, recall: null, refresh: vi.fn(), canOpenContact: () => false }) }))
vi.mock('../components/TopBar', () => ({ TopBar: ({ nowBar }: { nowBar: ReactNode }) => <header>{nowBar}</header> }))
vi.mock('../components/StationList', () => ({ StationList: () => null }))
vi.mock('../components/OperateCockpit', () => ({ OperateCockpit: ({ active, external }: { active: boolean; external: { spectrumResetKey: string; rotorInGlobalHeader?: boolean } }) => {
  const [draft, setDraft] = useState('')
  return <section data-testid="retained-cockpit" data-active={String(active)} data-spectrum-reset={external.spectrumResetKey} data-global-rotor={String(external.rotorInGlobalHeader)}>
    <input aria-label="Preserved cockpit draft" value={draft} onChange={event => setDraft(event.target.value)} />
  </section>
} }))
vi.mock('../api', async original => ({ ...await original<typeof import('../api')>(),
  getDxccEntityLocations: vi.fn(async () => { throw new Error('Independent centroid read forbidden') }),
  setPounceSpotterFilter: vi.fn(async () => { throw new Error('Independent feed mutation forbidden') }),
}))
beforeEach(() => { localStorage.clear(); sessionStorage.clear(); vi.clearAllMocks(); control.command.mockResolvedValue(undefined); vi.spyOn(performance, 'now').mockReturnValue(100) })
afterEach(() => { cleanup(); vi.restoreAllMocks() })

describe('original host Spots navigation', () => {
  it('keeps the cockpit mounted and its draft, while global Stop remains reachable on Spots', () => {
    const { container } = render(<OriginalHost />)
    const cockpit = screen.getByTestId('retained-cockpit'), input = screen.getByLabelText('Preserved cockpit draft')
    fireEvent.change(input, { target: { value: 'W9XYZ K1ABC -10' } })
    const oldReset = cockpit.dataset.spectrumReset
    fireEvent.click(screen.getByText('Spots', { selector: '.mode-label' }).closest('button')!)
    expect(screen.getByRole('table')).toBeTruthy()
    expect(screen.getByText('xLog assessment')).toBeTruthy()
    expect(container.querySelector('.operate-host')?.hasAttribute('hidden')).toBe(true)
    expect(cockpit.dataset.active).toBe('false')
    expect(cockpit.dataset.globalRotor).toBe('true')
    expect(screen.getAllByRole('group', { name: 'Rotator' })).toHaveLength(1)
    expect(screen.getByRole('button', { name: 'Stop rotation NOW' }).closest('[hidden]')).toBeNull()
    expect(cockpit.dataset.spectrumReset).not.toBe(oldReset)
    expect(screen.getByTestId('retained-cockpit')).toBe(cockpit)
    fireEvent.click(screen.getByRole('button', { name: 'K1ABC' }))
    expect(control.command).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Stop rotation NOW' }))
    expect(control.command).toHaveBeenCalledExactlyOnceWith({ kind: 'rotor.stop', rotor_id: 'exact-owner-route' })
    fireEvent.click(screen.getByRole('button', { name: 'Stop receiving' }))
    expect(control.suspend).toHaveBeenCalledOnce()
    expect(control.receive).toHaveBeenCalledExactlyOnceWith(false)
    // The stock FT navigation item returns to the same retained operating view.
    fireEvent.click(screen.getByRole('button', { name: 'FT weak-signal cockpit — FT8 / FT4 (pick the tier in the top bar)' }))
    expect(container.querySelector('.operate-host')?.hasAttribute('hidden')).toBe(false)
    expect(screen.getByTestId('retained-cockpit')).toBe(cockpit)
    expect((screen.getByLabelText('Preserved cockpit draft') as HTMLInputElement).value).toBe('W9XYZ K1ABC -10')
    expect(control.receive).toHaveBeenCalledTimes(1)
    expect(control.connection).not.toHaveBeenCalled()
  })
})
