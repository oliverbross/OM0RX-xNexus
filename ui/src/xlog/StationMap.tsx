// SPDX-License-Identifier: GPL-3.0-only
// Reuse Nexus's bundled offline geography. No Nexus network service or coordinate lookup.
import { useMemo } from 'react'
import { geoEquirectangular, geoPath } from 'd3-geo'
import { basemap, graticule } from '../mapGeo'
import type { Spot } from './api'

export default function StationMap({ spots, selectedId, select }: { spots: Spot[]; selectedId: string | null; select: (spot: Spot) => void }) {
  const geometry = useMemo(() => {
    const projection = geoEquirectangular().fitExtent([[4, 4], [956, 476]], { type: 'Sphere' })
    const path = geoPath(projection)
    return { projection, land: path(basemap()) ?? '', grid: path(graticule()) ?? '', ocean: path({ type: 'Sphere' }) ?? '' }
  }, [])
  const positioned = spots.filter(spot => typeof spot.latitude === 'number' && Number.isFinite(spot.latitude) && Math.abs(spot.latitude) <= 90 && typeof spot.longitude === 'number' && Number.isFinite(spot.longitude) && Math.abs(spot.longitude) <= 180)
  return <section className="xn-world" aria-label="World map of xLog stations">
    <h2>Stations around the world</h2>
    <p className="xn-hint">Nexus’s offline geography with positions supplied by xLog. Select a station to inspect its evidence. {spots.length - positioned.length} observations have no supplied position.</p>
    <svg viewBox="0 0 960 480" role="group" aria-label="xLog station locations">
      <path className="xn-world-ocean" d={geometry.ocean} />
      <path className="xn-world-land" d={geometry.land} />
      <path className="xn-world-grid" d={geometry.grid} />
      {positioned.map(spot => {
        const [x, y] = geometry.projection([spot.longitude!, spot.latitude!])!
        return <g key={spot.id} role="button" tabIndex={0} aria-label={`Inspect mapped station ${spot.callsign}`} className={`xn-world-spot xn-world-${spot.need.status}${selectedId === spot.id ? ' selected' : ''}`} onClick={() => select(spot)} onKeyDown={event => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); select(spot) } }}>
          <title>{spot.callsign} · {spot.country ?? 'Country unavailable'} · {(spot.frequency_hz / 1e6).toFixed(6)} MHz · {spot.need.reasons.join(' · ') || 'Need status unknown'}</title>
          <circle cx={x} cy={y} r={selectedId === spot.id ? 8 : 5} />
          {selectedId === spot.id && <text x={x > 850 ? x - 12 : x + 12} y={y} textAnchor={x > 850 ? 'end' : 'start'}>{spot.callsign}</text>}
        </g>
      })}
    </svg>
    <p className="xn-world-legend"><span className="xn-world-needed">● Needed</span><span className="xn-world-worked">● Worked</span><span className="xn-world-unknown">● Unknown</span> — classifications come from xLog.</p>
  </section>
}
