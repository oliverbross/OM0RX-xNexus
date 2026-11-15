import { describe,expect,it,vi } from 'vitest'
import { createOwnerBearingReader } from './ownerBearingReader'
import { bearingFixture,deferred } from './bearingTestSupport'
import type { BearingReadView } from './bearingApi'

describe('native-only bearing facts',()=>{
  it.each(['historical_grid','cty_approximation','coincident','reference_unavailable'] as const)('accepts actual producer %s unchanged in a synthetic native envelope',async name=>{
    const f=bearingFixture(name),transport={read:vi.fn(async()=>f.response)},reader=createOwnerBearingReader({transport,now:()=>100})
    const result=await reader.read(f.binding,f.call)
    expect(result.state).toBe('current')
    if(result.state!=='current') throw Error('missing result')
    expect(result.payload.bearing).toEqual(f.response.payload.bearing)
    expect(transport.read).toHaveBeenCalledExactlyOnceWith('native-token',f.call)
    expect(Object.isFrozen(result.payload.bearing)).toBe(true)
    f.response.payload.bearing.requested_callsign='MUTATED'
    expect(result.payload.bearing.requested_callsign).toBe(f.call)
  })
  it.each(['missing-key','extra-key','false-authority','binding','origin','call','case','nan','precision','null','uppercase-uuid','revision','worker'] as const)('rejects malformed %s facts without changing the main view',async kind=>{
    const f=bearingFixture(),before=JSON.stringify(f.binding.view),p=f.response.payload,b=p.bearing
    let response:unknown=f.response
    if(kind==='missing-key') {const {reason:ignored,...rest}=b;void ignored;response={...f.response,payload:{...p,bearing:rest}}}
    if(kind==='extra-key') response={...f.response,payload:{...p,extra:true}}
    if(kind==='false-authority') response={...f.response,payload:{...p,bearing:{...b,motion_authority:true}}}
    if(kind==='binding') p.station_identity_binding={...p.station_identity_binding,settings_revision:12}
    if(kind==='origin') b.origin!.grid='JN99'
    if(kind==='call') b.requested_callsign='VK8RX'
    if(kind==='case') b.normalized_callsign='vk8rx/p'
    if(kind==='nan') b.geometry!.short_path_deg=NaN
    if(kind==='precision') b.target!.precision='entity_approximation'
    if(kind==='null') b.target=null
    if(kind==='uppercase-uuid') b.target!.contact_id='ABCDEFAB-1111-2222-3333-444444444444'
    if(kind==='revision') b.target!.contact_revision=b.logbook_revision+1
    if(kind==='worker') p.worker_generation++
    const reader=createOwnerBearingReader({transport:{read:async()=>response},now:()=>100})
    expect((await reader.read(f.binding,f.call)).state).toBe('unavailable')
    expect(JSON.stringify(f.binding.view)).toBe(before)
  })
  it.each(['call','grant','grid','expiry','sync-failure'] as const)('does not invoke on invalid admission %s',async kind=>{
    const f=bearingFixture(),transport={read:vi.fn(async()=>f.response)}
    if(kind==='grant') f.binding.view.hello!.capabilities=[]
    if(kind==='grid') f.binding.view.snapshot!.station_identity!.mygrid=null
    if(kind==='expiry') f.binding.lease={...f.binding.lease!,deadline:100}
    if(kind==='sync-failure') f.binding.current=()=>false
    const reader=createOwnerBearingReader({transport,now:()=>100})
    expect((await reader.read(f.binding,kind==='call'?'ß1A':f.call)).state).toBe('unavailable')
    expect(transport.read).not.toHaveBeenCalled()
  })
  it('anchors to IPC start and captured snapshot deadline even after a fresh same-identity poll',async()=>{
    const f=bearingFixture(),d=deferred<BearingReadView>();let now=100
    f.binding.lease={...f.binding.lease!,deadline:500}
    const reader=createOwnerBearingReader({transport:{read:()=>d.promise},now:()=>now}),pending=reader.read(f.binding,f.call)
    f.binding.lease={...f.binding.lease,deadline:3100};reader.inspect(f.binding,f.call)
    now=501;d.resolve(f.response);expect((await pending).state).toBe('unavailable')
  })
  it('does not renew the same evidence after expiry or explicit reread',async()=>{
    const f=bearingFixture();let now=100;f.response.remaining_valid_ms=500
    const reader=createOwnerBearingReader({transport:{read:async()=>f.response},now:()=>now})
    const first=await reader.read(f.binding,f.call);expect(first.state).toBe('current')
    now=550;const second=await reader.read(f.binding,f.call);expect(second.state==='current'&&second.deadline).toBe(600)
    now=600;expect(reader.inspect(f.binding,f.call).state).toBe('unavailable')
    expect((await reader.read(f.binding,f.call)).state).toBe('unavailable')
  })
  it('never queues or resurrects a retired A after A-B-A while pending',async()=>{
    const f=bearingFixture(),d=deferred<BearingReadView>(),transport={read:vi.fn(()=>d.promise)}
    const reader=createOwnerBearingReader({transport,now:()=>100}),a=reader.read(f.binding,f.call)
    expect((await reader.read(f.binding,'W1AW')).state).toBe('unavailable')
    reader.inspect(f.binding,f.call);d.resolve(f.response)
    expect((await a).state).toBe('unavailable');expect(reader.inspect(f.binding,f.call).state).toBe('unavailable')
    expect(transport.read).toHaveBeenCalledTimes(1)
  })
  it('retires on synchronous poll failure before React publishes a new view',async()=>{
    const f=bearingFixture();let live=true;f.binding.current=()=>live
    const reader=createOwnerBearingReader({transport:{read:async()=>f.response},now:()=>100}),result=await reader.read(f.binding,f.call)
    live=false;expect(reader.isCurrent(result,f.binding,f.call)).toBe(false)
    live=true;expect(reader.isCurrent(result,f.binding,f.call)).toBe(false)
  })
  it.each(['rollback','throw','negative'] as const)('latches clock %s without another IPC',async kind=>{
    const f=bearingFixture(),transport={read:vi.fn(async()=>f.response)};let now=100,bad=false
    const reader=createOwnerBearingReader({transport,now:()=>{if(bad&&kind==='throw')throw Error('clock');return now}})
    await reader.read(f.binding,f.call);bad=true;now=kind==='negative'?-1:99
    expect((await reader.read(f.binding,f.call)).state).toBe('unavailable')
    bad=false;now=101;expect((await reader.read(f.binding,f.call)).state).toBe('unavailable')
    expect(transport.read).toHaveBeenCalledTimes(1)
  })
  it('plain native rejection is unavailable, not an empty successful result',async()=>{
    const f=bearingFixture(),reader=createOwnerBearingReader({transport:{read:async()=>{throw Error('bearing_busy')}},now:()=>100})
    expect((await reader.read(f.binding,f.call)).state).toBe('unavailable')
    expect(f.binding.view.connected).toBe(true)
  })
})
