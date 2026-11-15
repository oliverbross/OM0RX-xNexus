// @vitest-environment jsdom
import { act,cleanup,fireEvent,render,screen } from '@testing-library/react'
import { afterEach,beforeEach,expect,it,vi } from 'vitest'
import { RotorStrip } from './RotorStrip'
import { bearingFixture } from '../xlog/bearingTestSupport'
import type { ExternalBearingPresentation,ExternalRotorPresentation } from './rotorExternal'
import type { PointEvidenceView } from '../xlog/pointEvidence'
const native=vi.hoisted(()=>({getSettings:vi.fn(),readRotator:vi.fn(),getDeclination:vi.fn(),getSatTrackStatus:vi.fn(),
  getSatTransponder:vi.fn(),stopRotator:vi.fn(),stopSatTrack:vi.fn()}))
vi.mock('../api',()=>native)
vi.mock('../toast',()=>({pushToast:vi.fn()}))
beforeEach(()=>{vi.stubGlobal('ResizeObserver',class{observe(){}unobserve(){}disconnect(){}})})
afterEach(()=>{cleanup();vi.restoreAllMocks();vi.clearAllMocks();vi.useRealTimers();vi.unstubAllGlobals()})
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
  expect(screen.getByText('SP 81.7°T · LP 261.7°T')).toBeTruthy()
  expect(source.bearing!.facts!.geometry!.short_path_deg).toBe(81.68647738229987)
  expect(source.bearing!.facts!.geometry!.long_path_deg).toBe(261.68647738229987)
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
  expect(screen.getByRole('button',{name:/Last Point: accepted · 90.0°T/})).toBeTruthy()
  expect(screen.getByText(/not a motion confirmation/)).toBeTruthy()
  live=false;now=200
  await act(async()=>{await vi.advanceTimersByTimeAsync(100)})
  expect(screen.queryByText(/Historical grid centre/)).toBeNull()
  expect(screen.queryByText(/not a motion confirmation/)).toBeNull()
  expect(screen.getByRole('button',{name:/Last Point: accepted · 90.0°T/})).toBeTruthy()
})
it('keeps exact immutable receipt identity in a keyboard-accessible inspection tooltip rather than flooding the strip',()=>{
  vi.spyOn(performance,'now').mockReturnValue(100)
  const initial=presentation(),requestId='12345678-1234-4234-8234-123456789abc',exactAngle=261.68647738229987
  const scope={connectionToken:'token',sessionGeneration:1,serverInstance:'server',workspaceId:'workspace'}
  const evidence:PointEvidenceView={intent:{requestId,scope,command:{kind:'rotor.point',rotor_id:'retained-exact-rotor-identity',azimuth_deg:exactAngle},baseline:null},
    submitted:true,retired:false,settlement:{kind:'receipt',receipt:{request_id:requestId,server_instance:'server',workspace_id:'workspace',status:'accepted',message:null}},
    observation:{kind:'snapshot',azimuthDeg:exactAngle,moving:null,matchesTarget:true,revision:7,generatedAtUnixMs:1000,deadline:200}}
  const source={...initial.source,bearing:{...initial.bearing,evidence}}
  render(<RotorStrip external={source}/> )
  const trigger=screen.getByRole('button',{name:'Last Point: accepted · 261.7°T · Inspect last Point request'})
  expect(trigger.getAttribute('type')).toBe('button');expect(trigger.tabIndex).toBe(0)
  const strip=screen.getByRole('group',{name:'Rotator'})
  expect(strip.textContent).not.toContain(requestId);expect(strip.textContent).not.toContain('retained-exact-rotor-identity')
  expect(strip.textContent).toContain('Later snapshot 261.7°T matches target; not a motion confirmation')
  fireEvent.focus(trigger)
  const tip=document.querySelector('.ui-tooltip')
  expect(tip?.textContent).toContain(`Point request ${requestId}: accepted`)
  expect(tip?.textContent).toContain(`retained-exact-rotor-identity → ${exactAngle}°T`)
  expect(trigger.getAttribute('aria-describedby')).toBeTruthy()
  fireEvent.click(screen.getByRole('button',{name:'Stop rotation NOW'}));expect(source.stop).toHaveBeenCalledOnce()
  expect(evidence.intent.command.azimuth_deg).toBe(exactAngle);expect(evidence.intent.requestId).toBe(requestId)
  expect(initial.bearing.point).not.toHaveBeenCalled()
})
it('formats display-only angle boundaries without changing the supplied geometry',()=>{
  vi.spyOn(performance,'now').mockReturnValue(100)
  const original=presentation(),geometry={short_path_deg:0.04,long_path_deg:359.96,distance_km:1}
  const source={...original.source,observation:{azimuthDeg:0.04,moving:null,deadline:200},observationCurrent:()=>true,
    bearing:{...original.bearing,facts:{...original.bearing.facts!,geometry}}}
  render(<RotorStrip external={source}/> )
  expect(screen.getByText('0.0°T')).toBeTruthy();expect(screen.getByText('SP 0.0°T · LP 360.0°T')).toBeTruthy()
  fireEvent.click(screen.getByRole('button',{name:'LP'}));expect(source.bearing.point).toHaveBeenCalledWith(true)
  expect(geometry).toEqual({short_path_deg:0.04,long_path_deg:359.96,distance_km:1})
})
it.each([
  ['pending','Pending'],['uncertain','Unknown; no retry'],['locally_refused','Not sent'],
] as const)('keeps compact %s separate from accepted or observed motion', (kind,label)=>{
  const initial=presentation()
  const evidence:PointEvidenceView={intent:{requestId:'historical-request',scope:null,
    command:{kind:'rotor.point',rotor_id:'historical-rotor',azimuth_deg:81.68647738229987},baseline:null},
    submitted:kind!=='locally_refused',retired:true,
    settlement:kind==='pending'?{kind}:{kind,reason:'Preserved original reason'},observation:{kind:'unknown'}}
  render(<RotorStrip external={{...initial.source,bearing:{...initial.bearing,evidence}}}/> )
  const trigger=screen.getByRole('button',{name:`Last Point: ${label} · 81.7°T · Inspect last Point request`})
  expect(screen.queryByText(/matches target/)).toBeNull()
  fireEvent.focus(trigger)
  expect(document.querySelector('.ui-tooltip')?.textContent).toContain(`Point request historical-request: ${label}`)
  expect(document.querySelector('.ui-tooltip')?.textContent).toContain('historical-rotor → 81.68647738229987°T')
  expect(initial.bearing.point).not.toHaveBeenCalled()
})
