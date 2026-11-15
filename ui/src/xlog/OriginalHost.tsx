// SPDX-License-Identifier: GPL-3.0-only
// Original Nexus chrome and operating panes, with explicit external services.
// Do not mount the upstream App: its initialization owns a station engine.
import { useEffect, useMemo, useRef, useState } from 'react'
import { TopBar } from '../components/TopBar'
import { ModeNav } from '../components/ModeNav'
import { OperateCockpit } from '../components/OperateCockpit'
import { RotorStrip } from '../components/RotorStrip'
import { StationList } from '../components/StationList'
import { SpotsPanel } from '../components/SpotsPanel'
import { Dialog } from '../components/ui/Dialog'
import { ContactCard } from '../components/ContactCard'
import { ConfirmHost } from '../confirm'
import { ownerEditorResumeLabel } from '../features/ownerEditorMessages'
import { t } from '../i18n'
import { OperateServices, type OperateAction, type OperateServiceAdapter } from '../components/operateServices'
import { OPERATE_PANELS, usePanelLayout } from '../features/panelState'
import { allFeatureIds } from '../features/registry'
import { LogSourceProvider } from '../features/logSource'
import { useViewport } from '../useViewport'
import type { BandChannel, NeedTag, Tier } from '../types'
import { bridge, historyReads, contactReads, type Bridge } from './api'
import { useOwnerHistory } from './useOwnerHistory'
import { useOwnerContact } from './useOwnerContact'
import { rxApi, type RxApi } from './rxApi'
import { useOwnerConnection } from './useOwnerConnection'
import { useOwnerReceiver } from './useOwnerReceiver'
import { useOwnerSpectrum } from './useOwnerSpectrum'
import { spectrumApi, spectrumContextKey } from './spectrumApi'
import { ownerOperatingModel } from './ownerOperatingModel'
import { createOwnerRotor } from './ownerRotor'
import { bearingReads } from './bearingApi'
import { useOwnerBearing } from './useOwnerBearing'
import { ownerCockpitPresentation } from './ownerCockpitPresentation'
import { ownerDecodeEntries, ownerRoster } from './ownerDecodeEntries'
import { installOwnerLogBoundary, unavailableOwnerLog } from './ownerLogBoundary'
import { ownerSpotsSource } from './ownerSpots'

const NO_BAND_PLAN: BandChannel[] = []
const NO_NATIVE_NEEDS = new Map<string, NeedTag>()
const FT_MODES = new Set<Tier>(['FT8', 'FT4'])

/** Exported for the actual original-component browser fixture. Production entry
 * switches only after this host and its owner data boundaries are qualified. */
export function OriginalHost({ api = bridge, receiver = rxApi, spectrumTransport = spectrumApi.read, historyTransport = historyReads, contactTransport = contactReads, bearingTransport = bearingReads }: { api?: Bridge; receiver?: RxApi; spectrumTransport?: typeof spectrumApi.read; historyTransport?: typeof historyReads; contactTransport?: typeof contactReads; bearingTransport?: typeof bearingReads }) {
  installOwnerLogBoundary()
  const connection = useOwnerConnection(api)
  const { view, session, now } = connection
  const owner = view.paired && view.hello ? {
    server_instance: view.hello.server_instance, workspace_id: view.hello.workspace_id,
  } : null
  const fresh = !!connection.lease && view.connected && connection.lease.deadline > now
  const capabilities = view.hello?.capabilities ?? []
  const rx = useOwnerReceiver({ session, owner, connected: view.connected, fresh,
    audioCapable: capabilities.includes('audio.rx'), needsCapable: capabilities.includes('needs.query') }, receiver)
  const spectrum = useOwnerSpectrum({ session, owner: view, receiver, view: rx.view, pending: rx.pending, observation: rx.observation }, spectrumTransport)
  const receive = (start: boolean) => { spectrum.suspend(); void rx.receiving(start) }
  const decodeContext = `${session}:${spectrumContextKey(rx.view.current_context) ?? 'unavailable'}`
  const [markerHz, setMarkerHz] = useState(1500)
  const [selectedCall, setSelectedCall] = useState<string | null>(null)
  const selectionRetire = useRef<() => void>(() => {})
  const selectCall = (call: string | null) => { if(call !== selectedCall) selectionRetire.current(); setSelectedCall(call) }
  const [layout, setLayout] = useState<'classic' | 'roster'>('classic')
  const [page, setPage] = useState<'operate' | 'spots'>('operate')
  const [theme, setTheme] = useState<'dark' | 'light'>('dark')
  const [scale, setScale] = useState(1)
  const [connectionOpen, setConnectionOpen] = useState(false)
  const [notice, setNotice] = useState('')
  const panels = usePanelLayout(OPERATE_PANELS, 'xlog-original')
  useViewport(scale)
  useEffect(() => {
    document.documentElement.dataset.theme = theme
    document.documentElement.style.setProperty('--ui-zoom', String(scale))
  }, [theme, scale])
  useEffect(() => { selectCall(null); setNotice('') }, [session, decodeContext])

  const model = ownerOperatingModel({ view, lease: connection.lease, now, session,
    receive: rx.view, needsLeases: rx.leases, decoderMode: rx.mode, rxMarkerHz: markerHz })
  const entries = ownerDecodeEntries({ view: rx.view, owner, leases: rx.leases, now: rx.now,
    needsAvailable: fresh && capabilities.includes('needs.query') && !rx.pending,
    mycall: model.identity?.mycall ?? null })
  const snap = ownerCockpitPresentation(model, selectedCall)
  snap.stations = ownerRoster(entries)
  const history = useOwnerHistory(view, session, selectedCall ? { callsign: selectedCall,
    ...(snap.radio.band ? { band: snap.radio.band, exactMode: rx.mode } : {}) } : null, historyTransport)
  const contact = useOwnerContact(connection, api, history.canOpenContact, contactTransport)
  const current = useRef({ model, connection, capabilities })
  current.current = { model, connection, capabilities }
  const spotInput = { view, lease: connection.lease, session }
  const currentSpots = useRef(spotInput)
  currentSpots.current = spotInput
  const spotSource = useMemo(() => ownerSpotsSource({ view, lease: connection.lease, session },
    () => currentSpots.current, row => selectCall(row.callsign)), [view, connection.lease, session])
  const canTune = fresh && model.station?.transmitting === false && !!model.station.radio_id && capabilities.includes('station.tune')
  const rotorOwner = useRef(createOwnerRotor())
  const rotorInput = { api, view, lease: connection.lease, session }
  const currentRotorInput = useRef(rotorInput)
  currentRotorInput.current = rotorInput
  const rotor = rotorOwner.current.present(rotorInput, () => currentRotorInput.current,
    connection.isCurrentConnection, connection.command, now)
  const bearing = useOwnerBearing({ binding:{ view,session,lease:connection.lease,current:connection.isCurrentBearingIdentity },
    callsign:selectedCall,poll:connection.poll,pointEvidence:connection.pointEvidence,
    observedPoint:connection.observedPoint,retirePointObservation:connection.retirePointObservation },bearingTransport)
  selectionRetire.current = bearing.retire
  const services = useMemo<OperateServiceAdapter>(() => {
    const actions = new Set<OperateAction>(['decoder', 'rxMarker'])
    if (canTune) actions.add('radio')
    return {
      actions, tiers: rx.view.enabled || rx.pending ? new Set<Tier>([rx.mode]) : FT_MODES,
      onDeniedAction: () => setNotice('This control is not available through xLog.'),
      handlers: {
        getSpectrumRow: spectrum.getSpectrumRow,
        appVersion: async () => `${__XNEXUS_VERSION__} · ${__BUILD_ID__}`,
        setFrequency: async (dialMhz, _band, mode) => {
          const station = current.current.model.station
          if (!current.current.capabilities.includes('station.tune') || !current.current.model.fresh || !station?.radio_id || station.transmitting !== false) throw new Error('A current idle xLog radio is required.')
          await current.current.connection.command({ kind: 'station.tune', radio_id: station.radio_id,
            frequency_hz: Math.round(dialMhz * 1e6), mode })
        },
      },
    }
  }, [api, session, fresh, canTune, rx.view.enabled, rx.pending, rx.mode, spectrum.getSpectrumRow])
  const logSource = useMemo(() => unavailableOwnerLog('A current xLog history answer is unavailable.'), [session])
  const nav = useMemo(() => Object.fromEntries(allFeatureIds().map(id => [id, id === 'operate' || id === 'spots'])) as Record<ReturnType<typeof allFeatureIds>[number], boolean>, [])
  const denied = () => setNotice(model.transmit.reason)
  const selectMode = (tier: Tier) => { if (tier === 'FT8' || tier === 'FT4') rx.selectMode(tier) }
  const tune = (mhz: number, _band: string, mode: string) => {
    void services.handlers.setFrequency?.(mhz, _band, mode).catch(error => setNotice(String(error)))
  }
  const roster = <StationList stations={snap.stations} myGrid={snap.mygrid ?? ''}
    currentSlot={snap.radio.slot} activePeer={selectedCall} unreadByPeer={{}}
    needByCall={NO_NATIVE_NEEDS} onSelect={selectCall} onCall={denied}
    conversations={[]} onArchive={denied} bandActive={false} bandUnread={0} onSelectBand={denied}
    dropAfterCycles={3} grip="stations" />
  const status = <div className="now-bar now-bar-inbar" aria-label="xLog connection and receiver">
    <button className="op-btn" onClick={() => setConnectionOpen(true)}>{fresh ? 'xLog connected' : 'Connect xLog'}</button>
    {contact.savedAvailable && <button className="op-btn" disabled={!contact.resumeEnabled} onClick={() => void contact.resume()}>{ownerEditorResumeLabel()}</button>}
    <button className="op-btn" disabled={rx.pending || rx.view.enabled || !fresh || !capabilities.includes('audio.rx')}
      onClick={() => receive(true)}>Start receiving</button>
    <button className="op-btn" disabled={!rx.view.enabled && !rx.pending} onClick={() => receive(false)}>Stop receiving</button>
    <RotorStrip external={{ ...rotor, bearing: bearing.presentation }} />
    <span role="status">{rx.pollError || rx.actionError || connection.actionError || connection.pollError || notice ||
      (connection.receipt ? `xLog control: ${connection.receipt.status}${connection.receipt.message ? ` — ${connection.receipt.message}` : ''}` : rx.view.reason)}</span>
  </div>
  return <OperateServices adapter={services}><LogSourceProvider source={logSource}>
    <div className="app">
      <TopBar mycall={snap.mycall} mygrid={snap.mygrid} radio={snap.radio} link={snap.link}
        bandPlan={NO_BAND_PLAN} tier={rx.mode} onTierChange={selectMode} onSetFrequency={tune}
        onSetTxEnabled={denied} onSetTune={denied} onHaltTx={denied} onSetTxEven={denied}
        onSetTxCycleAuto={denied} onSetHoldTxFreq={denied}
        hideTxControls hideFrequencyControl ftScreen alarmCell nowBar={status} />
      <div className="shell">
        <ModeNav view={page} mode="qso" enabled={nav} tier={rx.mode}
          canSelect={selected => selected === 'operate' || selected === 'spots'}
          onSelect={selected => { if (selected === 'operate' || selected === 'spots') setPage(selected) }}
          onDigitalMode={mode => { if (mode === 'digital') setPage('operate'); else denied() }} />
        {/* Use the upstream keep-alive host: hidden collapses it and active pauses
            its polls/renderer. RX ownership and global Stop remain above it. */}
        <div className="operate-host" hidden={page !== 'operate'}>
          <OperateCockpit snap={snap} theme={theme} tier={rx.mode} services={services}
            external={{ decodes: { contextKey: decodeContext, entries },
              recent: history.recent, history: history.recall,
              spectrumResetKey: `${spectrum.resetKey}:${page}`, rotorInGlobalHeader: true }}
            onTierChange={selectMode} onSourceChange={denied}
            onTune={(hz, target) => { if (target === 'rx' && Number.isFinite(hz)) setMarkerHz(Math.max(0, Math.min(5000, hz))); else denied() }}
            onCall={denied} onSetTxLevel={denied} bandPlan={NO_BAND_PLAN} onSetFrequency={tune}
            onSetMode={denied} onSetTxEven={denied} onSetTxCycleAuto={denied} onResend={denied}
            onFreetext={denied} onLog={denied} onOverrideTx={denied} onHaltTx={denied}
            onSetTxEnabled={denied} onSetTune={denied} onSetHoldTxFreq={denied}
            roster={roster} needByCall={NO_NATIVE_NEEDS} selectedCall={selectedCall} onSelect={selectCall}
            onClearSelection={() => selectCall(null)} onOpenLogbook={(call, id) => {
              void contact.open(call, id).then(opened => { if (!opened) setNotice(t('contactCard.owner.unavailable')) })
            }}
            layoutMode={layout} onLayoutMode={setLayout} panels={panels} active={page === 'operate'} />
        </div>
        {page === 'spots' && <SpotsPanel external={spotSource} selectedCall={selectedCall} />}
      </div>
      {contact.card && <ContactCard {...contact.card} onClose={() => void contact.close()}
        connection={() => setConnectionOpen(true)}
        refresh={{ run: contact.refresh, disabled: !contact.refreshEnabled }} />}
      <ConfirmHost />
      <Dialog open={connectionOpen} onOpenChange={setConnectionOpen} title="xLog connection"
        description="Enable companion sharing in xLog, then open its pairing file here. Radio and logbook ownership stays in xLog.">
        <p role="status">{fresh ? 'Current station observations received.' : 'No current station observation.'}</p>
        <button disabled={connection.busy} onClick={() => void connection.connection('import')}>Open pairing file</button>
        <button disabled={connection.busy || !view.paired} onClick={() => void connection.connection('connect')}>Reconnect</button>
        <button disabled={connection.busy || !view.paired} onClick={() => void connection.connection('disconnect')}>Disconnect</button>
        <button disabled={!view.connected || !view.connection_token || !capabilities.includes('logbook.read')} onClick={history.refresh}>Refresh log history</button>
        <p>Frequency changes use xLog simplex tuning. Configure split operation in xLog. Transmission is unavailable in this connection.</p>
        <label>Theme <select value={theme} onChange={event => setTheme(event.target.value as 'dark' | 'light')}><option value="dark">Dark</option><option value="light">Light</option></select></label>
        <label>Scale <select value={scale} onChange={event => setScale(Number(event.target.value))}>{[0.8, 1, 1.25, 1.5, 1.75].map(value => <option key={value} value={value}>{value * 100}%</option>)}</select></label>
        <button onClick={() => setConnectionOpen(false)}>Close</button>
      </Dialog>
    </div>
  </LogSourceProvider></OperateServices>
}
