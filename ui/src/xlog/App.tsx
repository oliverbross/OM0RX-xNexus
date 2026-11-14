// SPDX-License-Identifier: GPL-3.0-only
import { useEffect, useRef, useState } from 'react'
import { useViewport } from '../useViewport'
import { QsoPanel } from './QsoPanel'
import { StationBandMap } from './StationBandMap'
import { bridge, disconnected, type Bridge, type Command, type Spot, type View } from './api'

const frequency = (hz: number | null | undefined) => hz == null ? '—' : (hz / 1_000_000).toFixed(6)
const errorText = (error: unknown) => error instanceof Error ? error.message : String(error)
const ageText = (ms: number) => ms < 1000 ? 'just now' : `${Math.floor(ms / 1000)}s ago`

export function App({ api = bridge }: { api?: Bridge }) {
  const [view, setView] = useState<View>(disconnected)
  const [error, setError] = useState('')
  const [action, setAction] = useState('')
  const [busy, setBusy] = useState(false)
  const [selectedId, select] = useState<string | null>(null)
  const [filter, setFilter] = useState<'needed' | 'all'>('needed')
  const [query, setQuery] = useState('')
  const [heading, setHeading] = useState('')
  const [theme, setTheme] = useState<'dark' | 'light'>('dark')
  const [scale, setScale] = useState(1)
  const [now, setNow] = useState(performance.now())
  const [received, setReceived] = useState({ mono: now, age: 0 })
  const [stopTarget, setStopTarget] = useState<string | null>(null)
  const [stopAllowed, setStopAllowed] = useState(false)
  const generation = useRef(0)
  useViewport(scale)

  function accept(next: View) {
    setView(next)
    if (next.snapshot) {
      setReceived({ mono: performance.now(), age: Math.max(0, Date.now() - next.snapshot.generated_at_unix_ms) })
      if (next.snapshot.rotor.rotor_id) setStopTarget(next.snapshot.rotor.rotor_id)
    }
    if (next.hello) setStopAllowed(next.hello.capabilities.includes('rotor.stop'))
    if (!next.paired) { setStopTarget(null); setStopAllowed(false) }
  }
  useEffect(() => { document.documentElement.dataset.theme = theme }, [theme])
  useEffect(() => {
    document.documentElement.style.setProperty('--ui-zoom', String(scale))
  }, [scale])
  useEffect(() => {
    const id = setInterval(() => setNow(performance.now()), 250)
    return () => clearInterval(id)
  }, [])
  useEffect(() => {
    let active = true
    api.status().then(next => { if (active) accept(next) }).catch(e => { if (active) setError(errorText(e)) })
    return () => { active = false }
  }, [api])
  useEffect(() => {
    if (!view.connected) return
    let active = true
    let timer: ReturnType<typeof setTimeout>
    const epoch = generation.current
    const poll = async () => {
      try {
        const next = await api.poll()
        if (!active || epoch !== generation.current) return
        accept(next)
        setError('')
        timer = setTimeout(poll, 1000)
      } catch (e) {
        if (!active || epoch !== generation.current) return
        setError(errorText(e))
        setView(old => ({ ...old, connected: false, snapshot: null }))
      }
    }
    timer = setTimeout(poll, 1000)
    return () => { active = false; clearTimeout(timer) }
  }, [api, view.connected])

  const snapshot = view.snapshot
  const age = received.age + Math.max(0, now - received.mono)
  const fresh = !!snapshot && view.connected && age < snapshot.max_age_ms
  const allows = (capability: string) => fresh && !!view.hello?.capabilities.includes(capability)
  const needed = snapshot?.spots.filter(spot => spot.need.status === 'needed') ?? []
  const spots = (filter === 'needed' ? needed : snapshot?.spots ?? [])
    .filter(spot => spot.callsign.toLowerCase().includes(query.trim().toLowerCase()))
    .slice().sort((a, b) => b.need.score - a.need.score || a.callsign.localeCompare(b.callsign))
  const selected = snapshot?.spots.find(spot => spot.id === selectedId)

  async function connection(kind: 'import' | 'connect' | 'disconnect' | 'forget') {
    const epoch = ++generation.current
    setBusy(true); setError(''); setAction('')
    setView(old => ({ ...old, connected: false, snapshot: null }))
    if (kind === 'import' || kind === 'connect') { setStopTarget(null); setStopAllowed(false) }
    try {
      const next = kind === 'import' ? await api.importPairing() : kind === 'connect' ? await api.connect() : await api.disconnect(kind === 'forget')
      if (epoch !== generation.current) return
      if (next) accept(next)
      else {
        const status = await api.status()
        if (epoch === generation.current) accept(status)
      }
    } catch (e) {
      if (epoch === generation.current) { setError(errorText(e)); setView(old => ({ ...old, connected: false, snapshot: null })) }
    } finally { if (epoch === generation.current) setBusy(false) }
  }
  async function command(value: Command) {
    setAction('Sending request to xLog…')
    try {
      const receipt = await api.command(value)
      setAction(`${receipt.status === 'observed' ? 'Observed by xLog' : receipt.status === 'applied' ? 'Applied by xLog' : receipt.status === 'accepted' ? 'Accepted by xLog — awaiting observation' : receipt.status === 'rejected' ? 'Rejected by xLog' : 'Outcome uncertain — check xLog before retrying'}${receipt.message ? `: ${receipt.message}` : ''}`)
    } catch (e) { setAction(`${errorText(e)}. Check xLog before retrying.`) }
  }
  const tune = (spot: Spot) => {
    const radio = snapshot?.station.radio_id
    if (radio && allows('station.tune')) void command({ kind: 'station.tune', radio_id: radio, frequency_hz: spot.frequency_hz, mode: spot.mode })
  }
  const azimuth = Number(heading)
  return <div className="app xn-shell">
    <header className="xn-header">
      <div><p className="xn-eyebrow">OM0RX STATION SUITE</p><h1>OM0RX-xNexus <span>xLog companion</span></h1></div>
      <div className="xn-header-tools">
        <label>Scale <select aria-label="Interface scale" value={scale} onChange={e => setScale(Number(e.target.value))}>{[0.8, 1, 1.25, 1.5, 1.75].map(s => <option key={s} value={s}>{s * 100}%</option>)}</select></label>
        <button onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}>{theme === 'dark' ? 'Light theme' : 'Dark theme'}</button>
      </div>
    </header>
    <div className="xn-connection" aria-label="xLog connection">
      <strong className={fresh ? 'xn-live' : 'xn-unavailable'} role="status">{fresh ? 'Connected to xLog' : view.connected ? 'Waiting for fresh observations' : 'xLog disconnected'}</strong>
      <span>{fresh ? `Updated ${ageText(age)}` : 'Station actions require a fresh xLog connection.'}</span>
      {!fresh && stopAllowed && stopTarget && <small>Stop targets the last paired rotator: {stopTarget}</small>}
      <div className="xn-connection-actions">
        <button disabled={busy} onClick={() => void connection('import')}>Open xLog pairing file</button>
        {!view.connected && view.paired && <button disabled={busy} onClick={() => void connection('connect')}>Reconnect</button>}
        {view.connected && <button onClick={() => void connection('disconnect')}>Disconnect</button>}
        {view.paired && <button disabled={busy} onClick={() => void connection('forget')}>Forget pairing</button>}
        <button className="xn-stop" disabled={!view.paired || !stopTarget || !stopAllowed} onClick={() => stopTarget && void command({ kind: 'rotor.stop', rotor_id: stopTarget })}>Stop rotator</button>
      </div>
    </div>
    <main className="xn-content">
      {error && <p role="alert" className="xn-error">{error}</p>}
      {action && <p role="status" className="xn-result">{action}</p>}
      {!view.paired && <section className="xn-onboarding" aria-label="Connect xLog"><h2>Your station, one source of truth</h2><p>Enable the companion bridge in xLog, then open its pairing file here. xLog supplies your logbook needs, cluster, radio and rotator observations.</p><p>The pairing secret stays in the native application and is forgotten when xNexus closes. This window never opens hardware or a second logbook.</p></section>}
      <section className="xn-summary" aria-label="Station observations">
        <div className="xn-card"><p className="xn-eyebrow">RADIO · {snapshot?.station.radio_id ?? 'UNAVAILABLE'}</p><strong className="xn-readout">{fresh ? frequency(snapshot?.station.frequency_hz) : '—'} <small>MHz</small></strong><p>{fresh ? `${snapshot?.station.mode ?? 'Mode unavailable'} · ${snapshot?.station.connected ? 'Connected' : 'Disconnected'}` : 'Waiting for xLog'}{fresh && snapshot?.station.transmitting === true ? ' · Transmitting' : ''}</p></div>
        <div className="xn-card"><p className="xn-eyebrow">ROTATOR · {snapshot?.rotor.rotor_id ?? 'UNAVAILABLE'}</p><strong className="xn-readout">{fresh && snapshot?.rotor.azimuth_deg != null ? `${snapshot.rotor.azimuth_deg.toFixed(1)}°` : '—'}</strong><p>{fresh ? snapshot?.rotor.connected ? snapshot.rotor.moving ? 'Moving' : 'Connected' : 'Disconnected' : 'Waiting for xLog'}</p></div>
        <div className="xn-card"><p className="xn-eyebrow">NEEDED NOW</p><strong className="xn-readout">{fresh ? needed.length : '—'}</strong><p>xLog classification · {fresh ? snapshot?.spots.length : '—'} cluster observations</p></div>
      </section>
      <section className="xn-workspace" aria-label="Station opportunities">
        <div className="xn-feed">
          <div className="xn-feed-tools"><h2>{filter === 'needed' ? 'Stations to work' : 'Cluster observations'}</h2><div><button aria-pressed={filter === 'needed'} onClick={() => setFilter('needed')}>Needed</button><button aria-pressed={filter === 'all'} onClick={() => setFilter('all')}>All spots</button></div><input aria-label="Filter callsigns" placeholder="Find callsign" value={query} onChange={e => setQuery(e.target.value)} /></div>
          {!fresh ? <p className="xn-empty">Connect to xLog to see current station opportunities. Cached or unknown data never counts as needed.</p> : spots.length === 0 ? <p className="xn-empty">{filter === 'needed' ? 'No currently needed stations in this xLog snapshot.' : 'No matching cluster observations.'}</p> : <table className="xn-spots"><thead><tr><th>Station</th><th>MHz / mode</th><th>Why work it</th><th>Source / age</th></tr></thead><tbody>{spots.map(spot => <tr key={spot.id} className={selectedId === spot.id ? 'xn-selected' : ''}><td><button aria-label={`Inspect ${spot.callsign}`} onClick={() => select(spot.id)}>{spot.callsign}</button><small>{spot.need.status}</small></td><td className="xn-mono">{frequency(spot.frequency_hz)}<small>{spot.mode}</small></td><td>{spot.need.reasons.join(' · ') || 'No need evidence available'}<small>Score {spot.need.score} from xLog</small></td><td>{spot.source}<small>{ageText(Math.max(0, Date.now() - spot.observed_at_unix_ms))}</small></td></tr>)}</tbody></table>}
        </div>
        <aside className="xn-inspector" aria-label="Selected station">
          <p className="xn-eyebrow">WORK WITH XLOG</p><h2>{selected?.callsign ?? 'Select a station'}</h2>
          {selected ? <><p className="xn-mono">{frequency(selected.frequency_hz)} MHz · {selected.mode}</p><p>{selected.need.reasons.join(' · ') || 'Need status unknown'}</p><button disabled={!allows('station.tune') || !snapshot?.station.connected || !snapshot.station.radio_id} onClick={() => tune(selected)}>Tune through xLog</button></> : <p>Choose a callsign to inspect xLog’s evidence and request a tune.</p>}
          {!view.hello?.capabilities.includes('station.tune') && <p className="xn-hint">Radio tuning is unavailable until xLog grants that capability.</p>}
          <hr /><h3>Point the antenna</h3><label>Azimuth in degrees<input type="number" min="0" max="359.9" step="0.1" value={heading} onChange={e => setHeading(e.target.value)} /></label><button disabled={!allows('rotor.point') || !snapshot?.rotor.connected || !snapshot.rotor.rotor_id || heading.trim() === '' || !Number.isFinite(azimuth) || azimuth < 0 || azimuth >= 360} onClick={() => snapshot?.rotor.rotor_id && void command({ kind: 'rotor.point', rotor_id: snapshot.rotor.rotor_id, azimuth_deg: azimuth })}>Point through xLog</button>
          <p className="xn-hint">xLog validates the target and travel limits. Stop rotator remains in the connection bar.</p>
        </aside>
      </section>
      {fresh && snapshot && <StationBandMap spots={snapshot.spots} dialHz={snapshot.station.frequency_hz} select={spot => select(spot.id)} />}
      <QsoPanel api={api} enabled={allows('logbook.write')} readEnabled={allows('logbook.read')} connected={view.connected} workspace={view.hello?.workspace_id ?? null} selected={fresh ? selected : undefined} />
      <footer className="xn-footer"><span>{view.endpoint ?? 'No pairing loaded'}</span><span>{view.hello ? `Workspace ${view.hello.workspace_id} · protocol ${view.hello.protocol_version}` : 'Nexus-derived companion · station owned by OM0RX-xLog'}</span><span>Audio and digital transmission are not enabled in this view.</span></footer>
    </main>
  </div>
}
