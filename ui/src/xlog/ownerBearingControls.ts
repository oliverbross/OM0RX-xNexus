// SPDX-License-Identifier: GPL-3.0-only
import type { ExternalBearingPresentation } from '../components/rotorExternal'
import type { ControlPoll, ControlScope } from './controlObservation'
import type { PointEvidenceView } from './pointEvidence'
import { bearingBindingKey, createOwnerBearingReader, literalBearingCall, type BearingBinding } from './ownerBearingReader'
import type { Command } from './api'

export interface BearingControlInput {
  binding: BearingBinding
  callsign: string | null
  poll: ControlPoll | null
  pointEvidence: PointEvidenceView | null
  observedPoint: (command: Extract<Command,{kind:'rotor.point'}>, scope: ControlScope) => Promise<PointEvidenceView>
  retirePointObservation: () => void
}
/** Read and Point closures capture the displayed call/context/rotor; they never
 * derive a different target from a later mutable ref. No native service fallback. */
export function bearingControls(reader: ReturnType<typeof createOwnerBearingReader>, input: BearingControlInput,
  latest: () => BearingControlInput, changed: () => void, now: () => number = () => performance.now()): ExternalBearingPresentation | undefined {
  const call=input.callsign
  if(!literalBearingCall(call)) return undefined
  const result=reader.inspect(input.binding,call), key=bearingBindingKey(input.binding,now())
  const rotorId=input.binding.lease?.snapshot.rotor.rotor_id, scope=input.poll?.scope
  const same=()=>{const v=latest();return v.callsign===call && key!==null && bearingBindingKey(v.binding,now())===key}
  // A same-turn selection intent can retire this result before React replaces
  // latest(). Do not let the old Read closure recreate that retired intent.
  const owns=()=>same()&&reader.inspect(latest().binding,call)===result
  const current=()=>same()&&reader.isCurrent(result,latest().binding,call)
  const canPoint=()=>{
    if(!current()||result.state!=='current'||result.payload.bearing.status!=='available'||!result.payload.bearing.geometry||!scope||!rotorId) return false
    const v=latest(), p=v.poll, at=now()
    return !!(v.binding.view.hello?.capabilities.includes('rotor.point')&&p&&p.origin==='poll'&&p.deadline>at
      && p.snapshot.rotor.connected&&p.snapshot.rotor.rotor_id===rotorId
      && p.scope.connectionToken===scope.connectionToken&&p.scope.sessionGeneration===scope.sessionGeneration
      && p.scope.serverInstance===scope.serverInstance&&p.scope.workspaceId===scope.workspaceId)
  }
  return {
    callsign:call,state:result.state,facts:result.state==='current'?result.payload.bearing:null,current,
    canRead:()=>owns()&&result.state!=='loading',
    read:async()=>{
      if(!owns()||result.state==='loading') return
      latest().retirePointObservation()
      const pending=reader.read(latest().binding,call);changed()
      await pending;changed()
    },
    canPoint,
    point:async longPath=>{
      if(!canPoint()||result.state!=='current'||!scope||!rotorId) return
      const geometry=result.payload.bearing.geometry!
      // Exact owner angle; no rounding, reciprocal calculation or inferred tolerance.
      const command:Extract<Command,{kind:'rotor.point'}>={kind:'rotor.point',rotor_id:rotorId,
        azimuth_deg:longPath?geometry.long_path_deg:geometry.short_path_deg}
      await latest().observedPoint(command,{...scope});changed()
    },
    evidence:input.pointEvidence,
    observationCurrent:()=>same()&&latest().pointEvidence===input.pointEvidence&&!input.pointEvidence?.retired,
  }
}
