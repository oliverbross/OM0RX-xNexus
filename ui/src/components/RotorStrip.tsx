// A one-line rotator strip for a cockpit header — the live azimuth at a glance
// plus an instant STOP, sized to sit inline beside the mode/TX badges. It carries
// its own rotctld poll (readRotator every 2 s, same cadence as RotorPane) and,
// per the rotor-pane honesty rule, renders NOTHING when no rotator answers: a
// needle with no daemon behind it would be an ornament. Optional targetCall +
// onPointAt adds a "→ CALL" one-click slew for the cockpit's selected station.
//
// ⚠️ THIS FILE IS ON THE **MIGRATED** LIST (i18n/hardcoded-strings.test.ts): the prose is in the
// catalog under `rotor.strip.*`. Its ■ buttons stop the ROTATOR and the satellite track, never a
// transmission — they are on no cockpit's stop-line census and no sweep looks for them — so
// nothing here is deferred.
//
// The units rule lands on the SKY AND THE MAST: bird names, the track's own state word, every
// azimuth in degrees and the true/magnetic marks are data, and the three annunciator plates
// below are the instrument's own vocabulary.
import { useEffect, useRef, useState, type CSSProperties } from 'react'
import { useOperateServices } from './operateServices'
import type { SatTrackStatus, SatTransponderHeld } from '../types'
import { magneticDeg } from '../grid'
import { pushToast } from '../toast'
import { t } from '../i18n'
import { pollSingleFlight } from '../singleFlight'
import type { ExternalRotorPresentation } from './rotorExternal'
import { bearingReason } from '../xlog/ownerBearingMessages'
import { Tooltip, TooltipProvider } from './ui/Tooltip'

/** The strip's annunciator plates — what the chip is, in the shortest form that fits a cockpit
 *  header bar. Instrument marks rather than sentences (the CwCockpit `SPLIT ▲` / `REC` class),
 *  named so the catalog guard reads them as the deliberate constants they are. */
const SAT_PLATE = 'SAT'
const ROTOR_PLATE = 'ROTOR'

/** The on-air abbreviation for the reciprocal heading — an invariant token, like the
 *  azimuth plates and the true/magnetic marks this strip already carries. */
const LONG_PATH = 'LP'

export interface RotorStripProps {
  /** Explicit owner observations/retained Stop; never invokes native readers. */
  external?: ExternalRotorPresentation
  /** Poll/render only while the host cockpit is the active view (defaults on). */
  active?: boolean
  /** A selected station to offer a one-click "point at" slew for. */
  targetCall?: string | null
  /** Slew the rotator toward targetCall (the host wires pointRotatorAtCall).
   *  `longPath` takes the reciprocal heading; absent or false is short path. */
  onPointAt?: (call: string, longPath?: boolean) => void
  /** Open Settings at a section id. Given one, the "not answering" chip becomes the way to
   * the rotator's model/port instead of a tooltip naming where to go looking; without one
   * (a host that cannot navigate) the chip stays the plain indicator it has always been. */
  onOpenSettings?: (target: string) => void
}

// Sized for operating distance (operator: the 16 px original was "super small").
const GLYPH = 22
const C = GLYPH / 2

// Neutral inline chip — inherits the header's text colour so it reads correctly
// in every cockpit bar (and in both themes) without a bespoke CSS class.
const chipStyle: CSSProperties = {
  font: 'inherit',
  fontSize: '0.9em',
  lineHeight: 1,
  color: 'inherit',
  background: 'transparent',
  border: '1px solid currentColor',
  borderRadius: 4,
  padding: '3px 8px',
  opacity: 0.7,
  cursor: 'pointer',
}

export function RotorStrip(props: RotorStripProps) {
  return props.external ? <OwnerRotorStrip source={props.external} /> : <NativeRotorStrip {...props} />
}

function OwnerRotorStrip({ source }: { source: ExternalRotorPresentation }) {
  const [, setNow] = useState(performance.now())
  useEffect(() => {
    const timer = setInterval(() => setNow(performance.now()), 100)
    return () => clearInterval(timer)
  }, [])
  const now = performance.now()
  const observation = source.observation
  const az = Number.isFinite(now) && now >= 0 && source.observationCurrent() && observation && observation.deadline > now
    && observation.azimuthDeg !== null && Number.isFinite(observation.azimuthDeg) && observation.azimuthDeg >= 0 && observation.azimuthDeg < 360
    ? observation.azimuthDeg : null
  const stop = source.stop, canStop = stop !== null && source.canStop()
  const bearing = source.bearing, facts = bearing?.current() ? bearing.facts : null
  const evidence = bearing?.evidence
  if (az === null && !canStop && !bearing) return null
  // Display precision only. Retained facts and the Point callback keep exact angles.
  const deg = az === null ? null : az.toFixed(1)
  const pointStatus = evidence?.settlement.kind === 'receipt' ? evidence.settlement.receipt.status
    : evidence?.settlement.kind === 'pending' ? t('rotor.owner.pending')
    : evidence?.settlement.kind === 'uncertain' ? t('rotor.owner.uncertain') : t('rotor.owner.notSent')
  const pointSummary = evidence ? t('rotor.owner.lastPoint', {
    status: pointStatus, deg: evidence.intent.command.azimuth_deg.toFixed(1),
  }) : ''
  return <span role="group" aria-label={t('rotor.strip.aria')}
    style={{ display: 'inline-flex', alignItems: 'center', flexWrap: 'wrap', gap: '0.35rem', color: 'inherit' }}>
    <span style={{ fontSize: '0.65em', letterSpacing: '0.08em', opacity: 0.55, fontWeight: 600 }} aria-hidden>{ROTOR_PLATE}</span>
    {deg !== null && <svg width={GLYPH} height={GLYPH} viewBox={`0 0 ${GLYPH} ${GLYPH}`} aria-hidden style={{ flex: '0 0 auto' }}>
      <circle cx={C} cy={C} r={C - 1} fill="none" stroke="currentColor" strokeOpacity={0.3} />
      <g transform={`rotate(${deg} ${C} ${C})`}><line x1={C} y1={C} x2={C} y2={2} stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" />
        <circle cx={C} cy={2} r={1.4} fill="currentColor" /></g>
    </svg>}
    <span className="mono" style={{ fontSize: '0.95em', fontWeight: 600, whiteSpace: 'nowrap' }}>{deg === null ? '—' : `${deg}°T`}</span>
    {bearing && <>
      <button type="button" style={chipStyle} disabled={!bearing.canRead()} onClick={() => { void bearing.read() }}>
        {t('rotor.owner.read', { call: bearing.callsign })}</button>
      {facts?.status === 'available' && facts.geometry && facts.target ? <>
        <span className="mono">{t('rotor.owner.paths', { sp: facts.geometry.short_path_deg.toFixed(1), lp: facts.geometry.long_path_deg.toFixed(1) })}</span>
        <span>{facts.target.precision === 'historical_grid_cell_center'
          ? t('rotor.owner.historical', { grid: facts.target.grid ?? '—' }) : t('rotor.owner.approximate')}</span>
      </> : <span role="status">{bearing.state === 'loading' ? t('rotor.owner.loading') : bearingReason(facts?.reason ?? null)}</span>}
      <button type="button" style={chipStyle} disabled={!bearing.canPoint()} onClick={() => { void bearing.point(false) }}
        title={t('rotor.strip.pointAt.title', { call: bearing.callsign })}>→ {bearing.callsign}</button>
      <button type="button" style={chipStyle} disabled={!bearing.canPoint()} onClick={() => { void bearing.point(true) }}
        title={t('rotor.strip.pointAtLong.title', { call: bearing.callsign })}>{LONG_PATH}</button>
      {evidence && <span role="status">
        <TooltipProvider><Tooltip side="bottom" content={<div style={{ overflowWrap: 'anywhere' }}>
          <div>{t('rotor.owner.receipt', { id: evidence.intent.requestId, status: pointStatus })}</div>
          <div className="mono">{`${evidence.intent.command.rotor_id} → ${evidence.intent.command.azimuth_deg}°T`}</div>
        </div>}>
          <button type="button" style={chipStyle} aria-label={`${pointSummary} · ${t('rotor.owner.inspect')}`}>{pointSummary}</button>
        </Tooltip></TooltipProvider>
        {bearing.observationCurrent() && evidence.observation.kind === 'snapshot' && !evidence.retired && evidence.observation.deadline > now
          ? evidence.observation.matchesTarget ? t('rotor.owner.laterMatch', { deg: evidence.observation.azimuthDeg.toFixed(1) })
            : t('rotor.owner.laterOther', { deg: evidence.observation.azimuthDeg.toFixed(1) }) : ''}
      </span>}
    </>}
    <button type="button" style={chipStyle} title={t('rotor.pane.stop.title')} aria-label={t('rotor.pane.stop.title')}
      disabled={!canStop} onClick={() => {
        if (stop && source.canStop()) void stop().catch(error => pushToast(t('rotor.stop.failed', { error: String(error) }), 'error'))
      }}>■</button>
  </span>
}

function NativeRotorStrip({ active = true, targetCall, onPointAt, onOpenSettings }: RotorStripProps) {
  const services = useOperateServices()
  const { api: { getDeclination, getSatTrackStatus, getSatTransponder, getSettings,
    readRotator, setSatTransponder, stopRotator, stopSatTrack }, can, canAction, adapter } = services
  // null = never read (no rotator / daemon down) → the strip hides itself.
  const [readAz, setAz] = useState<number | null>(null)
  const [readDeclination, setDeclination] = useState<number | null>(null)
  // Satellite auto-track owning the rotor right now (Satellites section's loop).
  // Shown so the operator knows WHY the needle is moving on its own — and so
  // the ■ button stops the LOOP, not just one slew it would immediately redo.
  const [readSatTrack, setSatTrack] = useState<SatTrackStatus | null>(null)
  // A transponder HELD with no armed track (the QO-100/park case, and any pick
  // made before AOS). The hold owns the dial — section entry and tier flips
  // now stand down for it (the sat-FT batch) — so it must be visible from the
  // operating cockpits, not only inside the Satellites section: a dial that
  // won't re-home with no visible owner is the same trust failure the track
  // chip exists for.
  const [readHeld, setHeld] = useState<SatTransponderHeld | null>(null)
  // Rotor CONFIGURED in settings (model-launched rotctld or external host) —
  // splits "no rotor in this station" (render nothing) from "configured but
  // not answering" (render a dim, honest placeholder: a configured rotor that
  // silently vanishes reads as a missing feature — operator report from the
  // FT cockpit).
  const [readConfigured, setConfigured] = useState(false)
  const [readingSource, setReadingSource] = useState(services.api)
  const current = readingSource === services.api
  const az = current ? readAz : null
  const declination = current ? readDeclination : null
  const satTrack = current ? readSatTrack : null
  const held = current ? readHeld : null
  const configured = current && readConfigured
  const sourceRef = useRef(services.api)
  sourceRef.current = services.api
  const ownsSource = () => sourceRef.current === services.api

  useEffect(() => {
    let alive = true
    setAz(null)
    setDeclination(null)
    setSatTrack(null)
    setHeld(null)
    setConfigured(false)
    setReadingSource(services.api)
    if (!active) return
    getSettings()
      .then((st) => {
        if (alive) setConfigured((st.rotatorModel ?? 0) > 0 || st.rotatorHost.trim() !== '')
      })
      .catch(() => {})
    // One single-flight poll (#335): `read_rotator` and `get_sat_transponder` take the engine
    // mutex, so a tick skips while any of the last set is still out.
    const stop = pollSingleFlight('rotor', 2_000, (owns) =>
      Promise.allSettled([
        readRotator()
          .then((v) => owns() && setAz(v))
          .catch(() => owns() && setAz(null)),
        getSatTrackStatus().then((t) => owns() && setSatTrack(t)),
        getSatTransponder().then((h) => owns() && setHeld(h)),
      ]),
    )
    getDeclination()
      .then((d) => alive && setDeclination(d))
      .catch(() => {})
    return () => {
      alive = false
      stop()
    }
  }, [active, services.api, getSettings, getDeclination, readRotator, getSatTrackStatus, getSatTransponder])

  // Is Doppler driving a radio surface at all? Read from the DTO's `mode` —
  // the engine's own per-tick answer (a pass-only track drives nothing and
  // must claim nothing). This is the app-wide ownership marker: a frequency
  // moving by itself with no visible owner is a trust failure, and the
  // rotor-less station is exactly the one with no other strip to say so.
  const dopplerInTrack =
    satTrack != null && (satTrack.mode === 'rotor+doppler' || satTrack.mode === 'doppler-only')
  // …but WHICH surface it owns keys on the DOWNLINK leg — the leg that
  // writes the dial. An uplink-only track drives only the TX (split) VFO,
  // and this chip claiming the dial for it contradicted the rail's own
  // "the dial stays yours" (round 3, defect 5).
  const dopplerOwnsDial = satTrack != null && dopplerInTrack && satTrack.dopplerDownlink

  // NO LIVE AZIMUTH. Two different stations land here — one with no rotator at
  // all (render nothing, most stations), one with a rotator configured that is
  // not answering (an honest dim placeholder, never a fake readout) — and they
  // share the thing that must never be invisible: a satellite track holding a
  // VFO, and the ■ that stops it.
  //
  // ⭐ "Configured but silent" is EXACTLY the state a mid-pass rotor give-up
  // leaves behind: the track lets the mast go and keeps the dial, running
  // Doppler to a real LOS. The ownership chip used to live only in the
  // no-rotator branch, so the operator whose rotator quit kept the moving
  // frequency and lost the app-wide sign of who owned it — the one failure the
  // chip exists to prevent — along with the only ■ outside the Satellites
  // section that could stop it.
  if (az == null) {
    const steering = satTrack != null && (satTrack.downlinkHz != null || satTrack.uplinkHz != null)
    // A transponder held with NO armed track still owns the dial (the pick
    // parked it on the downlink, and section entry / tier flips now stand down
    // for it) — QO-100-class operating and any pre-AOS pick. Same visibility
    // rule as the track chip: a dial with an invisible owner is the failure.
    const heldChip =
      held == null ? null : (
        <span
          role="group"
          aria-label={t('rotor.strip.held.aria')}
          title={t('rotor.strip.held.title', { bird: held.name })}
          style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem', color: 'inherit' }}
        >
          <span
            style={{ fontSize: '0.65em', letterSpacing: '0.08em', opacity: 0.55, fontWeight: 600 }}
            aria-hidden
          >
            {SAT_PLATE}
          </span>
          <span
            className="mono"
            style={{ fontSize: '0.9em', fontWeight: 600, whiteSpace: 'nowrap' }}
          >
            {t('rotor.strip.held.chip', { bird: held.name })}
          </span>
          <button
            type="button"
            style={chipStyle}
            aria-label={t('rotor.strip.release.aria')}
            disabled={!can('setSatTransponder')}
            onClick={() => {
              setSatTransponder(held.name, null)
                .then(() => { if (ownsSource()) setHeld(null) })
                .catch((e) =>
                  pushToast(
                    t('rotor.strip.release.failed', {
                      error: e instanceof Error ? e.message : String(e),
                    }),
                    'error',
                  ),
                )
            }}
            title={t('rotor.strip.release.title')}
          >
            ■
          </button>
        </span>
      )
    const satChip =
      satTrack == null || !dopplerInTrack ? heldChip : (
        <span
          role="group"
          aria-label={
            dopplerOwnsDial ? t('rotor.strip.doppler.dial.aria') : t('rotor.strip.doppler.tx.aria')
          }
          // The clause that says WHAT Doppler is doing is interpolated whole (the
          // `sat.badge.dopplerOnly` shape), so the sentence stays one sentence per surface
          // instead of four near-copies.
          title={
            dopplerOwnsDial
              ? t('rotor.strip.doppler.dial.title', {
                  what: steering
                    ? t('rotor.strip.doppler.dial.steering')
                    : t('rotor.strip.doppler.dial.atAos'),
                  bird: satTrack.name,
                  state: satTrack.state,
                })
              : t('rotor.strip.doppler.tx.title', {
                  what: steering
                    ? t('rotor.strip.doppler.tx.steering')
                    : t('rotor.strip.doppler.tx.atAos'),
                  bird: satTrack.name,
                  state: satTrack.state,
                })
          }
          style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem', color: 'inherit' }}
        >
          <span
            style={{ fontSize: '0.65em', letterSpacing: '0.08em', opacity: 0.55, fontWeight: 600 }}
            aria-hidden
          >
            {SAT_PLATE}
          </span>
          <span
            className="mono"
            style={{ fontSize: '0.9em', fontWeight: 600, whiteSpace: 'nowrap' }}
          >
            {dopplerOwnsDial
              ? steering
                ? t('rotor.strip.doppler.chip.dial', { bird: satTrack.name })
                : t('rotor.strip.doppler.chip.dialAtAos', { bird: satTrack.name })
              : steering
                ? t('rotor.strip.doppler.chip.tx', { bird: satTrack.name })
                : t('rotor.strip.doppler.chip.txAtAos', { bird: satTrack.name })}
          </span>
          <button
            type="button"
            style={chipStyle}
            aria-label={t('rotor.strip.trackStop.aria')}
            disabled={!can('stopSatTrack')}
            onClick={() => {
              stopSatTrack()
                .then(() => setSatTrack(null))
                .catch((e) =>
                  pushToast(
                    t('rotor.strip.trackStop.failed', {
                      error: e instanceof Error ? e.message : String(e),
                    }),
                    'error',
                  ),
                )
            }}
            title={t('rotor.strip.trackStop.title')}
          >
            ■
          </button>
        </span>
      )
    // The paired owner's emergency Stop remains reachable without an azimuth
    // reading or a fabricated Nexus rotator-settings object.
    if (adapter && can('stopRotator')) return <>
      <span className="mono">{ROTOR_PLATE} —</span>
      <button type="button" style={chipStyle} title={t('rotor.strip.stop.title')}
        onClick={() => void stopRotator().catch(e => pushToast(String(e), 'error'))}>■</button>
      {satChip}
    </>
    if (!configured) return satChip
    // The dim chip's whole job is the model/port — a rotator that is configured and silent is
    // nearly always wired to the wrong port. So where the host can navigate, the chip IS the
    // trip to those fields rather than a tooltip naming a place to go looking for. The path in
    // the text is the Rotator SECTION on the Radio tab; it was written as a child of Rig
    // Control, which it has never been.
    const lost = satTrack?.rotorLost === true
    const lostName = lost ? t('rotor.strip.lost.stopped') : t('rotor.strip.lost.silent')
    // Name the LIKELIEST cause, not just the place to look. A rotator at the wrong line rate
    // never answers and reads exactly like dead hardware, and until 1.7.0 every model was
    // handed the same 9600 — so "check the baud" is the first thing to say to the operator
    // whose SPID or Green Heron has never worked.
    const lostTitle = lost
      ? t('rotor.strip.lost.stopped.title')
      : t('rotor.strip.lost.silent.title')
    const lostStyle: CSSProperties = {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '0.3rem',
      opacity: 0.45,
      color: 'inherit',
    }
    const lostBody = (
      <>
        <span style={{ fontSize: '0.65em', letterSpacing: '0.08em', fontWeight: 600 }} aria-hidden>
          {ROTOR_PLATE}
        </span>
        <span className="mono" style={{ fontSize: '0.9em' }}>—</span>
      </>
    )
    return (
      <>
        {onOpenSettings ? (
          <button
            type="button"
            // Both are the state's own whole sentence with the "and you can click it" clause
            // interpolated after it, carrying its own separator — never two glued fragments.
            aria-label={t('rotor.strip.lost.open.aria', { state: lostName })}
            title={t('rotor.strip.lost.open.title', { detail: lostTitle })}
            // Button reset inline, the same reason chipStyle exists above: it must still read
            // as the dim indicator it replaced, not as a control this header never had.
            style={{
              ...lostStyle,
              font: 'inherit',
              background: 'transparent',
              border: 'none',
              padding: 0,
              cursor: 'pointer',
            }}
            onClick={() => onOpenSettings('rotator')}
          >
            {lostBody}
          </button>
        ) : (
          <span aria-label={lostName} title={lostTitle} style={lostStyle}>
            {lostBody}
          </span>
        )}
        {satChip}
      </>
    )
  }

  const deg = Math.round(az)
  const mag = magneticDeg(az, declination)

  return (
    <span
      role="group"
      aria-label={t('rotor.strip.aria')}
      title={
        mag != null
          ? t('rotor.strip.az.title.magnetic', { deg, mag })
          : t('rotor.strip.az.title', { deg })
      }
      style={{ display: 'inline-flex', alignItems: 'center', gap: '0.35rem', color: 'inherit' }}
    >
      <span
        style={{
          fontSize: '0.65em',
          letterSpacing: '0.08em',
          opacity: 0.55,
          fontWeight: 600,
        }}
        aria-hidden
      >
        {ROTOR_PLATE}
      </span>
      {/* Live azimuth needle — north-up, rotated clockwise by the true bearing. */}
      <svg width={GLYPH} height={GLYPH} viewBox={`0 0 ${GLYPH} ${GLYPH}`} aria-hidden style={{ flex: '0 0 auto' }}>
        <circle cx={C} cy={C} r={C - 1} fill="none" stroke="currentColor" strokeOpacity={0.3} />
        <g transform={`rotate(${deg} ${C} ${C})`}>
          <line x1={C} y1={C} x2={C} y2={2} stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" />
          <circle cx={C} cy={2} r={1.4} fill="currentColor" />
        </g>
      </svg>
      <span className="mono" style={{ fontSize: '0.95em', fontWeight: 600, whiteSpace: 'nowrap' }}>
        {deg}°T{mag != null && ` (${mag}°M)`}
      </span>
      {satTrack && (
        <span
          className="mono"
          style={{ fontSize: '0.75em', opacity: 0.8, whiteSpace: 'nowrap' }}
          // What Doppler ALSO drives is an optional clause carrying its own `; ` separator,
          // interpolated whole — the tracking sentence stays one sentence.
          title={t('rotor.strip.track.title', {
            bird: satTrack.name,
            state: satTrack.state,
            doppler: dopplerOwnsDial
              ? t('rotor.strip.track.title.dial')
              : dopplerInTrack
                ? t('rotor.strip.track.title.tx')
                : '',
          })}
        >
          {dopplerOwnsDial
            ? t('rotor.strip.track.chip.dial', { bird: satTrack.name })
            : dopplerInTrack
              ? t('rotor.strip.track.chip.uplink', { bird: satTrack.name })
              : t('rotor.strip.track.chip', { bird: satTrack.name })}
        </span>
      )}
      {!satTrack && held && (
        <span
          className="mono"
          style={{ fontSize: '0.75em', opacity: 0.8, whiteSpace: 'nowrap' }}
          title={t('rotor.strip.held.title.here', { bird: held.name })}
        >
          {t('rotor.strip.held.chip.dial', { bird: held.name })}
        </span>
      )}
      {targetCall && onPointAt && (
        <>
          <button
            type="button"
            style={chipStyle}
            onClick={() => onPointAt(targetCall, false)}
            disabled={!canAction('rotator')}
            title={t('rotor.strip.pointAt.title', { call: targetCall })}
          >
            → {targetCall}
          </button>
          {/* ⭐ LONG PATH (#338) — DESKTOP ONLY, deliberately. The browser branch above keeps
              the short-path button alone: Remote parity is deferred, and putting a control
              there would be exactly the parity work that was excluded. It is also the wrong
              surface — both testers asked from the desktop Phone/CW/FT tabs.
              A SEPARATE button rather than a modifier on the one beside it, because
              `grid.ts::azimuthTitle` already names the hazard: every bearing Nexus shows is
              short path and says so out loud, since "a chaser who assumes long path on a low
              band points the beam 180° wrong". A toggle carries a state the operator can
              misread at a glance; two buttons cannot be, and the toast names which path it
              took. `LP` is the on-air abbreviation — an invariant token like the azimuth
              plates beside it, not translated prose. */}
          <button
            type="button"
            style={chipStyle}
            onClick={() => onPointAt(targetCall, true)}
            disabled={!canAction('rotator')}
            title={t('rotor.strip.pointAtLong.title', { call: targetCall })}
          >
            {LONG_PATH}
          </button>
        </>
      )}
      <button
        type="button"
        style={chipStyle}
        disabled={!can('stopRotator')}
        onClick={() => {
          if (adapter) {
            // An owner's emergency Stop must not queue behind another service.
            const stopping = stopRotator()
            const tracking = can('stopSatTrack') ? stopSatTrack() : Promise.resolve()
            void Promise.all([stopping, tracking]).then(() => {
              if (ownsSource()) setSatTrack(null)
            }).catch(e => {
              if (ownsSource()) pushToast(t('rotor.stop.failed', { error: e instanceof Error ? e.message : String(e) }), 'error')
            })
            return
          }
          // ALWAYS stop the track first (no-op when idle): the local satTrack
          // poll is up to 2 s stale, and a bare rotor stop inside that window
          // would be undone by the loop's next 3 s tick. Belt-and-braces halt.
          ;(can('stopSatTrack') ? stopSatTrack() : Promise.resolve())
            .then(() => {
              setSatTrack(null)
              return stopRotator()
            })
            .catch((e) =>
              pushToast(
                t('rotor.stop.failed', { error: e instanceof Error ? e.message : String(e) }),
                'error',
              ),
            )
        }}
        title={t('rotor.strip.stop.title')}
      >
        ■
      </button>
    </span>
  )
}
