// SPDX-License-Identifier: GPL-3.0-only
// Actual Swift body bytes; synthetic paired/native envelope and local clock only.
import corpus from './fixtures/owner-bearing-swift.json'
import type { BearingData, BearingReadView } from './bearingApi'
import type { BearingBinding } from './ownerBearingReader'
export const SERVER='00000000-0000-4000-8000-000000000001'
export const WORKSPACE='00000000-0000-4000-8000-000000000002'
export function bearingFixture(name:keyof typeof corpus='historical_grid') {
  const body=structuredClone(corpus[name]) as BearingData
  const identity={workspace_generation:WORKSPACE,worker_generation:7,settings_revision:11,
    identity_epoch:'00000000-0000-4000-8000-000000000003',identity_revision:3,operating_module:'normal_logging' as const}
  const snapshot={protocol_version:1,server_instance:SERVER,workspace_id:WORKSPACE,revision:5,generated_at_unix_ms:10_000,max_age_ms:3000,
    station:{connected:false,radio_id:null,frequency_hz:null,mode:null,transmitting:null},
    rotor:{connected:true,rotor_id:'exact-rotor',azimuth_deg:171,moving:null},spots:[],
    station_identity:{schema_version:1 as const,source:'saved_station_settings' as const,state:'current' as const,
      mycall:'OM0RX',mygrid:body.origin!.grid,binding:identity,transmit_authority:false as const}}
  const binding:BearingBinding={session:1,view:{paired:true,connected:true,endpoint:null,connection_token:'native-token',
    hello:{application:'OM0RX-xLog',protocol_version:1,server_instance:SERVER,workspace_id:WORKSPACE,
      capabilities:['bearing.read','rotor.point','rotor.stop']},snapshot},lease:{snapshot,deadline:3100},current:()=>true}
  const response:BearingReadView={connection_token:'native-token',remaining_valid_ms:2000,payload:{protocol_version:1,
    server_instance:SERVER,workspace_id:WORKSPACE,generation:WORKSPACE,worker_generation:7,station_identity_binding:identity,
    generated_at_unix_ms:10_100,max_age_ms:5000,bearing:body}}
  return {binding,response,call:body.requested_callsign}
}
export function deferred<T>() {let resolve!:(v:T)=>void;const promise=new Promise<T>(r=>{resolve=r});return {promise,resolve}}
