// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import { rxApi, rxOff, type RxApi, type RxMode, type RxView } from './rxApi'

export function RxPanel({ api=rxApi, connected, capable, fresh, identity }: { api?: RxApi; connected: boolean; capable: boolean; fresh: boolean; identity: string }) {
  const [view, setView] = useState<RxView>(rxOff)
  const [mode, setMode] = useState<RxMode>('FT8')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const epoch = useRef(0)
  const action = useRef(0)
  function accept(next: RxView) { setView(next); if (next.mode) setMode(next.mode) }
  useEffect(() => {
    const generation = ++epoch.current
    setView(rxOff); setError(''); setBusy(false)
    if (!connected || !capable) return
    let timer: ReturnType<typeof setTimeout>
    const poll = async () => {
      const sequence = action.current
      try {
        const next = await api.status()
        if (epoch.current !== generation) return
        if (sequence === action.current) accept(next)
      } catch (cause) { if (epoch.current === generation) setError(String(cause)) }
      if (epoch.current === generation) timer = setTimeout(poll, 500)
    }
    void poll()
    return () => { ++epoch.current; clearTimeout(timer) }
  }, [api, identity, connected, capable])
  async function change(start: boolean) {
    const generation = epoch.current
    const sequence = ++action.current
    setBusy(true); setError('')
    try {
      const next = await (start ? api.start(mode) : api.stop())
      if (epoch.current === generation && sequence === action.current) accept(next)
    } catch (cause) { if (epoch.current === generation && sequence === action.current) setError(String(cause)) }
    finally { if (epoch.current === generation && sequence === action.current) { setBusy(false); ++action.current } }
  }
  const route = view.route
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
    {route && <p className="xn-rx-route"><strong>{route.radio_id} · slice {route.slice} · DAX {route.dax_channel}</strong> · {(route.frequency_hz / 1e6).toFixed(6)} MHz · {route.mode} · {route.sample_rate.toLocaleString()} Hz · input channel {route.selected_channel + 1}/{route.source_channels}</p>}
    {view.rows.length ? <table className="xn-spots"><thead><tr><th>Host time / mode</th><th>Message</th><th>SNR / DT / audio Hz</th></tr></thead><tbody>{view.rows.slice(-200).map((row, index) => <tr key={`${row.host_period_start_unix_us}-${index}`}><td className="xn-mono">{new Date(row.host_period_start_unix_us / 1000).toISOString().slice(11, 19)}<small>{row.mode} · host clock</small></td><td className="xn-mono">{row.message}</td><td className="xn-mono">{row.snr_db} dB · {row.time_offset_seconds.toFixed(2)} s · {row.audio_frequency_hz.toFixed(0)} Hz</td></tr>)}</tbody></table> : <p className="xn-empty">No decoded messages in the current receive context. A complete uninterrupted period is required.</p>}
  </section>
}
