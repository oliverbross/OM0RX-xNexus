// Synthetic-only browser verification. No Tauri host, station, credentials or hardware.
// Windows Node 20: node --experimental-websocket ui/probe/xlog-probe.mjs.
// Node 22+ needs no flag. Keeps screenshots/process evidence under .build.
import { createServer } from 'vite'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { spawn, execFileSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { resolve, dirname, join } from 'node:path'

if (process.platform !== 'win32') throw Error('This probe uses Windows process isolation verification')
const ui = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const artifacts = resolve(ui, '..', '.build', `xlog-browser-${Date.now()}`)
const profile = join(artifacts, 'edge-profile')
await mkdir(profile, { recursive: true })
const executable = process.env.XNEXUS_PROBE_BROWSER ?? 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe'
const browser = spawn(executable, ['--headless=new', '--no-first-run', '--no-default-browser-check', '--disable-background-networking', '--remote-debugging-port=0', `--user-data-dir=${profile}`, 'about:blank'], { windowsHide: true, stdio: 'ignore' })
let server, ws
const sleep = ms => new Promise(r => setTimeout(r, ms))
function syntheticBridge() {
  window.probeCalls = []
  let rx = { enabled:false, mode:null, phase:'off', reason:'Receive decoding is off.', route:null, rows:[] }
  const contact = { contact_id: '10000000-0000-4000-8000-000000000001', callsign: 'W0AW', band: '20m', mode: 'CW', started_at_unix_ms: 1700000000123, frequency_hz: 14025000, rst_sent: '599', rst_received: '599', grid: 'EM12', comment: 'Synthetic prior contact', contact_revision: 12 }
  const view = () => ({ paired: true, connected: true, endpoint: 'http://127.0.0.1:49123', hello: { application: 'OM0RX-xLog', protocol_version: 1, server_instance: 'synthetic-server', workspace_id: 'synthetic-workspace', capabilities: ['snapshot.read', 'station.tune', 'rotor.point', 'rotor.stop', 'logbook.read', 'logbook.write'] }, snapshot: { protocol_version: 1, server_instance: 'synthetic-server', workspace_id: 'synthetic-workspace', revision: 1, generated_at_unix_ms: Date.now(), max_age_ms: 10000, station: { connected: true, radio_id: 'synthetic-radio', frequency_hz: 14074000, mode: 'USB', transmitting: false }, rotor: { connected: true, rotor_id: 'synthetic-rotor', azimuth_deg: 123, moving: false }, spots: Array.from({ length: 35 }, (_, i) => ({ id: String(i), callsign: 'W' + i + 'AW', frequency_hz: 14025000 + i * 1000, mode: 'CW', observed_at_unix_ms: Date.now(), source: 'synthetic cluster', latitude: -60 + i * 3.5, longitude: -170 + i * 9.5, country: 'Fixture location', need: { status: 'needed', score: 100 - i, reasons: ['New band from xLog fixture'] } })) } })
  window.__TAURI__ = { core: { invoke: async (name, args) => {
    window.probeCalls.push({ name, args })
    if (name === 'xlog_rx_start') {
      rx = { enabled:true, mode:args.mode, phase:'receiving', reason:'Synthetic receive observation; no device was opened.', route:{ capture_id:'fixture-capture', epoch:'fixture-epoch', radio_id:'synthetic-radio', receiver_id:'synthetic-client/slice-a', mapping_generation:1, device_id:'synthetic-dax', slice:'A', dax_channel:2, frequency_hz:14074000, mode:'DIGU', source_channels:2, selected_channel:0, sample_rate:48000, sample_format:'s16le', timestamp_basis:'windows_endpoint_capture', remote_rf_latency_us:null, utc_sync_accuracy_us:null }, rows:[{ host_period_start_unix_us:1790000000000000, mode:args.mode, message:'CQ K1ABC FN42', snr_db:-12, time_offset_seconds:.2, audio_frequency_hz:1400, clock:{ timestamp_basis:'windows_endpoint_capture', host_mapping_uncertainty_us:20, calibration_uncertainty_us:10, utc_sync_accuracy_us:null, remote_rf_latency_us:null, sample_alignment_offset_ns:0, resampler_center_offset_ns:0 } }] }
      return rx
    }
    if (name === 'xlog_rx_stop') { rx = { enabled:false, mode:null, phase:'off', reason:'Receive decoding is off.', route:null, rows:[] }; return rx }
    if (name === 'xlog_rx_status') return rx
    if (name === 'xlog_outbox_list' || name === 'xlog_outbox_enqueue') return []
    if (name === 'xlog_history') return { protocol_version: 1, server_instance: 'synthetic-server', workspace_id: 'synthetic-workspace', callsign: args.callsign, generated_at_unix_ms: Date.now(), contacts: [contact], matrix: [{ band: '20m', mode: 'CW', worked: true, confirmed: true }] }
    if (name === 'xlog_command') return { request_id: args.requestId, server_instance: 'synthetic-server', workspace_id: 'synthetic-workspace', status: 'accepted', message: null }
    if (name === 'xlog_disconnect') return { paired: !args.forget, connected: false, endpoint: null, hello: null, snapshot: null }
    const next = view(); next.hello.capabilities.push('audio.rx'); return next
  } } }
}
try {
  await sleep(800)
  // Must verify the exact live executable and private-root arguments BEFORE binding CDP.
  const record = JSON.parse(execFileSync('powershell.exe', ['-NoProfile', '-Command', `Get-CimInstance Win32_Process -Filter 'ProcessId = ${browser.pid}' | Select-Object ProcessId,ExecutablePath,CommandLine | ConvertTo-Json -Compress`], { windowsHide: true, encoding: 'utf8' }))
  if (record.ProcessId !== browser.pid || record.ExecutablePath.toLowerCase() !== executable.toLowerCase() || !record.CommandLine.includes(`--user-data-dir=${profile}`) || !record.CommandLine.includes('--headless=new') || !record.CommandLine.includes('--remote-debugging-port=0')) throw Error('Private browser process could not be positively identified; stopping')
  await writeFile(join(artifacts, 'process.json'), JSON.stringify(record, null, 2))
  console.log('Verified isolated browser process', browser.pid)
  let portFile
  for (let n = 0; n < 60; n++) { try { portFile = await readFile(join(profile, 'DevToolsActivePort'), 'utf8'); break } catch { await sleep(200) } }
  if (!portFile || browser.exitCode !== null) throw Error('Verified private browser exited or did not open debugging endpoint')
  const [port, socket] = portFile.trim().split(/\r?\n/)
  ws = new WebSocket(`ws://127.0.0.1:${port}${socket}`)
  await new Promise((ok, fail) => { ws.addEventListener('open', ok, { once:true }); ws.addEventListener('error', fail, { once:true }) })
  let sequence = 0
  const pending = new Map()
  const browserErrors = []
  ws.addEventListener('message', event => { const message = JSON.parse(event.data); const entry = pending.get(message.id); if (entry) { pending.delete(message.id); message.error ? entry.reject(Error(message.error.message)) : entry.resolve(message.result) } })
  ws.addEventListener('message', event => { const message = JSON.parse(event.data); if (message.method === 'Runtime.exceptionThrown') browserErrors.push(message.params.exceptionDetails.exception?.description ?? message.params.exceptionDetails.text); if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') browserErrors.push(message.params.args.map(arg=>arg.value ?? arg.description ?? '').join(' ')) })
  const call = (method, params = {}, sessionId) => new Promise((resolve, reject) => { const id = ++sequence; const timeout = setTimeout(() => { pending.delete(id); reject(Error(`CDP timed out: ${method}`)) }, 30000); pending.set(id, { resolve: value => { clearTimeout(timeout); resolve(value) }, reject: error => { clearTimeout(timeout); reject(error) } }); ws.send(JSON.stringify({ id, method, params, sessionId })) })
  server = await createServer({ root: ui, configFile: join(ui, 'vite.config.ts'), optimizeDeps: { entries: ['xlog.html'] }, logLevel: 'warn', server: { host: '127.0.0.1', port: 0, strictPort: false } })
  await server.listen()
  console.log('Started isolated UI preview')
  const origin = `http://127.0.0.1:${server.httpServer.address().port}`
  const { targetId } = await call('Target.createTarget', { url: 'about:blank' })
  const { sessionId } = await call('Target.attachToTarget', { targetId, flatten: true })
  const page = (method, params) => call(method, params, sessionId)
  await page('Page.enable')
  await page('Runtime.enable')
  const evaluate = async expression => { const result = await page('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true }); if (result.exceptionDetails) throw Error(result.exceptionDetails.exception?.description ?? 'Browser evaluation failed'); return result.result.value }
  const fixture = `(${syntheticBridge.toString()})()`
  await page('Page.addScriptToEvaluateOnNewDocument', { source: fixture })
  const cases = [[1024,768,1],[1280,800,1],[1366,768,1],[1200,750,.8],[1200,1390,1],[3440,1440,1],[1024,768,1.75]]
  const results = []
  for (const [width,height,scale] of cases) {
    await page('Emulation.setDeviceMetricsOverride', { width,height,deviceScaleFactor:1,mobile:false })
    await page('Page.navigate', { url: `${origin}/xlog.html` })
    let ready=false
    for(let n=0;n<240;n++){if(await evaluate(`!!document.querySelector('[aria-label="Inspect W0AW"]')`)){ready=true;break}await sleep(250)}
    if(!ready)throw Error(`xLog page not ready: ${await evaluate('document.body.innerText')}`)
    await evaluate(`document.querySelector('[aria-label="Interface scale"]').value='${scale}';document.querySelector('[aria-label="Interface scale"]').dispatchEvent(new Event('change',{bubbles:true}));document.querySelector('[aria-label="Inspect W0AW"]').click()`)
    await sleep(300)
    for (const theme of ['dark','light']) {
      await evaluate(`(()=>{const label='${theme}'==='light'?'Light theme':'Dark theme';const button=[...document.querySelectorAll('button')].find(button=>button.textContent===label);button?.click()})()`)
      await sleep(100)
      const result=await evaluate(`(()=>{const main=document.querySelector('main'),stop=[...document.querySelectorAll('button')].find(x=>x.textContent==='Stop rotator'),r=stop.getBoundingClientRect(),shell=document.querySelector('.app').getBoundingClientRect();return{width:innerWidth,height:innerHeight,scale:getComputedStyle(document.documentElement).getPropertyValue('--ui-zoom'),theme:document.documentElement.dataset.theme,scrollOwner:getComputedStyle(main).overflowY,scrolls:main.scrollHeight>main.clientHeight,bodyOverflow:document.documentElement.scrollWidth>innerWidth+1,stopVisible:r.top>=0&&r.bottom<=innerHeight&&r.left>=0&&r.right<=innerWidth&&!stop.disabled,mainHeight:main.clientHeight,shellBottom:shell.bottom}})()`)
      if(result.bodyOverflow||!result.stopVisible||result.mainHeight<40||Math.abs(result.shellBottom-height)>2||result.scrollOwner!=='auto')throw Error(`Layout failure ${JSON.stringify(result)}`)
      results.push(result)
      if((width===1366||scale===1.75)){const shot=await page('Page.captureScreenshot',{format:'png'});await writeFile(join(artifacts,`${width}x${height}-${scale}-${theme}.png`),Buffer.from(shot.data,'base64'))}
    }
  }
  await page('Emulation.setDeviceMetricsOverride', { width:1366,height:768,deviceScaleFactor:1,mobile:false })
  await evaluate(`document.querySelector('[aria-label="Interface scale"]').value='1';document.querySelector('[aria-label="Interface scale"]').dispatchEvent(new Event('change',{bubbles:true}))`)
  for (const view of ['band', 'world']) {
    await evaluate(`document.querySelector('[aria-label="Station view"]').value='${view}';document.querySelector('[aria-label="Station view"]').dispatchEvent(new Event('change',{bubbles:true}))`)
    const selector = view === 'band' ? '.bandmap-track' : '.xn-world svg'
    let ready=false
    for(let n=0;n<240;n++){if(await evaluate(`!!document.querySelector('${selector}')`)){ready=true;break}await sleep(250)}
    if(!ready) {
      const shot=await page('Page.captureScreenshot',{format:'png'});await writeFile(join(artifacts,`${view}-failure.png`),Buffer.from(shot.data,'base64'))
      throw Error(`${view} view did not render; console: ${browserErrors.join(' | ')}; body: ${await evaluate('document.body.innerText')}`)
    }
    await evaluate(`document.querySelector('.xn-content').scrollTop=150`)
    await sleep(300)
    if(await evaluate(`document.documentElement.scrollWidth>innerWidth+1`))throw Error(`${view} overflowed the window`)
    const shot=await page('Page.captureScreenshot',{format:'png'});await writeFile(join(artifacts,`${view}-1366.png`),Buffer.from(shot.data,'base64'))
  }
  await evaluate(`[...document.querySelectorAll('button')].find(button=>button.textContent==='Load history from xLog').click()`)
  await sleep(150)
  if(!await evaluate(`document.querySelector('.xn-history').textContent.includes('Synthetic prior contact')`))throw Error('Owner history did not render')
  await evaluate(`[...document.querySelectorAll('button')].find(button=>button.textContent==='Edit QSO').click();document.querySelector('.xn-qso-form').scrollIntoView({block:'start'})`)
  await sleep(150)
  if(!await evaluate(`document.querySelector('.xn-qso-form input').readOnly`))throw Error('Edited identity was not protected')
  const qsoShot=await page('Page.captureScreenshot',{format:'png'});await writeFile(join(artifacts,'qso-history-1366.png'),Buffer.from(qsoShot.data,'base64'))
  await evaluate(`[...document.querySelectorAll('button')].find(button=>button.textContent==='Queue QSO correction').click()`)
  await sleep(100)
  if(!await evaluate(`window.probeCalls.some(call=>call.name==='xlog_outbox_enqueue'&&call.args.mutation.kind==='logbook.update'&&call.args.mutation.expected_contact_revision===12)`))throw Error('Correction lost its owner revision')
  if(!await evaluate(`window.probeCalls.every(call=>call.name.startsWith('xlog_'))`))throw Error('Unexpected ordinary Nexus command')
  if(browserErrors.length)throw Error(`Browser console errors: ${browserErrors.join(' | ')}`)
  await evaluate(`document.querySelector('[aria-label="Receive decoder"]').value='FT4';document.querySelector('[aria-label="Receive decoder"]').dispatchEvent(new Event('change',{bubbles:true}));[...document.querySelectorAll('button')].find(x=>x.textContent==='Start receiving').click()`)
  await sleep(200)
  if(!await evaluate(`document.querySelector('.xn-rx').textContent.includes('CQ K1ABC FN42') && window.probeCalls.some(call=>call.name==='xlog_rx_start'&&call.args.mode==='FT4')`))throw Error('RX selection or decode row did not render')
  for (const [width,height,scale] of [[1366,768,1],[1024,768,1.75]]) {
    await page('Emulation.setDeviceMetricsOverride', {width,height,deviceScaleFactor:1,mobile:false})
    await evaluate(`document.querySelector('[aria-label="Interface scale"]').value='${scale}';document.querySelector('[aria-label="Interface scale"]').dispatchEvent(new Event('change',{bubbles:true}))`)
    await sleep(150)
    await evaluate(`document.querySelector('.xn-rx').scrollIntoView({block:'start'})`)
    await sleep(100)
    const fits=await evaluate(`(()=>{const b=[...document.querySelectorAll('button')].find(x=>x.textContent==='Stop receiving'),r=b.getBoundingClientRect();return !b.disabled&&r.left>=0&&r.right<=innerWidth&&r.top>=0&&r.bottom<=innerHeight&&document.documentElement.scrollWidth<=innerWidth+1})()`)
    if(!fits)throw Error('RX controls not reachable at supported viewport/scale')
    const shot=await page('Page.captureScreenshot',{format:'png'});await writeFile(join(artifacts,`rx-${width}-${scale}.png`),Buffer.from(shot.data,'base64'))
  }
  await evaluate(`[...document.querySelectorAll('button')].find(x=>x.textContent==='Stop receiving').click()`)
  await sleep(150)
  if(!await evaluate(`window.probeCalls.some(call=>call.name==='xlog_rx_stop')&&!document.querySelector('.xn-rx').textContent.includes('CQ K1ABC FN42')`))throw Error('RX Stop did not discard decode context')
  await evaluate(`[...document.querySelectorAll('button')].find(x=>x.textContent==='Stop rotator').click()`)
  await sleep(100)
  if(!await evaluate(`window.probeCalls.some(x=>x.name==='xlog_command'&&x.args.command.kind==='rotor.stop'&&x.args.command.rotor_id==='synthetic-rotor')`))throw Error('Stop did not use observed xLog target')
  // Negative control: prove the geometry check notices an off-screen emergency control.
  if(await evaluate(`(()=>{const b=[...document.querySelectorAll('button')].find(x=>x.textContent==='Stop rotator');b.style.transform='translateY(10000px)';return b.getBoundingClientRect().bottom<=innerHeight})()`))throw Error('Layout negative control failed')
  await writeFile(join(artifacts,'results.json'),JSON.stringify(results,null,2))
  console.log(`PASS ${results.length} viewport/theme cases, RX FT4/route/rows/Stop at normal+175%, band/world views, history/correction, independent Stop dispatch and negative geometry control. Evidence: ${artifacts}`)
  await call('Browser.close')
} finally {
  ws?.close()
  await server?.close()
  // No recursive deletion or generic process-name kills. Preserve private evidence.
  if(browser.exitCode===null)browser.kill()
}
