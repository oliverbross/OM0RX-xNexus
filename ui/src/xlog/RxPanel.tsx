// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import { rxApi, rxOff, type RxApi, type RxMode, type RxView } from './rxApi'
import { currentNeeds, receiveNeeds, revokeNeeds, type NeedsLeases, type OwnerIdentity } from './rxNeeds'
import { DecodeNeed } from './DecodeNeed'

export function RxPanel({ api=rxApi, connected, capable, needsCapable=false, fresh, identity, ownerIdentity }: { api?: RxApi; connected: boolean; capable: boolean; needsCapable?: boolean; fresh: boolean; identity: string; ownerIdentity?: OwnerIdentity }) {
  const [view, setView] = useState<RxView>(rxOff)
  const [mode, setMode] = useState<RxMode>('FT8')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const epoch = useRef(0)
  const action = useRef(0)
  const pendingAction = useRef(false)
  const leaseRef = useRef<NeedsLeases>(new Map())
  const [leases, setLeases] = useState<NeedsLeases>(leaseRef.current)
  const [clockVersion, refreshClock] = useState(0)
  const owner = useRef(ownerIdentity)
  owner.current = ownerIdentity
  const canShowNeeds = connected && capable && needsCapable && fresh && !busy
  const admitted = useRef(canShowNeeds)
  admitted.current = canShowNeeds
  function remember(next: NeedsLeases) { leaseRef.current = next; setLeases(next) }
  function accept(next: RxView, requestStarted: number) {
    const nextLeases = receiveNeeds(leaseRef.current, next, requestStarted, owner.current)
    remember(admitted.current && !pendingAction.current ? nextLeases : revokeNeeds(nextLeases))
    setView(next); if (next.mode) setMode(next.mode)
  }
  useEffect(() => {
    if (!canShowNeeds) remember(revokeNeeds(leaseRef.current))
  }, [canShowNeeds])
  useEffect(() => {
    const now = performance.now()
    const deadlines = [...leases.values()].map(lease => lease.deadline).filter(deadline => deadline > now)
    if (!deadlines.length) return
    const timer = setTimeout(() => refreshClock(value => value + 1), Math.min(...deadlines) - now + 1)
    return () => clearTimeout(timer)
  }, [leases, clockVersion])
  useEffect(() => {
    const generation = ++epoch.current
    pendingAction.current = false
    setView(rxOff); setError(''); setBusy(false); remember(new Map())
    if (!connected || !capable) return
    let timer: ReturnType<typeof setTimeout>
    const poll = async () => {
      const sequence = action.current
      const requestStarted = performance.now()
      try {
        const next = await api.status()
        if (epoch.current !== generation) return
        if (sequence === action.current) accept(next, requestStarted)
      } catch (cause) { if (epoch.current === generation && sequence === action.current) { remember(revokeNeeds(leaseRef.current)); setError(String(cause)) } }
      if (epoch.current === generation) timer = setTimeout(poll, 500)
    }
    void poll()
    return () => { ++epoch.current; clearTimeout(timer) }
  }, [api, identity, connected, capable])
  async function change(start: boolean) {
    const generation = epoch.current
    const sequence = ++action.current
    pendingAction.current = true
    const requestStarted = performance.now()
    setBusy(true); setError('')
    remember(revokeNeeds(leaseRef.current))
    try {
      const next = await (start ? api.start(mode) : api.stop())
      if (epoch.current === generation && sequence === action.current) accept(next, requestStarted)
    } catch (cause) { if (epoch.current === generation && sequence === action.current) { remember(revokeNeeds(leaseRef.current)); setError(String(cause)) } }
    finally { if (epoch.current === generation && sequence === action.current) { pendingAction.current = false; setBusy(false); ++action.current } }
  }
  const route = view.route
  const now = performance.now()
  return <section className="xn-rx" aria-label="Receive-only digital decoding">
    <div className="xn-feed-tools"><h2>Receive-only FT8 / FT4</h2>
      <label>Receive decoder <select aria-label="Receive decoder" disabled={busy || view.enabled} value={mode} onChange={event => setMode(event.target.value as RxMode)}><option>FT8</option><option>FT4</option></select></label>
      <button disabled={busy || view.enabled || !connected || !capable || !fresh} onClick={() => void change(true)}>Start receiving</button>
      <button disabled={!view.enabled && !busy} onClick={() => void change(false)}>Stop receiving</button>
    </div>
    <p role="status">{connected && capable ? view.reason : 'Enable RX sharing in xLog, then connect with the audio capability.'}</p>
    {error && <p role="alert" className="xn-error">{error}</p>}
    <p className="xn-hint">Configure and start DAX sharing in xLog, then choose FT8 or FT4 here. Stop receiving before changing decoder.</p>
    <p className="xn-hint">Timing uses this PC’s clock; UTC sync and DAX delay are unverified.</p>
    {connected && !needsCapable && <p className="xn-hint">Needs lookup is not available from this xLog connection. Decoded messages remain Unknown.</p>}
    {route && <p className="xn-rx-route"><strong>{route.radio_id} · slice {route.slice} · DAX {route.dax_channel}</strong> · {(route.frequency_hz / 1e6).toFixed(6)} MHz · {route.mode} · {route.sample_rate.toLocaleString()} Hz · input channel {route.selected_channel + 1}/{route.source_channels}</p>}
    {view.rows.length ? <table className="xn-spots xn-decodes" aria-label="Decoded messages"><thead><tr><th>Host time / mode</th><th>Decoded message / signal</th><th>xLog need</th></tr></thead><tbody>{view.rows.slice(-200).map(row => {
      const facts = canShowNeeds ? currentNeeds(row, view, leases, now, ownerIdentity) : null
      const reason = !needsCapable ? 'native_capability_absent' : row.needs?.owner && row.needs.authority ? 'native_stale' : row.needs?.reason_code
      return <tr key={row.decode_id}><td className="xn-mono">{new Date(row.host_period_start_unix_us / 1000).toISOString().slice(11, 19)}<small>{row.mode} · host clock</small></td><td><span className="xn-mono">{row.message}</span><small className="xn-mono">{row.snr_db} dB · {row.time_offset_seconds.toFixed(2)} s · {row.audio_frequency_hz.toFixed(0)} Hz</small></td><td><DecodeNeed facts={facts} reason={reason} remaining={(leases.get(row.decode_id)?.deadline ?? now) - now} /></td></tr>
    })}</tbody></table> : <p className="xn-empty">No decoded messages in the current receive context. A complete uninterrupted period is required.</p>}
  </section>
}
