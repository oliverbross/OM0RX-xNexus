import { useEffect, useRef, useState } from 'react'
import type { Bridge } from './api'
import type { useOwnerConnection } from './useOwnerConnection'
import { createManualCreate, type ManualCreateController } from './manualCreateController'
import type { CreateStorage } from './manualCreateStore'
// Access can throw when storage is unavailable; controller loading preserves that state.
const storage:CreateStorage={getItem:key=>window.localStorage.getItem(key),setItem:(key,value)=>window.localStorage.setItem(key,value),removeItem:key=>window.localStorage.removeItem(key)}
type Connection=Pick<ReturnType<typeof useOwnerConnection>,'view'|'session'|'lease'|'isCurrentReadConnection'>
export function useOwnerManualCreate(connection:Connection,api:Bridge) {
  const [controller,setController]=useState<ManualCreateController|null>(null)
  const current=useRef({connection,api});current.current={connection,api}
  const live=useRef(true),retained=useRef<ManualCreateController|null>(null)
  useEffect(()=>{live.current=true;return()=>{live.current=false;retained.current?.dispose()}},[])
  function open(){
    if(retained.current)return
    const originalApi=api
    const next=createManualCreate({store:storage,port:api,source:()=>{
      const c=current.current.connection,v=c.view,lease=c.lease,h=v.hello
      if(!live.current||current.current.api!==originalApi||!h||!v.connection_token||!lease
        ||lease.snapshot.workspace_id!==h.workspace_id||lease.snapshot.server_instance!==h.server_instance)return null
      const scope={session:c.session,connectionToken:v.connection_token,serverInstance:h.server_instance,workspaceId:h.workspace_id}
      if(!c.isCurrentReadConnection(scope,['logbook.write']))return null
      return{workspaceId:h.workspace_id,serverInstance:h.server_instance,connectionToken:v.connection_token,generation:c.session,deadline:lease.deadline,writeGranted:true}
    }})
    retained.current=next;setController(next)
  }
  function close(){
    const active=retained.current
    if(!active||active.closePolicy()==='blocked')return
    active.dispose();retained.current=null;setController(null)
  }
  return{controller,open,close}
}
