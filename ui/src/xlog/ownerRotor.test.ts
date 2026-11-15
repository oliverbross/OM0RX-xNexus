import { describe, expect, it, vi } from 'vitest'
import { createOwnerRotor, type OwnerRotorInput } from './ownerRotor'
import { disconnected, type Bridge } from './api'

function input(): OwnerRotorInput {
  const snapshot = { protocol_version: 1 as const, server_instance: 'server', workspace_id: 'workspace', revision: 1,
    generated_at_unix_ms: 10_000, max_age_ms: 3000,
    station: { connected: false, radio_id: null, frequency_hz: null, mode: null, transmitting: null },
    rotor: { connected: true, rotor_id: 'route-1', azimuth_deg: 90, moving: null }, spots: [] }
  return { api: {} as Bridge, session: 1, view: { ...disconnected, paired: true, connected: true, connection_token: 'token-1',
    hello: { application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'server', workspace_id: 'workspace', capabilities: ['rotor.stop'] }, snapshot },
    lease: { snapshot, deadline: 1000 } }
}
describe('owner exact rotor target', () => {
  it('retains Stop through expiry, missing azimuth, ordinary lease removal and poll failure without retaining telemetry', async () => {
    const owner = createOwnerRotor(), command = vi.fn(async () => {}); let current = input()
    const show = (now = 100) => owner.present(current, () => current, () => true, command, now)
    const first = show(); expect(first.observation?.azimuthDeg).toBe(90)
    current = { ...current, lease: null }
    expect(first.observationCurrent()).toBe(false)
    const pending = show(); expect(pending.observation).toBeNull(); await pending.stop!()
    current = { ...current, view: { ...current.view, connected: false, snapshot: null } }
    const failed = show(2000); expect(failed.observation).toBeNull(); expect(failed.canStop()).toBe(true); await failed.stop!()
    expect(command.mock.calls).toEqual([[{ kind: 'rotor.stop', rotor_id: 'route-1' }], [{ kind: 'rotor.stop', rotor_id: 'route-1' }]])
  })
  it.each(['api', 'token', 'session', 'server', 'workspace', 'capability', 'unpaired'] as const)('retires old closure on %s and cannot restore it after ABA', async change => {
    const owner = createOwnerRotor(), command = vi.fn(async () => {}), original = input(); let current = original
    const show = () => owner.present(current, () => current, () => true, command, 100)
    const first = show()
    current = { ...original, view: { ...original.view, hello: { ...original.view.hello! } }, lease: null }
    if (change === 'api') current.api = {} as Bridge
    if (change === 'token') current.view.connection_token = 'replacement'
    if (change === 'session') current.session++
    if (change === 'server') current.view.hello!.server_instance = 'other'
    if (change === 'workspace') current.view.hello!.workspace_id = 'other'
    if (change === 'capability') current.view.hello!.capabilities = []
    if (change === 'unpaired') current.view.paired = false
    expect(first.canStop()).toBe(false); expect(show().stop).toBeNull()
    current = original; show()
    await expect(first.stop!()).rejects.toThrow('changed'); expect(command).not.toHaveBeenCalled()
  })
  it('does not retarget a captured Stop to a newly observed rotor in the same connection', async () => {
    const owner = createOwnerRotor(), command = vi.fn(async () => {}); let current = input()
    const first = owner.present(current, () => current, () => true, command, 100)
    const snapshot = { ...current.view.snapshot!, rotor: { ...current.view.snapshot!.rotor, rotor_id: 'route-2' } }
    current = { ...current, view: { ...current.view, snapshot }, lease: { snapshot, deadline: 1000 } }
    const next = owner.present(current, () => current, () => true, command, 100)
    await expect(first.stop!()).rejects.toThrow('changed'); await next.stop!()
    expect(command).toHaveBeenCalledExactlyOnceWith({ kind: 'rotor.stop', rotor_id: 'route-2' })
  })
  it('refuses a target never observed under a fresh lease and a late old snapshot cannot establish foreign authority', () => {
    const owner = createOwnerRotor(), command = vi.fn(async () => {}), current = input()
    current.lease = { ...current.lease!, deadline: 99 }
    expect(owner.present(current, () => current, () => true, command, 100).stop).toBeNull()
    current.lease = { ...current.lease!, deadline: 1000 }; current.view.hello!.workspace_id = 'foreign'
    expect(owner.present(current, () => current, () => true, command, 100).stop).toBeNull()
  })
  it('checks synchronous connection admission again immediately before dispatch', async () => {
    const owner = createOwnerRotor(), command = vi.fn(async () => {}), current = input(); let admitted = true
    const first = owner.present(current, () => current, () => admitted, command, 100)
    admitted = false
    await expect(first.stop!()).rejects.toThrow('changed'); expect(command).not.toHaveBeenCalled()
  })
})
