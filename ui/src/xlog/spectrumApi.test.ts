// Handwritten transport boundary fixtures. Actual Rust serialization replay is a
// separate opt-in fixture gate; these are not owner/device measurements.
import { afterEach, describe, expect, it, vi } from 'vitest'
import { createSpectrumReader, spectrumApi, type SpectrumBinding, type SpectrumFrame } from './spectrumApi'
import type { RxContext, RxView } from './rxApi'
import serializedFrame from './fixtures/spectrum-frame.json'
import serializedView from './fixtures/rx-context-view.json'

function context(): RxContext {
  return { worker_instance:'worker-1', worker_epoch:1, server_instance:'server-1', workspace_id:'workspace-1', decoder_mode:'FT8',
    route:{ capture_id:'capture-1', epoch:'epoch-1', radio_id:'radio-1', receiver_id:'slice-a', mapping_generation:1,
      device_id:'synthetic-dax', slice:'A', dax_channel:1, frequency_hz:14074000, mode:'DIGU', source_channels:2,
      selected_channel:0, sample_rate:48000, sample_format:'s16le', timestamp_basis:'windows_endpoint_capture', remote_rf_latency_us:null, utc_sync_accuracy_us:null } }
}
function binding(): SpectrumBinding {
  const ctx = context()
  return { sessionGeneration:1, owner:{paired:true,connected:true,hello:{application:'OM0RX-xLog',protocol_version:1,
    server_instance:ctx.server_instance,workspace_id:ctx.workspace_id,capabilities:['snapshot.read','audio.rx']}},
    rx:{enabled:true,mode:'FT8',route:ctx.route,current_context:ctx} }
}
function frame(ctx = context(), sequence = 1): SpectrumFrame {
  return { context:structuredClone(ctx),sequence,remaining_valid_ms:400,sample_rate_hz:12000,fft_samples:2048,
    first_output_index:0,source_origin_frame:0,source_origin_unix_us:1700000000000006,source_sample_rate_hz:48000,
    fir_center_source_frames_twice:63,window_start_unix_us:1700000000000662,window_end_unix_us:1700000000171328,
    timestamp_basis:'windows_endpoint_capture',host_mapping_uncertainty_us:20,calibration_uncertainty_us:10,
    utc_sync_accuracy_us:null,remote_rf_latency_us:null,scale_lo_dbfs:-120,scale_hi_dbfs:0,
    spectrum:{row:Array.from({length:640},(_,i)=>i/640),loHz:0,hiHz:5000,source:'audio'} }
}
function deferred() { let resolve!:(v:unknown)=>void;let reject!:(e:Error)=>void
  const promise = new Promise<unknown>((a,b)=>{resolve=a;reject=b});return {promise,resolve,reject} }
afterEach(()=>vi.unstubAllGlobals())

describe('owner spectrum delivery',()=>{
  it('accepts exact Rust-serialized spectrum and current context before the first decode',async()=>{
    // Actual native serializer, generated PCM/HTTP authority; not a physical owner capture.
    const rx=structuredClone(serializedView) as unknown as RxView
    const ctx=rx.current_context!;expect(rx.rows).toEqual([])
    const owner=binding().owner;owner.hello!.server_instance=ctx.server_instance;owner.hello!.workspace_id=ctx.workspace_id
    let now=100;const wait=deferred();const reader=createSpectrumReader({read:()=>wait.promise,now:()=>now})
    const pending=reader.read({sessionGeneration:1,owner,rx});now=125;wait.resolve(structuredClone(serializedFrame))
    const result=await pending;expect(result.kind).toBe('frame')
    if(result.kind==='frame') {
      expect(result.spectrum).toEqual(serializedFrame.spectrum)
      expect(result.deadline).toBe(100+serializedFrame.remaining_valid_ms)
      expect(result.frame.context).toEqual(rx.current_context)
      now=result.deadline;expect(reader.isCurrent(result)).toBe(false)
    }
  })
  it('invokes only the dedicated native command with explicit cursor, with no web fallback',async()=>{
    const invoke=vi.fn().mockResolvedValue(null)
    vi.stubGlobal('window',{__TAURI__:{core:{invoke}}})
    await spectrumApi.read(null)
    expect(invoke).toHaveBeenCalledExactlyOnceWith('xlog_rx_spectrum',{after:null})
    vi.stubGlobal('window',{})
    await expect(spectrumApi.read(null)).rejects.toThrow('desktop')
  })
  it('uses request START and consumes a sequence once without extending its deadline',async()=>{
    let now=1000;const wait=deferred();const native=vi.fn().mockReturnValueOnce(wait.promise).mockResolvedValue(frame())
    const reader=createSpectrumReader({read:native,now:()=>now});const scope=binding()
    const pending=reader.read(scope);now=1100;wait.resolve(frame());const first=await pending
    expect(first.kind).toBe('frame');expect(first.deadline).toBe(1400)
    expect(reader.isCurrent(first)).toBe(true)
    now=1200;const duplicate=await reader.read(scope)
    expect(duplicate.kind).toBe('unavailable');expect(duplicate.deadline).toBe(1400)
    expect(native.mock.calls[1][0]).toEqual({worker_instance:'worker-1',worker_epoch:1,sequence:1})
    now=1400;expect(reader.isCurrent(first)).toBe(false)
    const expired=reader.inspect();expect(expired.deadline).toBeNull();expect(expired.resetToken).toBeGreaterThan(first.resetToken)
    expect((await reader.read(scope)).kind).toBe('unavailable')
  })
  it('rejects transit expiry and never resurrects the same sequence with a new response budget',async()=>{
    let now=0;const wait=deferred();const native=vi.fn().mockReturnValueOnce(wait.promise).mockResolvedValue(frame())
    const reader=createSpectrumReader({read:native,now:()=>now});const scope=binding()
    const pending=reader.read(scope);now=401;wait.resolve(frame())
    expect((await pending).kind).toBe('unavailable')
    now=500;expect((await reader.read(scope)).kind).toBe('unavailable')
    native.mockResolvedValueOnce(frame(context(),2));expect((await reader.read(scope)).kind).toBe('frame')
  })
  it('null/no-new and errors never renew the last picture; errors clear immediately',async()=>{
    let now=0;const native=vi.fn().mockResolvedValueOnce(frame()).mockResolvedValueOnce(null).mockRejectedValueOnce(new Error('fixture'))
    const reader=createSpectrumReader({read:native,now:()=>now});const scope=binding();const first=await reader.read(scope)
    now=100;const empty=await reader.read(scope);expect(empty.kind).toBe('unavailable');expect(empty.deadline).toBe(400)
    now=200;const failed=await reader.read(scope);expect(failed.kind).toBe('unavailable');expect(failed.deadline).toBeNull()
    expect(failed.resetToken).toBeGreaterThan(first.resetToken);expect(reader.isCurrent(first)).toBe(false)
  })
  it('bounds stalled IPC to one request and rejects a late result after same-IDs re-pair',async()=>{
    const wait=deferred();const native=vi.fn().mockReturnValueOnce(wait.promise).mockResolvedValue(frame())
    const reader=createSpectrumReader({read:native,now:()=>0});const scope=binding();const pending=reader.read(scope)
    expect((await reader.read(scope))).toMatchObject({kind:'unavailable',reason:'busy'});expect(native).toHaveBeenCalledTimes(1)
    scope.sessionGeneration++;const state=reader.synchronize(scope)
    expect((await reader.read(scope)).kind).toBe('unavailable');expect(native).toHaveBeenCalledTimes(1)
    wait.resolve(frame());const old=await pending;expect(old).toMatchObject({kind:'unavailable',reason:'superseded'})
    expect(reader.isCurrent(old)).toBe(false)
    const fresh=await reader.read(scope);expect(fresh.kind).toBe('frame');expect(fresh.bindingGeneration).toBe(state.bindingGeneration)
  })
  it.each(['epoch','worker','route','mode','workspace','server','stop','reset'] as const)('retires inflight results at %s boundary',async boundary=>{
    const wait=deferred();const reader=createSpectrumReader({read:()=>wait.promise,now:()=>0});const scope=binding()
    const pending=reader.read(scope);const ctx=scope.rx.current_context!
    if(boundary==='epoch')ctx.worker_epoch++
    if(boundary==='worker')ctx.worker_instance='new-worker'
    if(boundary==='route')ctx.route.frequency_hz++
    if(boundary==='mode'){ctx.decoder_mode='FT4';scope.rx.mode='FT4'}
    if(boundary==='workspace'){ctx.workspace_id='other';scope.owner.hello!.workspace_id='other'}
    if(boundary==='server'){ctx.server_instance='other';scope.owner.hello!.server_instance='other'}
    if(boundary==='stop')scope.rx.enabled=false
    if(boundary==='reset')reader.reset();else reader.synchronize(scope)
    wait.resolve(frame());const result=await pending
    expect(result.kind).toBe('unavailable');expect(reader.isCurrent(result)).toBe(false)
  })
  it.each(['legacy','stopped','disconnected','capability','route','mode','foreignOwner'] as const)('does not query with unavailable %s authority',async reason=>{
    const scope=binding();const native=vi.fn();const reader=createSpectrumReader({read:native,now:()=>0})
    if(reason==='legacy')delete scope.rx.current_context
    if(reason==='stopped')scope.rx.enabled=false
    if(reason==='disconnected')scope.owner.connected=false
    if(reason==='capability')scope.owner.hello!.capabilities=[]
    if(reason==='route')scope.rx.route={...scope.rx.route!,frequency_hz:7074000}
    if(reason==='mode')scope.rx.mode='FT4'
    if(reason==='foreignOwner')scope.owner.hello!.workspace_id='other'
    expect((await reader.read(scope)).kind).toBe('unavailable');expect(native).not.toHaveBeenCalled()
  })
  it('allows a new epoch/worker sequence to restart only with matching current native context',async()=>{
    let offered=frame(context(),999);const reader=createSpectrumReader({read:async()=>offered,now:()=>0});const scope=binding()
    expect((await reader.read(scope)).kind).toBe('frame')
    scope.rx.current_context!.worker_epoch++;offered=frame(scope.rx.current_context!,1)
    expect((await reader.read(scope)).kind).toBe('frame')
    scope.rx.current_context!.worker_instance='worker-2';offered=frame(scope.rx.current_context!,1)
    expect((await reader.read(scope)).kind).toBe('frame')
  })
  it.each(['context','remaining','bins','nan','scale','source','rate','time','calibration','unsafeCounter'] as const)('rejects malformed %s native data without inventing a row',async reason=>{
    const bad=frame()
    if(reason==='context')bad.context.worker_epoch++
    if(reason==='remaining')bad.remaining_valid_ms=501
    if(reason==='bins')bad.spectrum.row.pop()
    if(reason==='nan')bad.spectrum.row[4]=NaN
    if(reason==='scale')Object.assign(bad,{scale_lo_dbfs:-100})
    if(reason==='source')Object.assign(bad.spectrum,{source:'flex'})
    if(reason==='rate')bad.source_sample_rate_hz=44100
    if(reason==='time')bad.window_start_unix_us++
    if(reason==='calibration')bad.calibration_uncertainty_us=21
    if(reason==='unsafeCounter')bad.sequence=Number.MAX_SAFE_INTEGER+1
    const reader=createSpectrumReader({read:async()=>bad,now:()=>0})
    expect(await reader.read(binding())).toMatchObject({kind:'unavailable',reason:'invalid'})
  })
  it('fails closed on a non-monotonic clock',async()=>{
    let now=100;const reader=createSpectrumReader({read:async()=>frame(),now:()=>now});const scope=binding()
    const first=await reader.read(scope);expect(first.kind).toBe('frame');now=99
    expect(reader.isCurrent(first)).toBe(false);expect((await reader.read(scope)).kind).toBe('unavailable')
  })
})
