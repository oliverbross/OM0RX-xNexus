// @vitest-environment jsdom
import { afterEach,it,expect,vi } from 'vitest'
import { useState } from 'react'
import { act,cleanup,fireEvent,render,screen } from '@testing-library/react'
import { OwnerManualCreate } from './OwnerManualCreate'
import { LogbookForm,type LogbookFormDraft } from './LogbookForm'
import { createManualCreate } from '../xlog/manualCreateController'
import type { Bridge,OutboxEntry } from '../xlog/api'
import { blankManualFields } from '../xlog/manualCreateFields'
import { confirmDialog } from '../confirm'
import { DRAFT_KEY } from '../xlog/draftStore'
import { MANUAL_CREATE_KEY } from '../xlog/manualCreateStore'
import { t } from '../i18n'
const native=vi.hoisted(()=>vi.fn(()=>{throw Error('unexpected native fallback')}))
vi.mock('../confirm',()=>({confirmDialog:vi.fn(async()=>true)}))
vi.mock('../api',()=>({logQso:native,qrzLookup:native}))
afterEach(()=>{cleanup();localStorage.clear();native.mockClear();vi.restoreAllMocks()})
function owner(){
 const port:Pick<Bridge,'enqueue'|'deliver'|'outbox'>={enqueue:vi.fn(async(mutation,request_id,workspace_id)=>[{queued:{version:1,request_id,workspace_id,mutation},state:'Pending'}] as OutboxEntry[]),deliver:vi.fn(),outbox:vi.fn(async()=>[])}
 const controller=createManualCreate({store:localStorage,port,source:()=>({workspaceId:'workspace',serverInstance:'owner',connectionToken:'memory-only',generation:1,deadline:5000,writeGranted:true}),now:()=>1,utc:()=>Date.UTC(2024,1,29,12,34,56,789)})
 return{controller,port}
}
const change=(label:string,value:string)=>fireEvent.change(screen.getByLabelText(label),{target:{value}})
it('actual original owner form preserves MHz units, millisecond UTC and raw multiline through explicit Queue',async()=>{
 localStorage.setItem(DRAFT_KEY,'retained-correction-v2')
 const h=owner();render(<OwnerManualCreate controller={h.controller} onClose={()=>{}} connection={()=>{}} />)
 expect(localStorage.getItem(MANUAL_CREATE_KEY)).toBeNull()
 change(t('logbook.field.call.label'),'K1ABC');change(t('logbook.field.band.label'),'20m');change(t('logbook.field.mode.label'),'FT8')
 const frequency=screen.getByLabelText('Freq (MHz)') as HTMLInputElement
 expect(frequency.step).toBe('0.000001');fireEvent.change(frequency,{target:{value:'14.074001'}})
 const time=screen.getByLabelText(t('logbook.field.time.label')) as HTMLInputElement
 expect(time.value).toBe('12:34:56.789');expect(time.maxLength).toBe(16)
 const comment=screen.getByLabelText(t('logbook.field.comment.label')) as HTMLTextAreaElement
 fireEvent.change(comment,{target:{value:'line one\nline two'}})
 const submit=vi.fn();comment.form!.requestSubmit=submit;fireEvent.keyDown(comment,{key:'Enter'})
 expect(comment.value).toBe('line one\nline two');expect(submit).not.toHaveBeenCalled();expect(h.port.enqueue).not.toHaveBeenCalled()
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:t('logbook.ownerCreate.queue')}))})
 expect(h.port.enqueue).toHaveBeenCalledWith({kind:'logbook.create',qso:expect.objectContaining({frequency_hz:14074001,started_at_unix_ms:Date.UTC(2024,1,29,12,34,56,789),mode:'FT8',comment:'line one\nline two'})},expect.any(String),'workspace')
 expect(localStorage.getItem(DRAFT_KEY)).toBe('retained-correction-v2');expect(h.port.deliver).not.toHaveBeenCalled();expect(native).not.toHaveBeenCalled()
})
it('unsupported original fields remain visible, disabled and explicit; no native lookup controls run',()=>{
 const h=owner();render(<OwnerManualCreate controller={h.controller} onClose={()=>{}} connection={()=>{}} />)
 const state=screen.getByLabelText(new RegExp(t('logbook.field.state.label'))) as HTMLInputElement
 expect(state.disabled).toBe(true);expect(state.closest('label')?.textContent).toContain(t('logbook.ownerCreate.unsupported'))
 expect((screen.getByRole('button',{name:'QRZ'}) as HTMLButtonElement).disabled).toBe(true)
 expect(native).not.toHaveBeenCalled();expect(h.port.enqueue).not.toHaveBeenCalled()
})
it('real original native form preserves one-line comments and Enter submission by default',()=>{
 const submit=vi.fn(),qrz=vi.fn()
 function Native(){
  const [draft,setDraft]=useState<LogbookFormDraft>({...blankManualFields(Date.UTC(2024,0,1)),name:'',qth:'',notes:'',endTime:'',state:'',txPower:'',parkTheirRef:'',parkMyRef:'',iota:'',myGrid:'',myRig:'',qslSentVia:'',qslCard:''})
  return <LogbookForm draft={draft} setField={(k,v)=>setDraft(old=>({...old,[k]:v}))} submit={submit} cancelForm={()=>{}} onQrzLookup={qrz}
   whenDateBad={false} whenTimeBad={false} endTimeBad={false} onIota={()=>{}} iotaInvalid={false} editingKey={null} qslViaLabel={()=>null} alsoStored={[]} err={null}/>
 }
 render(<Native/>);change(t('logbook.field.call.label'),'K1ABC')
 const comment=screen.getByLabelText(t('logbook.field.comment.label')) as HTMLTextAreaElement
 fireEvent.change(comment,{target:{value:'one\ntwo'}});expect(comment.value).toBe('one two')
 const request=vi.fn();comment.form!.requestSubmit=request;fireEvent.keyDown(comment,{key:'Enter'});expect(request).toHaveBeenCalledOnce()
 expect((screen.getByLabelText('Freq (MHz)') as HTMLInputElement).step).toBe('0.0001')
 expect((screen.getByLabelText(t('logbook.field.time.label')) as HTMLInputElement).maxLength).toBe(8)
 fireEvent.click(screen.getByRole('button',{name:'QRZ'}));expect(qrz).toHaveBeenCalledOnce()
})
it('close keeps dirty create bytes and correction; pending dispatch cannot close',async()=>{
 const h=owner(),close=vi.fn();render(<OwnerManualCreate controller={h.controller} onClose={close} connection={()=>{}} />)
 change(t('logbook.field.call.label'),'K1ABC');change(t('logbook.field.band.label'),'20m');change(t('logbook.field.mode.label'),'FT8')
 fireEvent.click(screen.getByRole('button',{name:t('logbook.form.close')}));expect(close).toHaveBeenCalledOnce();expect(localStorage.getItem(MANUAL_CREATE_KEY)).not.toBeNull()
 let release!:(v:OutboxEntry[])=>void;vi.mocked(h.port.enqueue).mockImplementation(()=>new Promise(resolve=>{release=resolve}))
 act(()=>{fireEvent.click(screen.getByRole('button',{name:t('logbook.ownerCreate.queue')}))})
 expect((screen.getByRole('button',{name:t('logbook.form.close')}) as HTMLButtonElement).disabled).toBe(true)
 await act(async()=>{release([])})
})

it('first-edit storage failure can be explicitly discarded and closed without erasing the correction',async()=>{
 localStorage.setItem(DRAFT_KEY,'preserved-correction')
 const h=owner(),close=vi.fn();render(<OwnerManualCreate controller={h.controller} onClose={close} connection={()=>{}} />)
 vi.spyOn(Storage.prototype,'setItem').mockImplementation(()=>{throw Error('full')})
 change(t('logbook.field.call.label'),'K1ABC')
 expect(screen.getByRole('status').textContent).toBe(t('contactCard.editor.storage'))
 expect((screen.getByRole('button',{name:t('logbook.form.close')}) as HTMLButtonElement).disabled).toBe(true)
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:t('logbook.ownerCreate.discard')}))})
 expect(confirmDialog).toHaveBeenCalled();fireEvent.click(screen.getByRole('button',{name:t('logbook.form.close')}))
 expect(close).toHaveBeenCalledOnce();expect(localStorage.getItem(DRAFT_KEY)).toBe('preserved-correction')
 expect(localStorage.getItem(MANUAL_CREATE_KEY)).toBeNull();expect(h.port.enqueue).not.toHaveBeenCalled();expect(h.port.deliver).not.toHaveBeenCalled()
})
it('storage filling exactly at freeze permits confirmed local discard and close with zero enqueue',async()=>{
 localStorage.setItem(DRAFT_KEY,'preserved-correction-at-freeze')
 const h=owner(),close=vi.fn();render(<OwnerManualCreate controller={h.controller} onClose={close} connection={()=>{}} />)
 change(t('logbook.field.call.label'),'K1ABC');change(t('logbook.field.band.label'),'20m');change(t('logbook.field.mode.label'),'FT8')
 const before=localStorage.getItem(MANUAL_CREATE_KEY)
 vi.spyOn(Storage.prototype,'setItem').mockImplementation(()=>{throw Error('full-at-freeze')})
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:t('logbook.ownerCreate.queue')}))})
 expect(h.port.enqueue).not.toHaveBeenCalled();expect(localStorage.getItem(MANUAL_CREATE_KEY)).toBe(before)
 expect(screen.getByRole('status').textContent).toBe(t('contactCard.editor.storage'))
 expect((screen.getByRole('button',{name:t('logbook.form.close')}) as HTMLButtonElement).disabled).toBe(true)
 await act(async()=>{fireEvent.click(screen.getByRole('button',{name:t('logbook.ownerCreate.discard')}))})
 expect(confirmDialog).toHaveBeenCalled();fireEvent.click(screen.getByRole('button',{name:t('logbook.form.close')}))
 expect(close).toHaveBeenCalledOnce();expect(localStorage.getItem(MANUAL_CREATE_KEY)).toBeNull()
 expect(localStorage.getItem(DRAFT_KEY)).toBe('preserved-correction-at-freeze');expect(h.port.enqueue).not.toHaveBeenCalled();expect(h.port.deliver).not.toHaveBeenCalled()
})

it('guidance appears once in ready and valid dirty states while pending and queued status remain visible',async()=>{
 const h=owner();render(<OwnerManualCreate controller={h.controller} onClose={()=>{}} connection={()=>{}} />)
 const guidance=t('logbook.ownerCreate.instructions')
 expect(screen.getAllByText(guidance)).toHaveLength(1)
 expect(screen.queryByRole('status')).toBeNull()
 change(t('logbook.field.call.label'),'K1ABC');change(t('logbook.field.band.label'),'20m');change(t('logbook.field.mode.label'),'FT8')
 expect(h.controller.view().status).toBe('dirty')
 expect(screen.getAllByText(guidance)).toHaveLength(1)
 expect(screen.queryByRole('status')).toBeNull()
 let release!:(entries:OutboxEntry[])=>void
 const rows:OutboxEntry[]=[]
 vi.mocked(h.port.enqueue).mockImplementation((mutation,request_id,workspace_id)=>{
  rows.push({queued:{version:1,mutation,request_id,workspace_id},state:'Pending'})
  return new Promise(resolve=>{release=resolve})
 })
 act(()=>{fireEvent.click(screen.getByRole('button',{name:t('logbook.ownerCreate.queue')}))})
 expect(screen.getAllByText(guidance)).toHaveLength(1)
 expect(screen.getByRole('status').textContent).toBe(t('contactCard.editor.pending'))
 await act(async()=>{release(rows)})
 expect(screen.getByRole('status').textContent).toBe(t('logbook.ownerCreate.queued'))
 expect(screen.getAllByText(guidance)).toHaveLength(1)
 expect(h.port.deliver).not.toHaveBeenCalled();expect(native).not.toHaveBeenCalled()
})
