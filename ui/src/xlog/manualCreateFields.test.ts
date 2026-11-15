import { describe,it,expect } from 'vitest'
import { blankManualFields,manualCreateQso,validManualFields } from './manualCreateFields'
const id='12345678-1234-4234-8234-123456789abc'
const fields=()=>({...blankManualFields(Date.UTC(2024,1,29,23,59,59,123)),call:'K1ABC',band:'20m',mode:'FT8',freq:'14.074001',grid:'FN42'})
describe('manual owner create projection',()=>{
  it('retains exact MHz→Hz and millisecond UTC, multiline and case without normalization',()=>{
    const f={...fields(),grid:'fn42',comment:'line one\r\nline two\tΩ'}
    expect(manualCreateQso(id,f)).toEqual({contact_id:id,callsign:'K1ABC',band:'20m',mode:'FT8',started_at_unix_ms:Date.UTC(2024,1,29,23,59,59,123),frequency_hz:14074001,rst_sent:'',rst_received:'',grid:'fn42',comment:f.comment})
  })
  it('keeps blank Hz unknown; never substitutes 0 or radio DIGU for FT8',()=>{
    expect(manualCreateQso(id,{...fields(),freq:''})?.frequency_hz).toBeNull()
    expect(manualCreateQso(id,fields())?.mode).toBe('FT8')
  })
  it.each(['14.0740001','1e3',' 14.074','NaN','Infinity','0','1000000.000001','-1'])('refuses inexact/unsafe frequency %s',freq=>expect(manualCreateQso(id,{...fields(),freq})).toBeNull())
  it.each([['2023-02-29','12:00'],['2024-04-31','12:00'],['2024-03-31','24:00'],['2024-01-01','12:60'],['1969-12-31','23:59'],['2100-01-01','00:00:00.001']])('rejects UTC calendar/bounds %s %s',(whenDate,whenTime)=>expect(manualCreateQso(id,{...fields(),whenDate,whenTime})).toBeNull())
  it('treats a local DST gap as an ordinary valid UTC time',()=>expect(manualCreateQso(id,{...fields(),whenDate:'2024-03-31',whenTime:'02:30'})?.started_at_unix_ms).toBe(Date.UTC(2024,2,31,2,30)))
  it('requires literal owner call and grid syntax',()=>{
    for(const call of ['ß1ABC','ı1ABC','k1abc','K1',' K1ABC'])expect(manualCreateQso(id,{...fields(),call})).toBeNull()
    for(const grid of ['FN','SS00','FN42zz','FN42AB123'])expect(manualCreateQso(id,{...fields(),grid})).toBeNull()
    expect(manualCreateQso(id,{...fields(),call:'K1ABC/P',grid:'FN42ab12'})).not.toBeNull()
  })
  it('counts UTF8 decoded bytes, allows real pairs and rejects isolated UTF16',()=>{
    expect(manualCreateQso(id,{...fields(),comment:'😀'.repeat(1024)})).not.toBeNull()
    expect(manualCreateQso(id,{...fields(),comment:'😀'.repeat(1025)})).toBeNull()
    expect(manualCreateQso(id,{...fields(),rstSent:'é'.repeat(9)})).toBeNull()
    expect(validManualFields({...fields(),comment:'\ud800'})).toBe(false)
    expect(validManualFields({...fields(),comment:'\udc00'})).toBe(false)
  })
  it('preserves partial raw dates but never admits them to the wire',()=>{
    const f={...fields(),whenDate:'2024-0',whenTime:'1:'}
    expect(validManualFields(f)).toBe(true);expect(manualCreateQso(id,f)).toBeNull()
  })
})
