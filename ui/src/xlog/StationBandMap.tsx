// SPDX-License-Identifier: GPL-3.0-only
import { useMemo, useState } from 'react'
import { BandMap } from '../components/BandMap'
import { bandLabelForMhz } from '../band'
import type { SpotRow } from '../types'
import type { Spot } from './api'

function modeClass(mode: string): 'CW' | 'Phone' | 'Digital' {
  return mode.toUpperCase() === 'CW' ? 'CW' : ['SSB', 'USB', 'LSB', 'AM', 'FM'].includes(mode.toUpperCase()) ? 'Phone' : 'Digital'
}
const bandName = (spot: Spot) => spot.band?.trim().toLowerCase() || bandLabelForMhz(spot.frequency_hz / 1e6)

export function StationBandMap({ spots, dialHz, select }: { spots: Spot[]; dialHz: number | null; select: (spot: Spot) => void }) {
  const [chosenBand, setBand] = useState('')
  const [mode, setMode] = useState<'CW' | 'Phone' | 'Digital'>('CW')
  // Band identification is presentation only. Need/award/worked evidence comes exclusively from xLog.
  const { rows, owners } = useMemo(() => {
    const owners = new Map<SpotRow, Spot>()
    const rows = spots.map((spot): SpotRow => {
      const row: SpotRow = { call: spot.callsign, entity: spot.country ?? '', zone: 0, band: bandName(spot), freqMhz: spot.frequency_hz / 1e6, mode: modeClass(spot.mode), submode: spot.mode, spotter: spot.source, corroborators: [], ageSecs: Math.max(0, (Date.now() - spot.observed_at_unix_ms) / 1000), comment: spot.need.reasons.join(' · '), licensed: false }
      // The renderer returns this row object. Keep its exact owner observation;
      // call/frequency/mode are presentation fields, not a unique spot identity.
      owners.set(row, spot)
      return row
    })
    return { rows, owners }
  }, [spots])
  const bands = [...new Set(rows.map(row => row.band).filter(Boolean))]
  const dialBand = bandLabelForMhz((dialHz ?? 0) / 1e6)
  if (dialBand && !bands.includes(dialBand)) bands.unshift(dialBand)
  const band = bands.includes(chosenBand) ? chosenBand : dialBand || bands[0] || ''
  // A call may have different needs on another band/mode: scope all evidence to the rendered rows.
  const evidence = spots.filter(spot => bandName(spot) === band && modeClass(spot.mode) === mode)
  const statuses = new Map<string, Set<Spot['need']['status']>>()
  for (const spot of evidence) {
    const call = spot.callsign.toUpperCase()
    if (!statuses.has(call)) statuses.set(call, new Set())
    statuses.get(call)!.add(spot.need.status)
  }
  const callsWithStatus = (status: 'worked' | 'unknown') => new Set([...statuses].filter(([, values]) => values.size === 1 && values.has(status)).map(([call]) => call))
  const unknown = callsWithStatus('unknown')
  const worked = callsWithStatus('worked')
  return <section className="xn-band-map" aria-label="Nexus band map from xLog">
    <div className="xn-feed-tools"><h2>Nexus band map</h2><label>Band <select aria-label="Band map band" value={band} onChange={event => setBand(event.target.value)}>{bands.map(value => <option key={value}>{value}</option>)}</select></label><label>Mode <select aria-label="Band map mode" value={mode} onChange={event => setMode(event.target.value as typeof mode)}><option value="CW">CW</option><option value="Phone">Phone</option><option value="Digital">Digital</option></select></label></div>
    <p className="xn-hint">Nexus’s frequency layout, fed by xLog. Select a marker to inspect it; tuning remains an explicit xLog action.</p>
    <div className="xn-band-map-canvas"><BandMap band={band} dialMhz={(dialHz ?? 0) / 1e6} spots={rows} spotMode={mode} callMenu={null} tuneEnabled={false} workedCalls={worked} unansweredCalls={unknown} unansweredTitle="Need status unknown in xLog" spotTitle={row => `Inspect ${row.call} · ${row.freqMhz.toFixed(6)} MHz · ${row.comment}`} onWorkSpot={row => { const spot = owners.get(row); if (spot) select(spot) }} /></div>
  </section>
}
