import { afterEach,expect,it,vi } from 'vitest'
const native=vi.hoisted(()=>vi.fn(async()=>({})))
vi.mock('./api',()=>({invoke:native}))
import { bearingReads } from './bearingApi'
afterEach(()=>vi.clearAllMocks())
it('uses the single native command with exact connection token and operating call',async()=>{
  await bearingReads.read('captured-token','VK8RX/P')
  expect(native).toHaveBeenCalledExactlyOnceWith('xlog_bearing_read',{connectionToken:'captured-token',callsign:'VK8RX/P'})
})
it('propagates native rejection without retry or browser HTTP fallback',async()=>{
  native.mockRejectedValueOnce(Error('native unavailable'))
  await expect(bearingReads.read('token','VK8RX')).rejects.toThrow('native unavailable')
  expect(native).toHaveBeenCalledTimes(1)
})
