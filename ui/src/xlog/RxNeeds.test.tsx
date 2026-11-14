// @vitest-environment jsdom
// SPDX-License-Identifier: GPL-3.0-only
import { act, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { RxPanel } from './RxPanel'
import { rxOff, type RxApi, type RxRow, type RxView } from './rxApi'
import serializedNativeView from './fixtures/decoded-needs.json'

const route = {
  capture_id:'capture-a', epoch:'epoch-a', radio_id:'radio1', receiver_id:'receiver-a',
  mapping_generation:1, device_id:'dax-rx-a', slice:'A', dax_channel:2,
  frequency_hz:14074000, mode:'DIGU', source_channels:2, selected_channel:0,
  sample_rate:48000, sample_format:'pcm_s16le', timestamp_basis:'windows_endpoint_capture',
  remote_rf_latency_us:null, utc_sync_accuracy_us:null,
}
const authority = { logbook_revision:3, classification_revision:5, country_reference_revision:'reference-a', policy_revision:'xlog-call-dxcc-needs-v1' }
function row(id='decode_a', call='K1ABC', status:'needed'|'worked'|'unknown'='needed') {
  const need = { status, score:status==='unknown' ? null : status==='needed' ? 91 : 0,
    reason_code:status==='unknown' ? 'country_unknown' : null, reasons:status==='needed' ? ['New call on this band'] : [] }
  return {
    decode_id:id, sender_callsign:call,
    context:{ worker_instance:'worker-a', worker_epoch:1, server_instance:'server-a', workspace_id:'workspace-a', route, decoder_mode:'FT8' as const },
    host_period_start_unix_us:1794682200000000, mode:'FT8' as const, message:`CQ ${call} FN42`,
    snr_db:-17, time_offset_seconds:0.25, audio_frequency_hz:1200,
    clock:{ timestamp_basis:'windows_endpoint_capture', host_mapping_uncertainty_us:1000, calibration_uncertainty_us:1000, utc_sync_accuracy_us:null, remote_rf_latency_us:null, sample_alignment_offset_ns:0, resampler_center_offset_ns:0 },
    needs:{ ...need, authority, generated_at_unix_ms:1794682200000, max_age_ms:2000, remaining_valid_ms:1000,
      owner:{ query_id:'cached-query-from-another-decode', normalized_callsign:call, dial_frequency_hz:14074000,
        decoder_mode:'FT8' as const, classification_mode:'DIGI', band:'20m', country:status==='unknown' ? null : 'United States',
        adif:status==='unknown' ? null : 291, country_resolution:status==='unknown' ? 'unknown' : 'resolved',
        call_status:status==='needed' ? 'New Call' : 'Worked', dxcc_status:status==='unknown' ? 'Unknown' : 'Worked', need },
    },
  }
}
function snapshot(rows: RxRow[]=[row()]): RxView {
  return { enabled:true, mode:'FT8', phase:'receiving', reason:'Receiving owner audio.', route, rows }
}
function native(view:RxView): RxApi {
  return { status:vi.fn(async () => view), start:vi.fn(async () => view), stop:vi.fn(async () => rxOff) }
}
let monotonic=0
beforeEach(() => { vi.useFakeTimers(); monotonic=0; vi.spyOn(performance,'now').mockImplementation(() => monotonic) })
afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers() })
async function tick(ms=0) { await act(async () => { monotonic+=ms; await vi.advanceTimersByTimeAsync(ms) }) }
function panel(api:RxApi, props:Partial<{fresh:boolean; needsCapable:boolean; identity:string}>={}) {
  return <RxPanel api={api} connected capable fresh needsCapable identity="pair-a" ownerIdentity={{server_instance:'server-a',workspace_id:'workspace-a'}} {...props} />
}
function decode(message='CQ K1ABC FN42') { return within(screen.getByText(message).closest('tr')!) }

describe('authoritative needs on decoded rows', () => {
  it('renders the exact Rust Row/View serialization without renaming fields', async () => {
    // Exact bytes from the native worker serialization test; source provenance is beside the fixture.
    const view=serializedNativeView as RxView
    render(<RxPanel api={native(view)} connected capable fresh needsCapable identity="native-fixture" ownerIdentity={view.rows[0].context} />)
    await tick()
    expect(decode().getByText('Needed')).toBeTruthy()
    expect(decode().getByText('xLog score 100')).toBeTruthy()
    expect(decode().getByText('Owner fixture new call')).toBeTruthy()
    expect(decode().getByText(/United States.*DXCC 291/)).toBeTruthy()
    expect(decode('CQ W9XYZ FN42').getByText('Unknown')).toBeTruthy()
    expect(decode('CQ W9XYZ FN42').queryByText(/xLog score/)).toBeNull()
    await tick(view.rows[0].needs.remaining_valid_ms+1)
    expect(decode().getByText('Unknown')).toBeTruthy()
  })

  it('shows owner status, score and evidence separately from raw signal measurements', async () => {
    render(panel(native(snapshot([row(),row('decode_b','W9XYZ','worked'),row('decode_c','K2ABC','unknown')]))))
    await tick()
    expect(screen.getByRole('table',{name:'Decoded messages'})).toBeTruthy()
    expect(decode().getByText('Needed')).toBeTruthy()
    expect(decode().getByText('xLog score 91')).toBeTruthy()
    expect(decode().getByText('New call on this band')).toBeTruthy()
    expect(decode().getByText(/-17 dB.*0.25 s.*1200 Hz/)).toBeTruthy()
    expect(decode('CQ W9XYZ FN42').getByText('Worked')).toBeTruthy()
    expect(decode('CQ W9XYZ FN42').getByText('xLog score 0')).toBeTruthy()
    expect(decode('CQ K2ABC FN42').getByText('Unknown')).toBeTruthy()
    expect(decode('CQ K2ABC FN42').queryByText(/xLog score/)).toBeNull()
    expect(decode().getByText(/20m.*FT8.*DIGI/)).toBeTruthy()
    expect(decode().getByText(/Owner time/)).toBeTruthy()
    expect(screen.queryByText(/propagation score/i)).toBeNull()
  })

  it('keeps decodes visible but Unknown when the classification capability is absent', async () => {
    const api=native(snapshot())
    render(panel(api,{needsCapable:false}))
    await tick()
    expect(decode().getByText('Unknown')).toBeTruthy()
    expect(screen.getByText(/Needs lookup is not available from this xLog connection/)).toBeTruthy()
    expect(decode().queryByText(/xLog score|New call on this band|United States/)).toBeNull()
    expect(api.start).not.toHaveBeenCalled()
  })

  it('does not infer needs from absent or incomplete metadata', async () => {
    const missing={...row()}; delete (missing as {needs?:unknown}).needs
    const incomplete={...row('decode_b','W9XYZ'),needs:{...row().needs,owner:null}}
    render(panel(native(snapshot([missing,incomplete]))))
    await tick()
    expect(screen.getAllByText('Unknown')).toHaveLength(2)
    expect(screen.queryByText(/xLog score|New call on this band/)).toBeNull()
  })

  it('accounts for IPC time before displaying a returned lease', async () => {
    let finish!:(value:RxView)=>void
    const api=native(rxOff);api.status=vi.fn(()=>new Promise<RxView>(resolve=>{finish=resolve}))
    render(panel(api))
    await tick(1100)
    await act(async()=>{finish(snapshot())})
    expect(decode().getByText('Unknown')).toBeTruthy()
    expect(decode().queryByText('Needed')).toBeNull()
  })

  it('expires repeated snapshots without renewing their facts or trusting wall-clock jumps', async () => {
    const api=native(snapshot())
    render(panel(api));await tick()
    expect(decode().getByText('Needed')).toBeTruthy()
    vi.setSystemTime(new Date('2099-01-01T00:00:00Z'))
    await tick(600)
    expect(decode().getByText('Needed')).toBeTruthy()
    await tick(450)
    expect(decode().getByText('Unknown')).toBeTruthy()
    expect(decode().queryByText(/xLog score|New call on this band|United States/)).toBeNull()
    expect(api.status).toHaveBeenCalledTimes(3)
  })

  it('clears facts immediately on stale observations or a native status error', async () => {
    const api=native(snapshot());const view=render(panel(api));await tick()
    view.rerender(panel(api,{fresh:false}))
    expect(decode().getByText('Unknown')).toBeTruthy()
    view.rerender(panel(api));await tick()
    api.status=vi.fn(async()=>{throw Error('Native status unavailable')})
    await tick(500)
    expect(decode().getByText('Unknown')).toBeTruthy()
    expect(decode().queryByText(/xLog score/)).toBeNull()
  })

  it('refuses facts belonging to a different route or literal sender', async () => {
    const wrongCall=row();wrongCall.needs.owner.normalized_callsign='W9XYZ'
    const wrongRoute=row('decode_b','K2ABC');wrongRoute.context={...wrongRoute.context,route:{...route,capture_id:'retired-capture'}}
    render(panel(native(snapshot([wrongCall,wrongRoute]))));await tick()
    expect(screen.getAllByText('Unknown')).toHaveLength(2)
    expect(screen.queryByText(/xLog score|New call on this band/)).toBeNull()
  })

  it('requires the current paired owner and matching decoder mode', async () => {
    const foreign=row();foreign.context.workspace_id='another-logbook'
    const mode:RxRow=row('decode_b','K2ABC');mode.needs.owner!.decoder_mode='FT4'
    render(panel(native(snapshot([foreign,mode]))));await tick()
    expect(screen.getAllByText('Unknown')).toHaveLength(2)
    expect(screen.queryByText(/xLog score/)).toBeNull()
  })

  it('rejects an old decoder mode even when the radio route has not changed', async () => {
    render(panel(native({...snapshot(),mode:'FT4'})));await tick()
    expect(decode().getByText('Unknown')).toBeTruthy()
    expect(decode().queryByText(/xLog score/)).toBeNull()
  })

  it('cannot restore new owner facts while Stop is pending', async () => {
    let current=snapshot();const api=native(current);api.status=vi.fn(async()=>current)
    let finish!:(value:RxView)=>void;api.stop=vi.fn(()=>new Promise<RxView>(resolve=>{finish=resolve}))
    render(panel(api));await tick()
    fireEvent.click(screen.getByRole('button',{name:'Stop receiving'}))
    expect(decode().getByText('Unknown')).toBeTruthy()
    current=snapshot([{...row(),needs:{...row().needs,generated_at_unix_ms:1794682200500}}]);await tick(500)
    expect(decode().getByText('Unknown')).toBeTruthy()
    expect(decode().queryByText(/xLog score/)).toBeNull()
    await act(async()=>{finish(rxOff)})
    expect(screen.queryByText('CQ K1ABC FN42')).toBeNull()
  })

  it('expires every row even when the next native status reply stalls', async () => {
    const a=row(),b=row('decode_b','W9XYZ','worked');b.needs.remaining_valid_ms=1500
    const api=native(snapshot([a,b]));vi.mocked(api.status).mockImplementationOnce(async()=>snapshot([a,b])).mockImplementation(()=>new Promise(()=>{}))
    render(panel(api));await tick();await tick(1050)
    expect(decode().getByText('Unknown')).toBeTruthy()
    expect(decode('CQ W9XYZ FN42').getByText('Worked')).toBeTruthy()
    await tick(500)
    expect(decode('CQ W9XYZ FN42').getByText('Unknown')).toBeTruthy()
  })

  it('keys rows by immutable identity across reordering and discards trimmed row evidence', async () => {
    const a=row(),b=row('decode_b','W9XYZ','worked');let current=snapshot([a,b])
    const api=native(current);api.status=vi.fn(async()=>current)
    render(panel(api));await tick()
    const first=screen.getByText(a.message).closest('tr')
    current=snapshot([b,a]);await tick(500)
    expect(screen.getByText(a.message).closest('tr')).toBe(first)
    current=snapshot([b]);await tick(500)
    expect(screen.queryByText(a.message)).toBeNull()
    expect(screen.queryByText('xLog score 91')).toBeNull()
  })

  it('cannot attach a delayed old-pairing result after reconnecting', async () => {
    let finish!:(value:RxView)=>void
    const old=native(rxOff);old.status=vi.fn(()=>new Promise<RxView>(resolve=>{finish=resolve}))
    const fresh=native(snapshot([row('decode_new','W9XYZ','worked')]))
    const view=render(panel(old));await tick()
    view.rerender(panel(fresh,{identity:'pair-b'}));await tick()
    await act(async()=>{finish(snapshot())})
    expect(screen.queryByText('CQ K1ABC FN42')).toBeNull()
    expect(decode('CQ W9XYZ FN42').getByText('Worked')).toBeTruthy()
  })

  it('keeps revoked evidence dead until xLog supplies a new fact', async () => {
    const known=row();known.needs.remaining_valid_ms=2000
    let current=snapshot([known]);const api=native(current)
    api.status=vi.fn(async()=>current)
    render(panel(api));await tick()
    const blocked={...known,needs:{status:'unknown' as const,score:null,reason_code:'native_write_pending',reasons:[],owner:null,authority:null,generated_at_unix_ms:null,max_age_ms:null,remaining_valid_ms:0}}
    current={...current,rows:[blocked]};await tick(500)
    expect(decode().getByText(/Awaiting the outcome of a queued logbook change/)).toBeTruthy()
    current=snapshot([known]);await tick(100)
    // The next status reply contains the same old owner fact; it cannot revive it.
    await tick(400)
    expect(decode().getByText('Unknown')).toBeTruthy()
    current=snapshot([{...known,needs:{...known.needs,generated_at_unix_ms:known.needs.generated_at_unix_ms+1000}}]);await tick(500)
    expect(decode().getByText('Needed')).toBeTruthy()
  })
})
