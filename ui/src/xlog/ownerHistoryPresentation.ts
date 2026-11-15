// SPDX-License-Identifier: GPL-3.0-only
import type { ExternalRecallRead } from '../components/RecallPanel'
import type { ExternalRecentRead } from '../components/RecentQsosPane'
import type { History, RecentContacts } from './api'
import type { OwnerReadState } from './ownerHistoryReader'

const canOpen = (contact: History['contacts'][number]) => contact.details_complete === true
  && contact.contact_revision !== null && Number.isSafeInteger(contact.contact_revision) && contact.contact_revision >= 0
export const canonicalContactId = (id: unknown): id is string => typeof id === 'string'
  && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(id)
const rowCanOpen = (contact: History['contacts'][number], contactRead?: boolean) =>
  contactRead === undefined ? canOpen(contact) : contactRead && canonicalContactId(contact.contact_id)

/** Call only with the shared host reader's currently inspected state. These
 * projections do not renew or own a read lease, nor manufacture LoggedQso. */
export function ownerRecallRead(read: OwnerReadState<History>, matchMode: boolean, contactRead?: boolean): ExternalRecallRead {
  if (read.state === 'loading') return { state: 'loading', message: 'Reading xLog history…' }
  if (read.state !== 'current') return { state: 'unavailable', message: read.reason }
  const payload = read.payload, summary = payload.history_summary
  if (!summary?.complete || summary.confirmation_policy !== 'xlog_cluster_paper_or_lotw_v1') {
    return { state: 'unavailable', message: 'Complete xLog history is unavailable.' }
  }
  const confirmationTitle = `${summary.confirmed_count} of ${summary.total} confirmed by paper QSL or LoTW in xLog.`
  return {
    state: 'current', callsign: payload.callsign, band: summary.requested_band,
    notice: `Showing ${payload.contacts.length} of ${summary.total} xLog contacts. ${confirmationTitle}`,
    confirmationTitle, unavailableMessage: 'xLog history is unavailable for the selected station.',
    history: {
      count: summary.total, confirmedCount: summary.confirmed_count,
      dupeThisBand: matchMode ? summary.requested_band_exact_mode_worked : summary.requested_band_worked,
      qsos: payload.contacts.map(contact => ({
        id: contact.contact_id, whenUnix: contact.started_at_unix_ms / 1000,
        band: contact.band, mode: contact.exact_mode ?? contact.mode,
        rstSent: contact.rst_sent, rstRcvd: contact.rst_received, comment: contact.comment,
        canOpen: rowCanOpen(contact, contactRead),
      })),
    },
  }
}

export function ownerRecentRead(read: OwnerReadState<RecentContacts>, contactRead?: boolean): ExternalRecentRead {
  if (read.state === 'loading') return { state: 'loading', message: 'Reading recent xLog contacts…' }
  if (read.state !== 'current') return { state: 'unavailable', message: read.reason }
  return { state: 'current', total: read.payload.total, rows: read.payload.contacts.map(contact => ({
    id: contact.contact_id, call: contact.callsign, band: contact.band, mode: contact.exact_mode ?? contact.mode,
    whenUnix: contact.started_at_unix_ms / 1000, canOpen: rowCanOpen(contact, contactRead),
  })) }
}
