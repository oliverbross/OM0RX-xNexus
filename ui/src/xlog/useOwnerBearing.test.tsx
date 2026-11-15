// @vitest-environment jsdom
import { act,cleanup,renderHook } from '@testing-library/react'
import { afterEach,describe,expect,it,vi } from 'vitest'
import { useOwnerConnection } from './useOwnerConnection'
import { useOwnerBearing } from './useOwnerBearing'
import { bearingFixture,deferred } from './bearingTestSupport'
import type { Bridge,Receipt,View } from './api'
import type { BearingReadView } from './bearingApi'

afterEach(()=>{cleanup();vi.restoreAllMocks();vi.useRealTimers()})
function setup() {
  vi.useFakeTimers();let mono=100;vi.spyOn(performance,'now').mockImplementation(()=>mono)
  const f=bearingFixture();let revision=5
  const view=():View=>({...f.binding.view,snapshot:{...f.binding.view.snapshot!,revision:revision++,generated_at_unix_ms:Date.now()}})
  const api:Bridge={status:vi.fn(async()=>view()),poll:vi.fn(async()=>view()),importPairing:vi.fn(),connect:vi.fn(),disconnect:vi.fn(),
    command:vi.fn<Bridge['command']>(async(_command,id)=>({request_id:id!,server_instance:f.response.payload.server_instance,workspace_id:f.response.payload.workspace_id,status:'accepted',message:null})),
    outbox:vi.fn(),enqueue:vi.fn(),deliver:vi.fn(),history:vi.fn()}
  const transport={read:vi.fn(async()=>f.response)}
  const hook=renderHook(({call}:{call:string})=>{
    const connection=useOwnerConnection(api)
    const bearing=useOwnerBearing({binding:{view:connection.view,lease:connection.lease,session:connection.session,current:connection.isCurrentBearingIdentity},
      callsign:call,poll:connection.poll,observedPoint:connection.observedPoint,pointEvidence:connection.pointEvidence,retirePointObservation:connection.retirePointObservation},transport)
    return {connection,bearing}
  },{initialProps:{call:f.call}})
  return {f,api,transport,hook,time:(n:number)=>{mono=n}}
}
describe('actual owner connection and bearing hook',()=>{
  it('requires explicit read and real poll, keeps Point settlement through temporary read-admission loss',async()=>{
    const s=setup();await act(async()=>{})
    expect(s.transport.read).not.toHaveBeenCalled()
    await act(async()=>{await s.hook.result.current.bearing.presentation!.read()})
    expect(s.hook.result.current.bearing.presentation!.canPoint()).toBe(false)
    await act(async()=>{await vi.advanceTimersByTimeAsync(500)})
    expect(s.hook.result.current.bearing.presentation!.canPoint()).toBe(true)
    const pending=deferred<Receipt>();vi.mocked(s.api.command).mockReturnValue(pending.promise)
    let action!:Promise<void>;act(()=>{action=s.hook.result.current.bearing.presentation!.point(false)})
    expect(s.api.command).toHaveBeenCalledTimes(1)
    const id=vi.mocked(s.api.command).mock.calls[0][1]!
    expect(s.hook.result.current.connection.pointEvidence?.retired).toBe(false)
    await act(async()=>{pending.resolve({request_id:id,server_instance:s.f.response.payload.server_instance,workspace_id:s.f.response.payload.workspace_id,status:'accepted',message:null});await action})
    expect(s.hook.result.current.connection.pointEvidence).toMatchObject({retired:false,settlement:{kind:'receipt',receipt:{status:'accepted'}},observation:{kind:'unknown'}})
    s.time(200);s.f.binding.view.snapshot!.rotor.azimuth_deg=s.f.response.payload.bearing.geometry!.short_path_deg
    await act(async()=>{await vi.advanceTimersByTimeAsync(500)})
    expect(s.hook.result.current.connection.pointEvidence?.observation).toMatchObject({kind:'snapshot',matchesTarget:true})
    s.hook.rerender({call:'W1AW'})
    expect(s.hook.result.current.bearing.presentation?.evidence).toMatchObject({retired:true,settlement:{kind:'receipt'},observation:{kind:'unknown'}})
    act(()=>{s.hook.result.current.bearing.retire()})
    expect(s.hook.result.current.connection.pointEvidence?.settlement.kind).toBe('receipt')
  })
  it('a same-turn connection intent refuses a captured Point before React flush',async()=>{
    const s=setup();await act(async()=>{});await act(async()=>{await vi.advanceTimersByTimeAsync(500)})
    await act(async()=>{await s.hook.result.current.bearing.presentation!.read()})
    const old=s.hook.result.current.bearing.presentation!,d=deferred<View>();vi.mocked(s.api.connect).mockReturnValue(d.promise)
    let reconnect!:Promise<void>
    act(()=>{reconnect=s.hook.result.current.connection.connection('connect');void old.point(false)})
    expect(s.api.command).not.toHaveBeenCalled()
    await act(async()=>{d.resolve(s.f.binding.view);await reconnect})
  })
  it('Stop stays independent of stalled passive read and cannot leave an old Point callback admitted',async()=>{
    const s=setup();await act(async()=>{});await act(async()=>{await vi.advanceTimersByTimeAsync(500)})
    await act(async()=>{await s.hook.result.current.bearing.presentation!.read()})
    const old=s.hook.result.current.bearing.presentation!
    const d=deferred<BearingReadView>();s.transport.read.mockReturnValue(d.promise)
    let reading!:Promise<void>;act(()=>{reading=s.hook.result.current.bearing.presentation!.read()})
    await act(async()=>{await s.hook.result.current.connection.command({kind:'rotor.stop',rotor_id:'exact-rotor'});await old.point(false)})
    expect(s.api.command).toHaveBeenCalledTimes(1)
    expect(vi.mocked(s.api.command).mock.calls[0][0].kind).toBe('rotor.stop')
    await act(async()=>{d.resolve(s.f.response);await reading})
  })
  it('actual poll failure synchronously revokes bearing admission for captured callbacks',async()=>{
    const s=setup();await act(async()=>{});await act(async()=>{await vi.advanceTimersByTimeAsync(500)})
    await act(async()=>{await s.hook.result.current.bearing.presentation!.read()})
    const old=s.hook.result.current.bearing.presentation!,d=deferred<View>();vi.mocked(s.api.poll).mockReturnValue(d.promise)
    // Exercise a rejected actual main poll; the hook clears accepted identity
    // admission before publishing its React state, independently of this reader.
    vi.mocked(s.api.poll).mockRejectedValueOnce(Error('owner unavailable'))
    await act(async()=>{await vi.advanceTimersByTimeAsync(500);await old.point(false)})
    expect(old.current()).toBe(false);expect(s.api.command).not.toHaveBeenCalled()
    expect(s.hook.result.current.connection.view.paired).toBe(true)
  })
})
