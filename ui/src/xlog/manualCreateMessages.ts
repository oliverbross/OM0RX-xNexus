import { t } from '../i18n'
import type { ManualCreateStatus } from './manualCreateController'
export function manualCreateMessage(status:ManualCreateStatus,busy:boolean):string {
  if(busy)return t('contactCard.editor.pending')
  switch(status){
    case 'ready':case 'dirty':return t('logbook.ownerCreate.instructions')
    case 'invalid_input':return t('contactCard.editor.invalid')
    case 'invalid':case 'occupied':case 'storage_unavailable':return t('contactCard.editor.storage')
    case 'foreign':case 'source_changed':case 'unavailable':return t('logbook.ownerCreate.connectionRequired')
    case 'frozen':case 'queue_unknown':return t('logbook.ownerCreate.frozen')
    case 'queued':return t('logbook.ownerCreate.queued')
    case 'needs_receipt':case 'receipt_invalid':case 'uncertain':return t('logbook.ownerCreate.uncertain')
    case 'accepted':return t('contactCard.editor.accepted')
    case 'observed':return t('contactCard.editor.observed')
    case 'rejected':return t('logbook.ownerCreate.rejected')
    case 'applied':return t('logbook.ownerCreate.applied')
  }
}
