// @vitest-environment jsdom
import { afterEach,it,expect,vi } from 'vitest'
import { act,cleanup,renderHook } from '@testing-library/react'
import { useOwnerManualCreate } from './useOwnerManualCreate'
import { disconnected,type Bridge } from './api'
import type { useOwnerConnection } from './useOwnerConnection'
import { DRAFT_KEY,newSavedDraft } from './draftStore'
import { MANUAL_CREATE_KEY } from './manualCreateStore'
afterEach(()=>{cleanup();localStorage.clear()})
type Connection=Pick<ReturnType<typeof useOwnerConnection>,'view'|'session'|'lease'|'isCurrentReadConnection'>
function fixture(){
 let admitted=true
 const isCurrentReadConnection=vi.fn(()=>admitted)
 const connection:Connection={view:{...disconnected,paired:true,connected:true,connection_token:'token',hello:{application:'OM0RX-xLog',protocol_version:1,server_instance:'owner',workspace_id:'workspace',capabilities:['logbook.write']}},session:1,lease:{snapshot:{protocol_version:1,revision:1,generated_at_unix_ms:Date.now(),max_age_ms:5000,server_instance:'owner',workspace_id:'workspace',station:{connected:false,radio_id:null,frequency_hz:null,mode:null,transmitting:null},rotor:{connected:false,rotor_id:null,azimuth_deg:null,moving:null},spots:[]},deadline:performance.now()+5000},isCurrentReadConnection}
 const forbidden=vi.fn(()=>{throw Error('Unexpected network/IPC')})
 const api={enqueue:forbidden,deliver:forbidden,outbox:forbidden} as unknown as Bridge
 return{connection,api,forbidden,setAdmission:(v:boolean)=>{admitted=v}}
}
it('explicit open only, independent correction bytes survive dirty create close/reopen and no automatic IPC',()=>{
 const f=fixture(),legacy=JSON.stringify(newSavedDraft());localStorage.setItem(DRAFT_KEY,legacy)
 const hook=renderHook(()=>useOwnerManualCreate(f.connection,f.api));expect(hook.result.current.controller).toBeNull()
 expect(localStorage.getItem(MANUAL_CREATE_KEY)).toBeNull();act(()=>hook.result.current.open())
 const first=hook.result.current.controller!;expect(localStorage.getItem(MANUAL_CREATE_KEY)).toBeNull()
 act(()=>first.edit('call','K1ABC'));const raw=localStorage.getItem(MANUAL_CREATE_KEY);act(()=>hook.result.current.close());act(()=>hook.result.current.open())
 expect(hook.result.current.controller!.view().fields.call).toBe('K1ABC');expect(hook.result.current.controller!.view().requestId).toBe(first.view().requestId)
 expect(localStorage.getItem(MANUAL_CREATE_KEY)).toBe(raw);expect(localStorage.getItem(DRAFT_KEY)).toBe(legacy);expect(f.forbidden).not.toHaveBeenCalled()
})
it('fresh synchronous predicate revocation blocks an old handler before a React rerender',()=>{
 const f=fixture(),hook=renderHook(()=>useOwnerManualCreate(f.connection,f.api));act(()=>hook.result.current.open())
 const c=hook.result.current.controller!;f.setAdmission(false);act(()=>c.edit('call','K1ABC'))
 expect(localStorage.getItem(MANUAL_CREATE_KEY)).toBeNull();expect(c.view().canQueue).toBe(false)
 expect(f.connection.isCurrentReadConnection).toHaveBeenCalledWith(expect.objectContaining({connectionToken:'token'}),['logbook.write'])
})
it('API replacement cannot retarget the open draft or old port; explicit reopen retains raw identity',()=>{
 const f=fixture(),hook=renderHook(({api})=>useOwnerManualCreate(f.connection,api),{initialProps:{api:f.api}})
 act(()=>hook.result.current.open());const c=hook.result.current.controller!;act(()=>c.edit('call','K1ABC'))
 const raw=localStorage.getItem(MANUAL_CREATE_KEY);hook.rerender({api:{...f.api}})
 expect(c.view().editable).toBe(false);act(()=>c.edit('call','W9XYZ'));expect(localStorage.getItem(MANUAL_CREATE_KEY)).toBe(raw);expect(f.forbidden).not.toHaveBeenCalled()
})
