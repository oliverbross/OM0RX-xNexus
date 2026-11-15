// SPDX-License-Identifier: GPL-3.0-only
import { t } from '../i18n'
export function bearingReason(reason: string | null) {
  switch(reason) {
    case 'origin_unavailable': return t('rotor.owner.originUnavailable')
    case 'record_too_large': case 'invalid_record': case 'logbook_unavailable': return t('rotor.owner.recordUnavailable')
    case 'reference_unavailable': case 'reference_metadata_invalid': return t('rotor.owner.referenceUnavailable')
    case 'country_ambiguous': case 'country_known_no_entity': case 'country_invalid': case 'country_missing_date': case 'country_unknown':
    case 'target_coordinates_unavailable': return t('rotor.owner.targetUnavailable')
    case 'coincident': return t('rotor.owner.coincident')
    case 'antipodal': return t('rotor.owner.antipodal')
    default: return t('rotor.owner.unavailable')
  }
}
