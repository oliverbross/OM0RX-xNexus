import { describe,expect,it,vi } from 'vitest'
import { bearingControls,type BearingControlInput } from './ownerBearingControls'
import { createOwnerBearingReader } from './ownerBearingReader'
import { createPointEvidence } from './pointEvidence'
import { bearingFixture,deferred } from './bearingTestSupport'
import type { BearingReadView } from './bearingApi'

function setup() {
  const f=bearingFixture(),transport={read:vi.fn(async()=>f.response)},reader=createOwnerBearingReader({transport,now:()=>100})
  const scope={connectionToken:'native-token',sessionGeneration:1,serverInstance:f.response.payload.server_instance,workspaceId:f.response.payload.workspace_id}
  const poll={scope,origin:'poll' as const,requestStartedMono:50,deadline:3100,snapshot:f.binding.view.snapshot!}
  const observedPoint=vi.fn<BearingControlInput['observedPoint']>(async(command,s)=>{
    const evidence=createPointEvidence({requestId:'11111111-1111-4111-8111-111111111111',scope:s,command,baseline:poll},()=>100)
    evidence.submit(s);return evidence.inspect(s)
  })
  let input:BearingControlInput={binding:f.binding,callsign:f.call,poll,pointEvidence:null,observedPoint,retirePointObservation:vi.fn()}
  const show=()=>bearingControls(reader,input,()=>input,()=>{},()=>100)!
  return {f,reader,transport,observedPoint,show,get:()=>input,set:(next:BearingControlInput)=>{input=next}}
}
describe('explicit owner bearing controls',()=>{
  it('mount and read never Point; explicit SP and LP use exactly the owner angles',async()=>{
    const s=setup();expect(s.transport.read).not.toHaveBeenCalled();expect(s.show().canPoint()).toBe(false)
    await s.show().read();expect(s.observedPoint).not.toHaveBeenCalled()
    await s.show().point(false)
    expect(s.observedPoint).toHaveBeenLastCalledWith({kind:'rotor.point',rotor_id:'exact-rotor',azimuth_deg:s.f.response.payload.bearing.geometry!.short_path_deg},s.get().poll!.scope)
    await s.show().point(true)
    expect(s.observedPoint).toHaveBeenLastCalledWith({kind:'rotor.point',rotor_id:'exact-rotor',azimuth_deg:s.f.response.payload.bearing.geometry!.long_path_deg},s.get().poll!.scope)
  })
  it.each(['call','rotor','grant','poll','sync-scope'] as const)('a captured Point refuses changed %s without retargeting',async kind=>{
    const s=setup();await s.show().read();const captured=s.show()
    if(kind==='call')s.set({...s.get(),callsign:'W1AW'})
    if(kind==='rotor')s.set({...s.get(),poll:{...s.get().poll!,snapshot:{...s.get().poll!.snapshot,rotor:{...s.get().poll!.snapshot.rotor,rotor_id:'replacement'}}}})
    if(kind==='grant')s.f.binding.view.hello!.capabilities=['bearing.read','rotor.stop']
    if(kind==='poll')s.set({...s.get(),poll:null})
    if(kind==='sync-scope')s.f.binding.current=()=>false
    expect(captured.canPoint()).toBe(false);await captured.point(false);expect(s.observedPoint).not.toHaveBeenCalled()
  })
  it('an old Read closure cannot target a new connection with the same call',async()=>{
    const s=setup(),old=s.show();s.f.binding.session++
    await old.read();expect(s.transport.read).not.toHaveBeenCalled()
  })
  it('same-turn selection retirement refuses old Read before React updates input, then fresh B can read',async()=>{
    const s=setup(),old=s.show()
    s.reader.invalidate() // Actual host selection intent, latest input still A.
    expect(old.canRead()).toBe(false);await old.read();expect(s.transport.read).not.toHaveBeenCalled()
    s.set({...s.get(),callsign:'W1AW'})
    const fresh=s.show();expect(fresh.canRead()).toBe(true)
    // The synthetic reply still describes A: B's request is dispatched exactly
    // as selected, and those mismatched facts must remain unavailable.
    await fresh.read();expect(s.transport.read).toHaveBeenCalledExactlyOnceWith('native-token','W1AW')
    expect(s.show().facts).toBeNull()
  })
  it('unavailable owner facts retain their reason but can never authorize Point',async()=>{
    const s=setup(),typed=bearingFixture('reference_unavailable')
    s.set({...s.get(),binding:typed.binding,callsign:typed.call});s.transport.read.mockResolvedValue(typed.response)
    await s.show().read();expect(s.show().facts?.reason).toBe('reference_unavailable')
    await s.show().point(true);expect(s.observedPoint).not.toHaveBeenCalled()
  })
  it('a new explicit read retires old observation before waiting without retrying',async()=>{
    const s=setup(),d=deferred<BearingReadView>();s.transport.read.mockReturnValue(d.promise)
    const pending=s.show().read();expect(s.get().retirePointObservation).toHaveBeenCalledOnce()
    expect(s.show().canRead()).toBe(false);expect(s.observedPoint).not.toHaveBeenCalled()
    d.resolve(s.f.response);await pending;expect(s.transport.read).toHaveBeenCalledTimes(1)
  })
})
