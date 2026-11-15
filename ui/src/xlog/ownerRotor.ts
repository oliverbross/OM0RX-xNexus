// SPDX-License-Identifier: GPL-3.0-only
import type { ExternalRotorPresentation } from '../components/rotorExternal'
import type { Bridge, Command, View } from './api'
import type { SnapshotLease } from './ownerOperatingModel'
import type { OwnerConnectionScope } from './useOwnerConnection'

export interface OwnerRotorInput { api: Bridge; view: View; lease: SnapshotLease | null; session: number }
type Target = { api: Bridge; scope: OwnerConnectionScope; rotorId: string }
const scopeOf = ({ view, session }: OwnerRotorInput): OwnerConnectionScope | null => view.paired && view.connection_token && view.hello
  ? { session, connectionToken: view.connection_token, serverInstance: view.hello.server_instance, workspaceId: view.hello.workspace_id } : null
const same = (a: OwnerConnectionScope, b: OwnerConnectionScope) => a.session === b.session && a.connectionToken === b.connectionToken
  && a.serverInstance === b.serverInstance && a.workspaceId === b.workspaceId
const clockValid = (n: number) => Number.isFinite(n) && n >= 0

/** One host's retained exact target. No IO, timers or automatic command retry.
 * Each returned Stop closure captures its target; a later route cannot retarget it. */
export function createOwnerRotor() {
  let retained: Target | null = null
  return {
    present(input: OwnerRotorInput, latest: () => OwnerRotorInput,
      isCurrentConnection: (scope: OwnerConnectionScope) => boolean,
      command: (command: Command) => Promise<void>, now = performance.now()): ExternalRotorPresentation {
      const scope = scopeOf(input), snapshot = input.lease?.snapshot
      if (!scope || !input.view.hello?.capabilities.includes('rotor.stop') || retained &&
        (retained.api !== input.api || !same(retained.scope, scope))) retained = null
      const fresh = !!(scope && clockValid(now) && input.view.connected && snapshot && input.view.snapshot === snapshot
        && input.lease!.deadline > now && snapshot.server_instance === scope.serverInstance && snapshot.workspace_id === scope.workspaceId)
      if (fresh && snapshot!.rotor.rotor_id && scope && input.view.hello?.capabilities.includes('rotor.stop')) {
        if (!retained || retained.rotorId !== snapshot!.rotor.rotor_id) retained = { api: input.api, scope: { ...scope }, rotorId: snapshot!.rotor.rotor_id }
      }
      const captured = retained
      const canStop = () => {
        const current = latest(), currentScope = scopeOf(current)
        return !!(captured && retained === captured && current.api === captured.api && currentScope && same(currentScope, captured.scope)
          && current.view.hello?.capabilities.includes('rotor.stop') && isCurrentConnection(captured.scope))
      }
      return {
        kind: 'xlog', targetKey: captured ? JSON.stringify([captured.scope, captured.rotorId]) : null,
        observation: fresh && snapshot!.rotor.connected ? { azimuthDeg: snapshot!.rotor.azimuth_deg,
          moving: snapshot!.rotor.moving, deadline: input.lease!.deadline } : null,
        observationCurrent: () => {
          const current = latest(), currentScope = scopeOf(current)
          return !!(fresh && scope && currentScope && same(scope, currentScope) && current.api === input.api && current.view.connected
            && isCurrentConnection(scope) && current.view.snapshot === snapshot && current.lease?.snapshot === snapshot
            && current.lease.deadline === input.lease!.deadline)
        },
        canStop,
        stop: captured ? async () => {
          if (!canStop()) throw new Error('The xLog rotator connection or target changed.')
          // No await between the last scope check and the independent command.
          await command({ kind: 'rotor.stop', rotor_id: captured.rotorId })
        } : null,
      }
    },
  }
}
