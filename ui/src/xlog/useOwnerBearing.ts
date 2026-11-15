// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useMemo, useRef, useState } from 'react'
import { bearingReads } from './bearingApi'
import { bearingIdentityKey, createOwnerBearingReader } from './ownerBearingReader'
import { bearingControls, type BearingControlInput } from './ownerBearingControls'

/** Explicit reads only; timer retires displayed leases, never issues a read. */
export function useOwnerBearing(input: BearingControlInput, transport= bearingReads) {
  const reader=useMemo(()=>createOwnerBearingReader({transport}),[transport])
  const latest=useRef(input)
  const [,refresh]=useState(0), active=useRef(true)
  const changed=()=>{if(active.current) refresh(n=>n+1)}
  const v=input.binding.view, identity=v.snapshot?.station_identity
  // An ordinary command temporarily removes read admission. That must hide
  // bearing facts without destroying the Point's later-observation tracker.
  const key=JSON.stringify([input.binding.session,v.connection_token,v.connected,v.hello?.server_instance,
    v.hello?.workspace_id,v.hello?.capabilities.includes('bearing.read'),v.hello?.capabilities.includes('rotor.point'),
    bearingIdentityKey(identity?.binding),identity?.mygrid,input.callsign])
  const prior=useRef(key)
  let effective=input
  if(prior.current!==key) {
    prior.current=key;reader.invalidate();input.retirePointObservation()
    // The connection hook's returned object was constructed earlier in this
    // same render. Hide its retired observation now, retaining settlement.
    if(input.pointEvidence) effective={...input,pointEvidence:{...input.pointEvidence,retired:true,observation:{kind:'unknown'}}}
  }
  latest.current=effective
  reader.inspect(input.binding,input.callsign)
  useEffect(()=>{active.current=true;const timer=setInterval(changed,100)
    return()=>{active.current=false;clearInterval(timer);reader.invalidate()}
  },[reader])
  return { presentation:bearingControls(reader,effective,()=>latest.current,changed),
    retire:()=>{reader.invalidate();input.retirePointObservation()} }
}
