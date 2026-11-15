// @vitest-environment jsdom
import { act,cleanup,fireEvent,render,screen } from '@testing-library/react'
import { afterEach,expect,it,vi } from 'vitest'
import { RotorStrip } from './RotorStrip'
import { bearingFixture } from '../xlog/bearingTestSupport'
import type { ExternalBearingPresentation,ExternalRotorPresentation } from './rotorExternal'
import type { PointEvidenceView } from '../xlog/pointEvidence'
const native=vi.hoisted(()=>({getSettings:vi.fn(),readRotator:vi.fn(),getDeclination:vi.fn(),getSatTrackStatus:vi.fn(),
  getSatTransponder:vi.fn(),stopRotator:vi.fn(),stopSatTrack:vi.fn()}))
vi.mock('../api',()=>native)
vi.mock('../toast',()=>({pushToast:vi.fn()}))
afterEach(()=>{cleanup();vi.restoreAllMocks();vi.clearAllMocks();vi.useRealTimers()})
function presentation(name:Parameters<typeof bearingFixture>[0]='historical_grid') {
  const f=bearingFixture(name)
  const bearing:ExternalBearingPresentation={callsign:f.call,state:'current',facts:f.response.payload.bearing,current:()=>true,
    canRead:()=>true,read:vi.fn(async()=>{}),canPoint:()=>f.response.payload.bearing.status==='available',point:vi.fn(async()=>{}),evidence:null,observationCurrent:()=>true}
  const source:ExternalRotorPresentation={kind:'xlog',targetKey:'exact',observation:null,observationCurrent:()=>false,
    canStop:()=>true,stop:vi.fn(async()=>{}),bearing}
  return {source,bearing}
}
it('keeps original explicit SP/LP controls, visible historical provenance and independent Stop without native services',()=>{
  const {source,bearing}=presentation();render(<RotorStrip external={source}/> )
  expect(screen.getByText(/Historical grid centre/)).toBeTruthy()
  expect(bearing.read).not.toHaveBeenCalled();expect(bearing.point).not.toHaveBeenCalled()
  fireEvent.click(screen.getByRole('button',{name:'→ VK8RX/P'}));expect(bearing.point).toHaveBeenLastCalledWith(false)
  fireEvent.click(screen.getByRole('button',{name:'LP'}));expect(bearing.point).toHaveBeenLastCalledWith(true)
  fireEvent.click(screen.getByRole('button',{name:'Stop rotation NOW'}));expect(source.stop).toHaveBeenCalledOnce()
  for(const method of Object.values(native))expect(method).not.toHaveBeenCalled()
})
it('labels entity coordinates approximate and a typed unavailable reason without enabling movement',()=>{
  const approximate=presentation('cty_approximation'),{rerender}=render(<RotorStrip external={approximate.source}/> )
  expect(screen.getByText('Approximate entity position')).toBeTruthy()
  const unavailable=presentation('reference_unavailable');rerender(<RotorStrip external={unavailable.source}/> )
  expect(screen.getByText('Owner reference unavailable')).toBeTruthy()
  expect((screen.getByRole('button',{name:'→ VK8RX'}) as HTMLButtonElement).disabled).toBe(true)
  expect((screen.getByRole('button',{name:'Stop rotation NOW'}) as HTMLButtonElement).disabled).toBe(false)
})
it('expires facts and preserves receipt as historical request evidence, never a pointed toast',async()=>{
  vi.useFakeTimers();let now=100;vi.spyOn(performance,'now').mockImplementation(()=>now)
  const initial=presentation();let live=true
  const scope={connectionToken:'token',sessionGeneration:1,serverInstance:'server',workspaceId:'workspace'}
  const evidence:PointEvidenceView={intent:{requestId:'immutable-id',scope,command:{kind:'rotor.point',rotor_id:'exact',azimuth_deg:90},baseline:null},
    submitted:true,retired:false,settlement:{kind:'receipt',receipt:{request_id:'immutable-id',server_instance:'server',workspace_id:'workspace',status:'accepted',message:null}},
    observation:{kind:'snapshot',azimuthDeg:90,moving:null,matchesTarget:true,revision:7,generatedAtUnixMs:1000,deadline:200}}
  const source={...initial.source,bearing:{...initial.bearing,current:()=>live,canPoint:()=>live,evidence}}
  render(<RotorStrip external={source}/> )
  expect(screen.getByText(/Point request immutable-id: accepted/)).toBeTruthy()
  expect(screen.getByText(/not a motion confirmation/)).toBeTruthy()
  live=false;now=200
  await act(async()=>{await vi.advanceTimersByTimeAsync(100)})
  expect(screen.queryByText(/Historical grid centre/)).toBeNull()
  expect(screen.queryByText(/not a motion confirmation/)).toBeNull()
  expect(screen.getByText(/Point request immutable-id: accepted/)).toBeTruthy()
})
