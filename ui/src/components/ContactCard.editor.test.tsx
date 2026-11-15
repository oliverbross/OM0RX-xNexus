// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react'
import { ContactCard, ContactCardSurface, type ContactCardSurfaceProps } from './ContactCard'
import { createOwnerContactEditor, type OwnerEditorSource } from '../features/ownerContactEditor'
import type { OwnerContactDetail } from '../features/ownerContactDetail'
import type { OutboxEntry, Receipt } from '../xlog/api'
import producer from '../xlog/fixtures/contact-detail-producer.json'
import { DRAFT_KEY, newSavedDraft, saveDraft } from '../xlog/draftStore'
import { t } from '../i18n'

const native=vi.hoisted(()=>vi.fn(()=>{throw Error('Owner editor used independent native API')}))
const confirm=vi.hoisted(()=>vi.fn(async()=>false))
const normalization=vi.hoisted(()=>vi.fn((el:HTMLTextAreaElement)=>el.value.replace(/[\r\n]+/g,' ')))
vi.mock('../api',()=>({qrzLookup:native}))
vi.mock('../confirm',()=>({confirmDialog:confirm}))
vi.mock('../typedInput',()=>({oneLineTyped:normalization}))
vi.mock('../features/callbookAnswers',()=>({subscribeCallbookAnswers:native,callbookAnswer:native}))
vi.mock('../features/logSource',()=>({useLogAnswer:native}))
vi.mock('./QsoHistory',()=>({QsoHistory:native}))
vi.mock('./AwardStatus',()=>({CardAwardStatus:native}))
vi.mock('./Iota',()=>({IotaIslandPick:native,IotaReferenceNote:native}))
vi.mock('./CallCard',()=>({CallbookPhoto:native,qrzPageOf:native}))
vi.mock('./prop/ContactPath',()=>({ContactPath:native}))
vi.mock('./ui/Dialog',()=>({Dialog:({children}:{children:React.ReactNode})=><div role="dialog">{children}</div>}))
vi.mock('../focusReturn',()=>({useFocusReturn:()=>()=>{}}))

function setup(){
  const detail=JSON.parse(JSON.stringify(producer.found)) as OwnerContactDetail
  let clock=100
  const binding={token:'not-persisted',workspaceId:'workspace-A',serverInstance:'owner-A',generation:1}
  const source:OwnerEditorSource={connection:{...binding,writeGranted:true},read:{detail,deadlineMono:1000,binding}}
  const port={enqueue:vi.fn(async(mutation:OutboxEntry['queued']['mutation'],request_id:string,workspace_id:string):Promise<OutboxEntry[]>=>[
    {queued:{version:1,mutation,request_id,workspace_id},state:'Pending'}]),outbox:vi.fn(async():Promise<OutboxEntry[]>=>[]),
    deliver:vi.fn(async():Promise<Receipt>=>({request_id:'11111111-1111-4111-8111-111111111111',workspace_id:binding.workspaceId,
      server_instance:binding.serverInstance,status:'uncertain',message:null}))}
  const editor=createOwnerContactEditor({contactId:detail.contact_id,source:()=>source,port,now:()=>clock,requestId:()=>'11111111-1111-4111-8111-111111111111'})
  const props={external:{contactId:detail.contact_id,scopeKey:'read-A',read:{state:'current' as const,detail}},editor,onClose:vi.fn()}
  return {source,editor,port,props,time:(v:number)=>{clock=v}}
}
const input=(key:string)=>screen.getByLabelText(key) as HTMLInputElement
beforeEach(()=>{localStorage.clear();native.mockClear();normalization.mockClear();confirm.mockReset();confirm.mockResolvedValue(false)})
afterEach(()=>{cleanup();vi.restoreAllMocks()})

it('keeps original card geometry/classes and only the six owner correction fields editable',()=>{
  const s=setup();render(<ContactCard {...s.props}/>)
  expect(document.querySelector('form.contact-card .contact-card-actions')).toBeTruthy()
  for(const label of [t('logbook.field.date.label'),t('logbook.field.time.label'),t('logbook.field.freq.label'),
    t('logbook.field.rstSent.label'),t('logbook.field.rstRcvd.label'),t('logbook.field.grid.label')]){
    expect(input(label).readOnly).toBe(false)
  }
  for(const label of [t('logbook.field.call.label'),t('logbook.field.band.label'),t('logbook.field.mode.label'),t('logbook.field.end.label'),t('logbook.field.notes.label'),t('logbook.field.name.label')])expect(input(label).readOnly).toBe(true)
  expect(screen.queryByRole('button',{name:t('contactCard.field.add')})).toBeNull()
  expect(localStorage.length).toBe(0);expect(native).not.toHaveBeenCalled()
})
it('owner multiline paste/change and Enter preserve text, never normalize or submit',()=>{
  const s=setup();render(<ContactCard {...s.props}/>)
  const comment=input(t('logbook.field.comment.label'))
  const form=comment.closest('form')!,submit=vi.spyOn(form,'requestSubmit')
  fireEvent.paste(comment,{clipboardData:{getData:()=> 'first\nsecond'}})
  fireEvent.change(comment,{target:{value:'first\nsecond'}})
  fireEvent.keyDown(comment,{key:'Enter'})
  expect(comment.value).toBe('first\nsecond');expect(s.editor.inspect().fields?.comment).toBe('first\nsecond')
  expect(normalization).not.toHaveBeenCalled();expect(submit).not.toHaveBeenCalled()
  expect(s.port.enqueue).not.toHaveBeenCalled();expect(s.port.deliver).not.toHaveBeenCalled();expect(native).not.toHaveBeenCalled()
})
it('queues explicitly, then exposes a separate submit action; uncertain is not saved',async()=>{
  const s=setup();render(<ContactCard {...s.props}/>)
  fireEvent.change(input(t('logbook.field.comment.label')),{target:{value:'corrected'}})
  await act(async()=>{fireEvent.click(screen.getByRole('button',{name:t('contactCard.editor.queue')}))})
  expect(s.port.enqueue).toHaveBeenCalledTimes(1);expect(s.port.deliver).not.toHaveBeenCalled()
  expect(screen.getAllByText(t('contactCard.editor.queued')).length).toBeGreaterThan(0)
  await act(async()=>{fireEvent.click(screen.getByRole('button',{name:t('contactCard.editor.submit')}))})
  expect(s.port.deliver).toHaveBeenCalledTimes(1)
  expect(screen.queryByText(t('contactCard.editor.applied'))).toBeNull()
  expect(screen.getAllByText(t('contactCard.editor.uncertain')).length).toBeGreaterThan(0)
})
it('pending close and same-turn duplicate form submission cannot bypass controller admission',async()=>{
  const s=setup();let resolve!:(v:OutboxEntry[])=>void
  s.port.enqueue.mockImplementation(()=>new Promise(r=>{resolve=r}))
  render(<ContactCard {...s.props}/>);const comment=input(t('logbook.field.comment.label'))
  fireEvent.change(comment,{target:{value:'once'}})
  fireEvent.submit(comment.closest('form')!);fireEvent.submit(comment.closest('form')!)
  fireEvent.click(screen.getByRole('button',{name:t('contactCard.cancel')}))
  expect(s.port.enqueue).toHaveBeenCalledTimes(1);expect(s.props.onClose).not.toHaveBeenCalled();expect(confirm).not.toHaveBeenCalled()
  await act(async()=>resolve([]))
  fireEvent.click(screen.getByRole('button',{name:t('contactCard.cancel')}))
  expect(s.props.onClose).toHaveBeenCalledTimes(1);expect(localStorage.getItem(DRAFT_KEY)).not.toBeNull()
})
it('expiry and read-key remount retain the same draft/request while locking edits and withdrawing current facts',()=>{
  const s=setup(),view=render(<ContactCard {...s.props}/>)
  fireEvent.change(input(t('logbook.field.comment.label')),{target:{value:'keep me'}})
  const saved=localStorage.getItem(DRAFT_KEY)
  s.time(1000);view.rerender(<ContactCard {...s.props} external={{...s.props.external,scopeKey:'read-B'}}/>)
  expect(input(t('logbook.field.comment.label')).value).toBe('keep me')
  expect(input(t('logbook.field.comment.label')).readOnly).toBe(true)
  expect(localStorage.getItem(DRAFT_KEY)).toBe(saved);expect(s.port.enqueue).not.toHaveBeenCalled()
})
it('dirty close uses the original confirmation; Keep preserves and explicit Discard removes only this unqueued draft',async()=>{
  const s=setup();render(<ContactCard {...s.props}/>)
  fireEvent.change(input(t('logbook.field.grid.label')),{target:{value:'FN42'}})
  await act(async()=>fireEvent.click(screen.getByRole('button',{name:t('contactCard.cancel')})))
  expect(s.props.onClose).not.toHaveBeenCalled();expect(localStorage.getItem(DRAFT_KEY)).not.toBeNull()
  confirm.mockResolvedValue(true)
  await act(async()=>fireEvent.click(screen.getByRole('button',{name:t('contactCard.cancel')})))
  expect(s.props.onClose).toHaveBeenCalledTimes(1);expect(localStorage.getItem(DRAFT_KEY)).toBeNull()
})
it('an occupied legacy draft is never replaced by card viewing or Save',()=>{
  saveDraft(newSavedDraft());const saved=localStorage.getItem(DRAFT_KEY),s=setup()
  render(<ContactCard {...s.props}/>);fireEvent.submit(document.querySelector('form')!)
  expect(input(t('logbook.field.comment.label')).readOnly).toBe(true)
  expect(localStorage.getItem(DRAFT_KEY)).toBe(saved);expect(s.port.enqueue).not.toHaveBeenCalled()
})
it('the shared surface default preserves native one-line normalization, Enter-submit and extended editing',()=>{
  const props={call:'K1ABC',draft:{comment:'before',notes:''},fields:[],err:null,busy:false,dirty:true,canSave:true,
    notResent:[],facts:[],box:()=>null,program:()=>null,set:vi.fn(),setField:vi.fn(),setFields:vi.fn(),
    leave:async()=>{},onClose:vi.fn(),onSave:vi.fn(),returnFocus:vi.fn(),header:null,iota:null,confirmation:null,
    path:null,awards:null,recent:null,history:null} as unknown as ContactCardSurfaceProps
  render(<ContactCardSurface {...props}/>);const comment=input(t('logbook.field.comment.label'))
  const submit=vi.spyOn(comment.closest('form')!,'requestSubmit').mockImplementation(()=>{})
  fireEvent.change(comment,{target:{value:'one\ntwo'}});fireEvent.keyDown(comment,{key:'Enter'})
  expect(normalization).toHaveBeenCalledTimes(1);expect(props.set).toHaveBeenCalledWith('comment','one two')
  expect(submit).toHaveBeenCalledTimes(1);expect(input(t('logbook.field.notes.label')).readOnly).toBe(false)
  expect(screen.getByRole('button',{name:t('contactCard.field.add')})).toBeTruthy()
})
