// SPDX-License-Identifier: GPL-3.0-only
import { setLogSource, type LogSource } from '../features/logSource'

/** Until a particular owner query is mapped with its actual completeness and
 * confirmation policy, it is unavailable. Never answer with an empty native log.
 * This global boundary also protects legacy imperative consumers; the provider
 * in OriginalHost gives hook consumers a distinct owner/session scope. */
export function unavailableOwnerLog(reason: string): LogSource {
  return {
    peek: () => undefined,
    status: () => 'failed',
    failure: () => reason,
    ask: async () => { throw new Error(reason) },
    want: () => () => {},
    follow: () => {},
    subscribe: () => () => {},
    refresh: () => {},
  }
}

const legacyBoundary = unavailableOwnerLog('This log view is not available from xLog.')
export function installOwnerLogBoundary(): void { setLogSource(legacyBoundary) }
