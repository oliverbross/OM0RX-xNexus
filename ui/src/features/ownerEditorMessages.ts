import { t } from '../i18n'
export const ownerEditorResumeLabel = () => t('contactCard.editor.resume')
import type { OwnerEditorView } from './ownerContactEditor'

export function ownerEditorMessage(view:OwnerEditorView):string{
  if(view.busy)return t('contactCard.editor.pending')
  switch(view.status){
    case 'ready':case 'dirty':return t('contactCard.editor.editing')
    case 'invalid_input':return t('contactCard.editor.invalid')
    case 'occupied':case 'invalid':case 'storage_unavailable':return t('contactCard.editor.storage')
    case 'conflict':case 'foreign':case 'source_changed':return t('contactCard.editor.changed')
    case 'frozen':case 'queue_unknown':return t('contactCard.editor.queueUnknown')
    case 'queued':return t('contactCard.editor.queued')
    case 'needs_receipt':case 'receipt_invalid':return t('contactCard.editor.uncertain')
    case 'accepted':return t('contactCard.editor.accepted')
    case 'observed':return t('contactCard.editor.observed')
    case 'rejected':return t('contactCard.editor.rejected')
    case 'applied':return t('contactCard.editor.applied')
    default:return t('contactCard.editor.unavailable')
  }
}
