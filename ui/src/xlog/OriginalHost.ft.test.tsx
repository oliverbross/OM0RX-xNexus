// @vitest-environment jsdom
// Actual host, navigation and SpotsPanel. Unrelated asynchronous services and
// heavyweight cockpit rendering are controlled here; browser geometry is separate.
import { type ReactNode, type ComponentProps } from 'react'
import { cleanup, render } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { OriginalHost } from './OriginalHost'
import { ftOff } from './ftApi'
import type { OperateCockpit } from '../components/OperateCockpit'
import { rxOff } from './rxApi'
import type { View } from './api'

const control = vi.hoisted(() => ({ ftAction: vi.fn(async () => {}), ftLog: vi.fn(async () => {}), fresh: true, mode: 'FT8' as 'FT8' | 'FT4' | null, cockpit: null as unknown, receive: vi.fn(), receiverScope: vi.fn(), suspend: vi.fn(), command: vi.fn(), connection: vi.fn() }))
const view: View = {
  paired: true, connected: true, connection_token: 'connection-1', endpoint: null,
  hello: { application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'server', workspace_id: 'workspace', capabilities: ['audio.rx', 'cluster.read', 'rotor.stop', 'logbook.classify', 'digital.tx', 'logbook.write'] },
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
  isCurrentConnection: () => true, isCurrentReadConnection: () => false,
  isCurrentBearingIdentity: () => false, retirePointObservation: () => {}, poll: null, pointEvidence: null, observedPoint: vi.fn() }) }))
vi.mock('./useOwnerReceiver', () => ({ useOwnerReceiver: (scope: unknown) => (control.receiverScope(scope), ({ view: { ...rxOff, enabled: true }, pending: false,
  observation: 1, leases: new Map(), mode: 'FT8', now: 100, receiving: control.receive,
  selectMode: vi.fn(), pollError: '', actionError: '' })) }))
vi.mock('./useOwnerSpectrum', () => ({ useOwnerSpectrum: () => ({ getSpectrumRow: async () => null,
  resetKey: 'spectrum-1', suspend: control.suspend }) }))
vi.mock('./useOwnerHistory', () => ({ useOwnerHistory: () => ({ recent: null, recall: null, refresh: vi.fn(), canOpenContact: () => false }) }))
vi.mock('../components/TopBar', () => ({ TopBar: ({ nowBar }: { nowBar: ReactNode }) => <header>{nowBar}</header> }))
vi.mock('../components/StationList', () => ({ StationList: () => null }))
vi.mock('../components/OperateCockpit', () => ({ OperateCockpit: (props: ComponentProps<typeof OperateCockpit>) => { control.cockpit = props; return <section data-testid="cockpit" /> } }))
vi.mock('./useOwnerTransmitter', () => ({ useOwnerTransmitter: () => ({
  view: { ...ftOff, connection_token: 'connection-1', available: control.fresh, remaining_valid_ms: control.fresh ? 1000 : 0,
    mode: control.mode, qso_state: 'CallingCq', cq_running: true, operation_id: 'retained-operation', release_pending: !control.fresh },
  fresh: control.fresh, action: control.ftAction, log: control.ftLog, error: '', busy: false, logNotice: '',
}) }))
vi.mock('../api', async original => ({ ...await original<typeof import('../api')>(),
  getDxccEntityLocations: vi.fn(async () => { throw new Error('Independent centroid read forbidden') }),
  setPounceSpotterFilter: vi.fn(async () => { throw new Error('Independent feed mutation forbidden') }),
}))
beforeEach(() => { localStorage.clear(); sessionStorage.clear(); vi.clearAllMocks(); lease.deadline = 1000; control.fresh = true; control.mode = 'FT8'; control.command.mockResolvedValue(undefined); vi.spyOn(performance, 'now').mockReturnValue(100) })
afterEach(() => { cleanup(); vi.restoreAllMocks() })

const cockpit = () => control.cockpit as ComponentProps<typeof OperateCockpit>
describe('original host FT control bindings', () => {
  it('uses fresh TX context while ordinary station snapshot is expired, then retains Stop after TX expiry', () => {
    lease.deadline = 0
    const mounted = render(<OriginalHost />)
    expect(cockpit().services!.actions.has('transmit')).toBe(true)
    expect(cockpit().snap.radio.txAllowed).toBe(true)
    cockpit().onSetTxEnabled!(true)
    expect(control.ftAction).toHaveBeenCalledExactlyOnceWith({ kind: 'enable', enabled: true })
    control.fresh = false
    mounted.rerender(<OriginalHost />)
    expect(cockpit().services!.actions.has('transmit')).toBe(false)
    expect(cockpit().snap.radio.txAllowed).toBe(false)
    expect(cockpit().services!.actions.has('stopTransmit')).toBe(true)
    cockpit().onHaltTx()
    expect(control.ftAction).toHaveBeenLastCalledWith({ kind: 'stop' })
  })
  it('forwards directed CQ and exact native decode identities', async () => {
    render(<OriginalHost />)
    await cockpit().services!.handlers.startRunning!('DX')
    expect(control.ftAction).toHaveBeenLastCalledWith({ kind: 'cq', direction: 'DX' })
    cockpit().external!.onCallDecode!('immutable-decode-id')
    expect(control.ftAction).toHaveBeenLastCalledWith({ kind: 'work', decode_id: 'immutable-decode-id' })
    expect(control.command).not.toHaveBeenCalled()
  })
  it('keeps automatic parity distinct from automatic QSO sequencing', () => {
    render(<OriginalHost />)
    cockpit().onSetTxCycleAuto(true)
    expect(control.ftAction).toHaveBeenLastCalledWith({ kind: 'auto_cycle', enabled: true })
    cockpit().onSetTxEven(false)
    expect(control.ftAction).toHaveBeenLastCalledWith({ kind: 'parity', even: false })
  })
  it('allows commissioned Tune before the first actual receive period', () => {
    control.mode = null; render(<OriginalHost />)
    expect(cockpit().services!.actions.has('transmit')).toBe(true)
    cockpit().onSetTune!(true)
    expect(control.ftAction).toHaveBeenCalledExactlyOnceWith({ kind: 'tune', enabled: true })
  })
  it('refuses an observed decoder mode mismatch', () => {
    control.mode = 'FT4'; render(<OriginalHost />)
    expect(cockpit().services!.actions.has('transmit')).toBe(false)
  })
  it('retains Stop independently when TX context is unavailable', () => {
    control.fresh = false; render(<OriginalHost />)
    expect(cockpit().services!.actions.has('transmit')).toBe(false)
    expect(cockpit().services!.actions.has('stopTransmit')).toBe(true)
    cockpit().onHaltTx()
    expect(control.ftAction).toHaveBeenCalledExactlyOnceWith({ kind: 'stop' })
  })
  it('preserves manual target/message intent and explicit Log', () => {
    render(<OriginalHost />)
    cockpit().onOverrideTx('K1ABC', 'FN42', 'K1ABC W9XYZ -10')
    expect(control.ftAction).toHaveBeenLastCalledWith({ kind: 'override', call: 'K1ABC', grid: 'FN42', message: 'K1ABC W9XYZ -10' })
    cockpit().onLog()
    expect(control.ftLog).toHaveBeenCalledOnce()
  })
  it('does not substitute local sequence state for physical radio observations', () => {
    render(<OriginalHost />)
    expect(cockpit().snap.radio.transmitting).toBeNull()
    expect(cockpit().snap.qso?.state).toBe('CallingCq')
    expect(cockpit().snap.qso?.cqRunning).toBe(true)
  })
})
