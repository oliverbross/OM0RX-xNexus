// SPDX-License-Identifier: GPL-3.0-only
//! Bounded native-only receive worker. Never owns a device or a transmit path.
//! Polling/assembly and decoding have separate threads and a one-period mailbox.
use crate::xlog_decode::{CompleteFrame, Decoder};
#[path = "xlog_decoded_needs.rs"]
mod decoded_needs;
#[path = "xlog_spectrum.rs"]
mod spectrum;
pub use spectrum::{Cursor as SpectrumCursor, Frame as SpectrumFrame};
#[cfg(test)]
#[path = "xlog_rx_needs_transport_tests.rs"]
mod needs_transport_tests;
use decoded_needs::{
    Cache, Cadence, Candidate, Context as NeedsContext, Fact, LookupSchedule, Observation,
};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use xnexus_xlog_client::audio::{AudioBatch, AudioClient, AudioDescriptor, AudioState};
use xnexus_xlog_client::needs::{NeedsBatch, NeedsClient, NeedsQuery};
use xnexus_xlog_client::outbox::{OutboxEntry, OutboxState};
use xnexus_xlog_rx::{
    CompletePeriod, OwnerIdentity, PcmChunk, ReceiveContext, RxAssembler, RxMode,
};

/// Reconstruct on every RX start, including after reconnect or application restart.
pub fn needs_blocked_by_outbox(entries: &[OutboxEntry], workspace: &str) -> bool {
    entries.iter().any(|entry| {
        entry.queued.workspace_id == workspace && !matches!(entry.state, OutboxState::Settled(_))
    })
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum Mode {
    FT8,
    FT4,
}
impl Mode {
    fn native(self) -> RxMode {
        match self {
            Self::FT8 => RxMode::Ft8,
            Self::FT4 => RxMode::Ft4,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct Clock {
    pub timestamp_basis: String,
    pub host_mapping_uncertainty_us: u32,
    pub calibration_uncertainty_us: u32,
    pub utc_sync_accuracy_us: Option<u32>,
    pub remote_rf_latency_us: Option<i64>,
    pub sample_alignment_offset_ns: u64,
    pub resampler_center_offset_ns: u64,
}
#[derive(Clone, Serialize)]
pub struct Row {
    pub decode_id: String,
    pub context: NeedsContext,
    pub sender_callsign: Option<String>,
    pub needs: decoded_needs::View,
    #[serde(skip)]
    fact: Option<Fact>,
    #[serde(skip)]
    needs_fallback: &'static str,
    pub host_period_start_unix_us: u64,
    pub mode: Mode,
    pub message: String,
    pub snr_db: i32,
    pub time_offset_seconds: f32,
    pub audio_frequency_hz: f32,
    pub clock: Clock,
}
impl Row {
    fn candidate(&self) -> Option<Candidate> {
        Some(Candidate {
            decode_id: self.decode_id.clone(),
            callsign: self.sender_callsign.clone()?,
            context: self.context.clone(),
        })
    }
    fn refresh_needs(&mut self) {
        match self.fact.as_ref().and_then(Fact::view) {
            Some(view) => self.needs = view,
            None => {
                if self.fact.is_some() {
                    self.needs_fallback = "native_stale";
                }
                self.fact = None;
                self.needs = decoded_needs::View::unknown(self.needs_fallback);
            }
        }
    }
}
#[derive(Clone, Serialize)]
pub struct View {
    pub enabled: bool,
    pub mode: Option<Mode>,
    pub phase: String,
    pub reason: String,
    pub route: Option<AudioDescriptor>,
    /// Current receive authority, independent of whether a station has decoded.
    pub current_context: Option<NeedsContext>,
    pub rows: Vec<Row>,
}
impl Default for View {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: None,
            phase: "off".into(),
            reason: "Receive decoding is off.".into(),
            route: None,
            current_context: None,
            rows: vec![],
        }
    }
}
struct State {
    view: View,
    epoch: u64,
    authority: Option<Arc<AudioBatch>>,
    needs_blocked: Arc<AtomicBool>,
    spectrum: Option<spectrum::Published>,
    spectrum_mailbox: Arc<spectrum::Mailbox>,
    spectrum_identity: (String, String, String),
}
impl State {
    fn refresh_needs(&mut self) {
        let blocked = self.needs_blocked.load(Ordering::Acquire);
        for row in &mut self.view.rows {
            if blocked {
                row.fact = None;
                row.needs = decoded_needs::View::unknown("native_write_pending");
            } else {
                row.refresh_needs();
            }
        }
    }
    fn reset(&mut self, phase: &str, reason: &str) {
        self.epoch = self.epoch.wrapping_add(1);
        self.authority = None;
        self.spectrum = None;
        self.spectrum_mailbox.clear();
        self.view.route = None;
        self.view.current_context = None;
        self.view.rows.clear();
        self.view.phase = phase.into();
        self.view.reason = reason.into();
    }
    fn expire(&mut self) -> bool {
        if self
            .authority
            .as_ref()
            .is_some_and(|batch| !batch.is_fresh())
        {
            self.reset(
                "waiting",
                "Audio authority expired; waiting for a fresh complete period.",
            );
            true
        } else {
            false
        }
    }
    fn accepts(&mut self, epoch: u64) -> bool {
        self.expire();
        self.view.enabled
            && self.epoch == epoch
            && self
                .authority
                .as_ref()
                .is_some_and(|batch| batch.is_fresh())
    }
    fn contains(&mut self, candidate: &Candidate) -> bool {
        !self.needs_blocked.load(Ordering::Acquire)
            && self.accepts(candidate.context.worker_epoch)
            && self.view.route.as_ref() == Some(&candidate.context.route)
            && self.view.mode == Some(candidate.context.decoder_mode)
            && self.view.rows.iter().any(|row| {
                row.decode_id == candidate.decode_id
                    && row.sender_callsign.as_deref() == Some(candidate.callsign.as_str())
                    && row.context == candidate.context
            })
    }
    fn attach(&mut self, candidate: &Candidate, fact: Fact) -> bool {
        if !self.contains(candidate) || !fact.matches(candidate) {
            return false;
        }
        let Some(view) = fact.view() else {
            return false;
        };
        if let Some(row) = self
            .view
            .rows
            .iter_mut()
            .find(|row| row.decode_id == candidate.decode_id)
        {
            row.fact = Some(fact);
            row.needs = view;
            return true;
        }
        false
    }
}
pub struct Worker {
    state: Arc<Mutex<State>>,
    cancel: Arc<AtomicBool>,
    needs_wake: Option<mpsc::SyncSender<()>>,
    needs_observation: Arc<Observation>,
    spectrum_wake: mpsc::SyncSender<()>,
    spectrum_thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}
struct Job {
    epoch: u64,
    period: CompletePeriod,
}
static ACTIVE: AtomicBool = AtomicBool::new(false);
static NEXT_WORKER: AtomicU64 = AtomicU64::new(1);
struct Lease;
impl Drop for Lease {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
    }
}

impl Worker {
    pub fn start(
        audio: AudioClient,
        server: String,
        workspace: String,
        mode: Mode,
    ) -> Result<Self, String> {
        Self::start_with_needs(audio, server, workspace, mode, None)
    }
    pub fn start_with_needs(
        audio: AudioClient,
        server: String,
        workspace: String,
        mode: Mode,
        needs: Option<NeedsClient>,
    ) -> Result<Self, String> {
        Self::start_with_needs_gate(
            audio,
            server,
            workspace,
            mode,
            needs,
            Arc::new(AtomicBool::new(false)),
        )
    }
    pub fn start_with_needs_gate(
        mut audio: AudioClient,
        server: String,
        workspace: String,
        mode: Mode,
        needs: Option<NeedsClient>,
        needs_blocked: Arc<AtomicBool>,
    ) -> Result<Self, String> {
        if ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(
                "The previous decoder is still stopping. Try receiving again shortly.".into(),
            );
        }
        let lease = Arc::new(Lease);
        let counter = NEXT_WORKER
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
            .map_err(|_| "Receive worker identity exhausted")?;
        let stamp = u64::try_from(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| "Invalid host clock")?
                .as_micros(),
        )
        .map_err(|_| "Receive worker clock identity exhausted")?;
        // At most 43 ASCII bytes even at the integer bounds; descriptor text is never in an ID.
        let worker_instance = format!("w{:x}_{:x}_{:x}", std::process::id(), stamp, counter);
        let spectrum_mailbox = Arc::new(spectrum::Mailbox::default());
        let state = Arc::new(Mutex::new(State {
            view: View {
                enabled: true,
                mode: Some(mode),
                phase: "waiting".into(),
                reason: "Waiting for xLog RX sharing and a complete period.".into(),
                ..View::default()
            },
            epoch: 1,
            authority: None,
            needs_blocked,
            spectrum: None,
            spectrum_mailbox: spectrum_mailbox.clone(),
            spectrum_identity: (worker_instance.clone(), server.clone(), workspace.clone()),
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::sync_channel::<Job>(1);
        // Only a coalesced wake token can be pending, never an unbounded batch queue.
        let (needs_wake, needs_rx) = mpsc::sync_channel::<()>(1);
        let observation_wake = needs_wake.clone();
        let needs_observation = Arc::new(Observation::default());
        let needs_available = needs.is_some();
        if let Some(mut client) = needs {
            let observation = needs_observation.clone();
            spawn_lookup(
                move |queries| {
                    let remaining = client.remaining_authority_ms();
                    if remaining == 0 {
                        // Expiry is not session retirement. Keep the bounded
                        // pending group until a fresh main-poll observation.
                        return Lookup::Deferred;
                    }
                    if !observation.admit(remaining) {
                        return Lookup::Deferred;
                    }
                    Lookup::Attempt(client.query(queries).map_err(|error| {
                        use xnexus_xlog_client::Error;
                        if matches!(
                            error,
                            Error::Disconnected
                                | Error::IdentityChanged
                                | Error::Incompatible
                                | Error::Capability
                                | Error::Http(401 | 403 | 404)
                        ) {
                            LookupFailure::Retired
                        } else {
                            LookupFailure::Retry
                        }
                    }))
                },
                state.clone(),
                cancel.clone(),
                needs_rx,
                lease.clone(),
            )
            .map_err(|_| "Cannot start decoded-station lookup worker")?;
        }
        let decode_state = state.clone();
        let decode_cancel = cancel.clone();
        let decode_lease = lease.clone();
        let decode_server = server.clone();
        let decode_workspace = workspace.clone();
        let spectrum_worker_instance = worker_instance.clone();
        let decode_started = std::thread::Builder::new().name("xlog-rx-decode".into()).spawn(move || {
            let _lease = decode_lease;
            let mut decoder = Decoder::default();
            let mut last_epoch = None;
            let mut ordinal = 0u64;
            while !decode_cancel.load(Ordering::Acquire) {
                let job = match rx.recv_timeout(Duration::from_millis(100)) {
                    Ok(job) => job,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(_) => break,
                };
                if !decode_state.lock().is_ok_and(|mut state| state.accepts(job.epoch)) { continue; }
                if last_epoch != Some(job.epoch) { decoder.reset(); last_epoch = Some(job.epoch); }
                let host_time = job.period.host_period_start_unix_us;
                let result = CompleteFrame::from_period(job.period).and_then(|frame| decoder.decode(frame));
                if decode_cancel.load(Ordering::Acquire) { break; }
                let Ok(mut state) = decode_state.lock() else { break; };
                if !state.accepts(job.epoch) { continue; }
                match result {
                    Ok(period) => {
                        let Some(route) = state.view.route.clone() else { continue; };
                        let row_context = NeedsContext { worker_instance:worker_instance.clone(), worker_epoch:job.epoch, server_instance:decode_server.clone(), workspace_id:decode_workspace.clone(), route, decoder_mode:mode };
                        let clock = Clock { timestamp_basis:period.clock.timestamp_basis.into(), host_mapping_uncertainty_us:period.clock.host_mapping_uncertainty_us, calibration_uncertainty_us:period.clock.calibration_uncertainty_us, utc_sync_accuracy_us:period.clock.utc_sync_accuracy_us, remote_rf_latency_us:period.clock.remote_rf_latency_us, sample_alignment_offset_ns:period.clock.sample_alignment_offset_ns, resampler_center_offset_ns:period.clock.resampler_center_offset_ns };
                        state.view.phase = "receiving".into();
                        state.view.reason = "Receiving xLog audio. Host clock timing; UTC synchronization and RF latency are unknown.".into();
                        for row in period.rows.into_iter().take(100) {
                            if row.message.len() > 512 || !row.time_offset_seconds.is_finite() || !row.audio_frequency_hz.is_finite() { continue; }
                            let Some(next) = ordinal.checked_add(1) else { state.reset("error", "Decode identity exhausted"); decode_cancel.store(true, Ordering::Release); break; };
                            ordinal = next;
                            let Some(decode_id) = decoded_needs::decode_id(&worker_instance, job.epoch, host_time, ordinal) else { state.reset("error", "Decode identity invalid"); decode_cancel.store(true, Ordering::Release); break; };
                            let sender_callsign = decoded_needs::literal_sender(&row.message);
                            let needs_fallback = if sender_callsign.is_none() { "native_not_literal_sender" } else if needs_available { "native_unclassified" } else { "native_capability_absent" };
                            state.view.rows.push(Row { decode_id, context:row_context.clone(), sender_callsign, needs:decoded_needs::View::unknown(needs_fallback), fact:None, needs_fallback, host_period_start_unix_us:host_time, mode, message:row.message, snr_db:row.snr_db, time_offset_seconds:row.time_offset_seconds, audio_frequency_hz:row.audio_frequency_hz, clock:clock.clone() });
                        }
                        let extra = state.view.rows.len().saturating_sub(200);
                        state.view.rows.drain(..extra);
                        let _ = needs_wake.try_send(());
                    }
                    Err(error) => state.reset("waiting", &format!("Decode discarded: {error}")),
                }
            }
        });
        if decode_started.is_err() {
            cancel.store(true, Ordering::Release);
            return Err("Cannot start receive decoder worker".into());
        }
        let (spectrum_wake, spectrum_rx) = mpsc::sync_channel(1);
        let spectrum_thread = match spectrum::spawn(
            state.clone(),
            cancel.clone(),
            lease.clone(),
            spectrum_mailbox.clone(),
            spectrum_rx,
        ) {
            Ok(thread) => thread,
            Err(_) => {
                cancel.store(true, Ordering::Release);
                return Err("Cannot start receive spectrum worker".into());
            }
        };
        let poll_spectrum_wake = spectrum_wake.clone();
        let poll_state = state.clone();
        let poll_cancel = cancel.clone();
        let started = std::thread::Builder::new().name("xlog-rx-read".into()).spawn(move || {
            let _lease = lease;
            let mut assembler = RxAssembler::default();
            let mut spectrum = spectrum::Producer::new(spectrum_mailbox, poll_spectrum_wake);
            let mut epoch = 1;
            while !poll_cancel.load(Ordering::Acquire) {
                {
                    let Ok(mut state) = poll_state.lock() else { break; };
                    state.expire();
                    if state.epoch != epoch { assembler.reset(); spectrum.reset(); epoch = state.epoch; }
                }
                let batch = match audio.poll() {
                    Ok(batch) => Arc::new(batch),
                    Err(error) => {
                        if let Ok(mut state) = poll_state.lock() { state.reset("error", &format!("RX stopped: {error}. Start receiving again when xLog RX sharing is ready.")); state.view.enabled = false; }
                        break;
                    }
                };
                if poll_cancel.load(Ordering::Acquire) { break; }
                let running = batch.state == AudioState::Running && batch.is_fresh();
                {
                    let Ok(mut state) = poll_state.lock() else { break; };
                    if batch.reset || !running || state.epoch != epoch {
                        assembler.reset();
                        spectrum.reset();
                        state.reset("waiting", batch.reason.as_deref().unwrap_or("Audio route changed; waiting for a complete period."));
                    }
                    epoch = state.epoch;
                    if running { state.view.route = batch.descriptor.clone(); state.authority = Some(batch.clone()); }
                }
                if let Some(route) = batch.descriptor.as_ref().filter(|_| running) {
                    let context = context(route, &server, &workspace, mode);
                    let spectrum_context = NeedsContext { worker_instance:spectrum_worker_instance.clone(), worker_epoch:epoch,
                        server_instance:server.clone(), workspace_id:workspace.clone(), route:route.clone(), decoder_mode:mode };
                    for chunk in &batch.chunks {
                        if poll_cancel.load(Ordering::Acquire) || !batch.is_fresh() { break; }
                        if let Some(end) = chunk.first_frame.checked_add(chunk.samples.len() as u64) { spectrum.input(batch.clone(), end); }
                        match assembler.push_observed(&context, PcmChunk { sequence:chunk.sequence, first_frame:chunk.first_frame, sample0_unix_us:chunk.sample0_unix_us, calibration_uncertainty_us:chunk.calibration_uncertainty_us, samples:&chunk.samples },
                            |span| spectrum.push(spectrum_context.clone(), span)) {
                            Ok(Some(period)) => match tx.try_send(Job { epoch, period }) {
                                Ok(()) => {},
                                Err(mpsc::TrySendError::Full(_)) => {
                                    assembler.reset();
                                    spectrum.reset();
                                    if let Ok(mut state) = poll_state.lock() { state.reset("waiting", "Decoder backlog discarded; waiting for a complete period."); epoch = state.epoch; }
                                    break;
                                },
                                Err(mpsc::TrySendError::Disconnected(_)) => {
                                    if let Ok(mut state) = poll_state.lock() { state.reset("error", "The native decoder exited. Start receiving again to recover."); state.view.enabled = false; }
                                    poll_cancel.store(true, Ordering::Release);
                                    break;
                                }
                            },
                            Ok(None) => {},
                            Err(error) => {
                                spectrum.reset();
                                if let Ok(mut state) = poll_state.lock() { state.reset("waiting", &format!("Audio continuity lost: {error:?}. Waiting for a complete period.")); epoch = state.epoch; }
                                break;
                            }
                        }
                    }
                }
                // Small batches are drained promptly without a busy HTTP loop.
                std::thread::sleep(Duration::from_millis(if batch.chunks.len() == 32 { 5 } else { 40 }));
            }
            audio.disconnect();
            poll_cancel.store(true, Ordering::Release);
        });
        if started.is_err() {
            cancel.store(true, Ordering::Release);
            return Err("Cannot start receive polling worker".into());
        }
        Ok(Self {
            state,
            cancel,
            needs_wake: Some(observation_wake),
            needs_observation,
            spectrum_wake,
            spectrum_thread: Mutex::new(Some(spectrum_thread)),
        })
    }
    /// A main-client snapshot observation wakes the independent lane; it neither
    /// polls the owner again nor extends any captured fact lease.
    pub fn needs_snapshot_observed(&self, generated_at_unix_ms: u64) {
        self.needs_observation.observe(generated_at_unix_ms);
        if let Some(wake) = &self.needs_wake {
            let _ = wake.try_send(());
        }
    }
    pub fn view(&self) -> Result<View, String> {
        let mut state = self.state.lock().map_err(|_| "Receive state unavailable")?;
        state.expire();
        state.refresh_needs();
        let epoch = state.epoch;
        let current_context = if state.accepts(epoch) {
            state
                .view
                .route
                .clone()
                .zip(state.view.mode)
                .map(|(route, decoder_mode)| NeedsContext {
                    worker_instance: state.spectrum_identity.0.clone(),
                    worker_epoch: epoch,
                    server_instance: state.spectrum_identity.1.clone(),
                    workspace_id: state.spectrum_identity.2.clone(),
                    route,
                    decoder_mode,
                })
        } else {
            None
        };
        let mut view = state.view.clone();
        view.current_context = current_context;
        Ok(view)
    }
    /// No HTTP, DSP, device or Engine access. An absent/newless/stale spectrum
    /// is None, never an invented row of zero measurements.
    pub fn spectrum(
        &self,
        after: Option<&SpectrumCursor>,
    ) -> Result<Option<SpectrumFrame>, String> {
        let mut state = self.state.lock().map_err(|_| "Receive state unavailable")?;
        state.expire();
        let epoch = state.epoch;
        if !state.accepts(epoch) {
            return Ok(None);
        }
        Ok(state
            .spectrum
            .as_ref()
            .filter(|frame| frame.matches(&state))
            .and_then(|frame| frame.read(after)))
    }
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Release);
        let _ = self.spectrum_wake.try_send(());
        if let Ok(mut state) = self.state.lock() {
            state.reset("off", "Receive decoding is off.");
            state.view.enabled = false;
        }
        // Never wait for FFT on the Stop lane. Finished threads are joined; an
        // unfinished thread retains ACTIVE's lease until it actually retires.
        if let Ok(mut thread) = self.spectrum_thread.lock() {
            if thread.as_ref().is_some_and(|thread| thread.is_finished()) {
                if let Some(thread) = thread.take() {
                    let _ = thread.join();
                }
            }
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
    }
}

enum LookupFailure {
    Retry,
    Retired,
}
enum Lookup {
    Deferred,
    Attempt(Result<NeedsBatch, LookupFailure>),
}

fn spawn_lookup(
    query: impl FnMut(&[NeedsQuery]) -> Lookup + Send + 'static,
    state: Arc<Mutex<State>>,
    cancel: Arc<AtomicBool>,
    wake: mpsc::Receiver<()>,
    lease: Arc<Lease>,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("xlog-rx-needs".into())
        .spawn(move || {
            let _lease = lease;
            // Dropping the owned query closure also drops NeedsClient, revoking facts.
            lookup_loop(query, state, cancel, wake);
        })
}

fn lookup_loop(
    mut query: impl FnMut(&[NeedsQuery]) -> Lookup,
    state: Arc<Mutex<State>>,
    cancel: Arc<AtomicBool>,
    wake: mpsc::Receiver<()>,
) {
    let mut cache = Cache::default();
    let mut schedule = LookupSchedule::default();
    let needs_blocked = match state.lock() {
        Ok(state) => state.needs_blocked.clone(),
        Err(_) => return,
    };
    let mut cadence = Cadence::new(Instant::now());
    while !cancel.load(Ordering::Acquire) {
        // Re-check revocable TTLs while idle, but at most one HTTP batch per second.
        if matches!(
            wake.recv_timeout(cadence.wait(Instant::now())),
            Err(mpsc::RecvTimeoutError::Disconnected)
        ) {
            break;
        }
        if cancel.load(Ordering::Acquire) {
            break;
        }
        let (retained, mut candidates) = {
            let Ok(mut state) = state.lock() else {
                break;
            };
            state.expire();
            if !state.view.enabled {
                break;
            }
            state.refresh_needs();
            if state.needs_blocked.load(Ordering::Acquire) {
                cache.clear();
                schedule.cancel(false);
                continue;
            }
            let retained = state
                .view
                .rows
                .iter()
                .filter_map(Row::candidate)
                .collect::<Vec<_>>();
            let candidates = state
                .view
                .rows
                .iter()
                .rev()
                .filter(|row| row.fact.is_none())
                .filter_map(Row::candidate)
                .collect::<Vec<_>>();
            (retained, candidates)
        };
        schedule.order(&retained, &mut candidates, Instant::now());
        let mut requests: Vec<NeedsQuery> = Vec::new();
        let mut keys: Vec<Candidate> = Vec::new();
        let mut bindings: Vec<(Candidate, String)> = Vec::new();
        let mut cached = Vec::new();
        for candidate in candidates {
            if let Some(fact) = cache.find(&candidate) {
                cached.push((candidate, fact));
                continue;
            }
            if let Some(index) = keys.iter().position(|old| old.same_lookup(&candidate)) {
                bindings.push((candidate, requests[index].query_id.clone()));
            } else if requests.len() < 32 {
                let key = schedule.query_key(&candidate);
                let query_id = key.decode_id.clone();
                requests.push(NeedsQuery {
                    query_id: query_id.clone(),
                    callsign: candidate.callsign.clone(),
                    dial_frequency_hz: candidate.context.route.frequency_hz,
                    mode: candidate.context.decoder_mode(),
                });
                keys.push(key);
                bindings.push((candidate, query_id));
            }
        }
        if let Ok(mut state) = state.lock() {
            for (candidate, fact) in cached {
                state.attach(&candidate, fact);
            }
        } else {
            break;
        }
        if requests.is_empty()
            || !cadence.ready(Instant::now())
            || cancel.load(Ordering::Acquire)
            || needs_blocked.load(Ordering::Acquire)
        {
            continue;
        }
        // No receive, main client, session, or Stop lock crosses this bounded HTTP call.
        let started = Instant::now();
        let result = match query(&requests) {
            Lookup::Deferred => continue,
            Lookup::Attempt(result) => result,
        };
        cadence.attempted(started);
        schedule.attempted(&keys, started);
        if cancel.load(Ordering::Acquire) {
            break;
        }
        let Ok(mut state) = state.lock() else {
            break;
        };
        if needs_blocked.load(Ordering::Acquire) {
            schedule.cancel(false);
            cache.clear();
            state.refresh_needs();
            continue;
        }
        // Original representative rows may have been trimmed during HTTP.
        let surviving: Vec<_> = state.view.rows.iter().filter_map(Row::candidate).collect();
        bindings.retain(|(candidate, _)| schedule.original_survives(candidate, &surviving));
        let mut useful = false;
        match result {
            Ok(batch) => {
                let authoritative = batch
                    .metadata()
                    .is_ok_and(|metadata| metadata.authority.is_some());
                if !authoritative {
                    // Preserve an unavailable owner explanation without caching a
                    // placeholder as a fact or suppressing this group's retry.
                    let reason = batch
                        .results()
                        .ok()
                        .and_then(|rows| rows.first())
                        .and_then(|row| row.need.reason_code.as_deref());
                    let reason = match reason {
                        Some("refreshing") => "refreshing",
                        Some("uncertain") => "uncertain",
                        Some("stale") => "stale",
                        _ => "unavailable",
                    };
                    for row in &mut state.view.rows {
                        if row.candidate().is_some_and(|candidate| {
                            bindings.iter().any(|(key, _)| *key == candidate)
                        }) {
                            row.fact = None;
                            row.needs_fallback = reason;
                            row.refresh_needs();
                        }
                    }
                } else {
                    let batch = Arc::new(batch);
                    for (candidate, query_id) in bindings {
                        let Some(fact) = Fact::new(batch.clone(), query_id) else {
                            continue;
                        };
                        // Populate cache only when an original row/context still survives.
                        if state.attach(&candidate, fact.clone()) {
                            useful = true;
                            cache.insert(candidate, fact);
                        }
                    }
                }
            }
            Err(error) => {
                if matches!(error, LookupFailure::Retired) {
                    schedule.cancel(true);
                }
                cache.clear();
                for row in &mut state.view.rows {
                    row.fact = None;
                    if row.sender_callsign.is_some() {
                        row.needs_fallback = "native_lookup_unavailable";
                    }
                    row.refresh_needs();
                }
            }
        }
        schedule.complete(useful, Instant::now());
    }
}

fn context(d: &AudioDescriptor, server: &str, workspace: &str, mode: Mode) -> ReceiveContext {
    ReceiveContext {
        mode: mode.native(),
        owner: OwnerIdentity {
            server_instance: server.into(),
            workspace_id: workspace.into(),
            capture_id: d.capture_id.clone(),
            epoch: d.epoch.clone(),
            radio_id: d.radio_id.clone(),
            receiver_id: d.receiver_id.clone(),
            mapping_generation: d.mapping_generation,
            device_id: d.device_id.clone(),
            slice: d.slice.clone(),
            dax_channel: d.dax_channel,
            frequency_hz: d.frequency_hz,
            radio_mode: d.mode.clone(),
            sample_rate: d.sample_rate,
            source_channels: d.source_channels as u8,
            selected_channel: d.selected_channel as u8,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::decoded_needs::Scheduler;
    use super::*;
    pub(super) static ACTIVE_TEST_LOCK: Mutex<()> = Mutex::new(());
    pub(super) fn live_authority() -> (
        xnexus_xlog_client::BridgeClient,
        AudioClient,
        Arc<AudioBatch>,
    ) {
        let (bridge, audio, mut batches) = authority_batches(48000, &[1000]);
        (bridge, audio, batches.remove(0))
    }
    pub(super) fn authority_batches(
        rate: u32,
        lifetimes: &[u64],
    ) -> (
        xnexus_xlog_client::BridgeClient,
        AudioClient,
        Vec<Arc<AudioBatch>>,
    ) {
        use std::io::{Read, Write};
        let lifetimes = lifetimes.to_vec();
        let count = lifetimes.len();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for index in 0..=count {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                    assert!(request.len() < 16384);
                }
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let value = if index == 0 {
                    serde_json::json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","capabilities":["snapshot.read","audio.rx"]})
                } else {
                    serde_json::json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","generated_at_unix_ms":now,"max_age_ms":lifetimes[index - 1],"state":"running","reason":null,"descriptor":{"capture_id":"00000000-0000-4000-8000-000000000001","epoch":"00000000-0000-4000-8000-000000000002","radio_id":"radio1","receiver_id":"fixture/slice-a","mapping_generation":1,"device_id":"fixture-dax","slice":"A","dax_channel":2,"frequency_hz":14074000,"mode":"DIGU","source_channels":2,"selected_channel":0,"sample_rate":rate,"sample_format":"s16le","timestamp_basis":"windows_endpoint_capture","remote_rf_latency_us":null,"utc_sync_accuracy_us":null},"oldest_sequence":1,"next_sequence":index as u64 + 1,"gap":false,"chunks":[{"sequence":index as u64,"first_frame":index as u64 - 1,"frame_count":1,"sample0_unix_us":now*1000 + (index as u64 - 1)*1_000_000/u64::from(rate),"calibration_uncertainty_us":10,"pcm_base64":"AAA="}]})
                };
                let body = value.to_string();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        let mut bridge = xnexus_xlog_client::BridgeClient::new(
            &endpoint,
            "synthetic-worker-test-pairing-marker",
        )
        .unwrap();
        bridge.connect().unwrap();
        let mut audio = bridge.audio_handle().unwrap();
        let began = Instant::now();
        let batches = (0..count).map(|_| Arc::new(audio.poll().unwrap_or_else(|error| panic!(
            "synthetic authority setup audio.poll failed: {error:?}; elapsed_ms={}; server_finished={}",
            began.elapsed().as_millis(), server.is_finished()
        )))).collect();
        server.join().unwrap();
        (bridge, audio, batches)
    }
    #[test]
    fn changing_epoch_or_stopping_rejects_late_decoder_results() {
        let (_bridge, _audio, batch) = live_authority();
        let mut state = State {
            view: View {
                enabled: true,
                ..View::default()
            },
            epoch: 7,
            authority: Some(batch.clone()),
            needs_blocked: Arc::new(AtomicBool::new(false)),
            spectrum: None,
            spectrum_mailbox: Arc::new(spectrum::Mailbox::default()),
            spectrum_identity: (
                "fixture".into(),
                "worker-fixture".into(),
                "worker-workspace".into(),
            ),
        };
        assert!(state.accepts(7));
        state.epoch = 8;
        assert!(state.accepts(8));
        assert!(!state.accepts(7));
        state.reset("waiting", "changed");
        assert_eq!(state.epoch, 9);
        assert!(!state.accepts(7));
        assert!(!state.accepts(9)); // reset removed owner authority
        let worker = Worker {
            state: Arc::new(Mutex::new(state)),
            cancel: Arc::new(AtomicBool::new(false)),
            needs_wake: None,
            needs_observation: Arc::new(Observation::default()),
            spectrum_wake: mpsc::sync_channel(1).0,
            spectrum_thread: Mutex::new(None),
        };
        worker.stop();
        assert!(worker.cancel.load(Ordering::Acquire));
        assert!(!worker.view().unwrap().enabled);
    }
    #[test]
    fn revoked_owner_authority_discards_rows_and_partial_context() {
        let (mut bridge, _audio, batch) = live_authority();
        let mut state = State {
            view: View {
                enabled: true,
                route: batch.descriptor.clone(),
                ..View::default()
            },
            epoch: 3,
            authority: Some(batch),
            needs_blocked: Arc::new(AtomicBool::new(false)),
            spectrum: None,
            spectrum_mailbox: Arc::new(spectrum::Mailbox::default()),
            spectrum_identity: (
                "fixture".into(),
                "worker-fixture".into(),
                "worker-workspace".into(),
            ),
        };
        assert!(state.accepts(3));
        state.view.rows.push(Row {
            decode_id: "fixture_e3_p1_n1".into(),
            context: NeedsContext {
                worker_instance: "fixture".into(),
                worker_epoch: 3,
                server_instance: "worker-fixture".into(),
                workspace_id: "worker-workspace".into(),
                route: state.view.route.clone().unwrap(),
                decoder_mode: Mode::FT8,
            },
            sender_callsign: None,
            needs: Default::default(),
            fact: None,
            needs_fallback: "native_unclassified",
            host_period_start_unix_us: 1_790_000_000_000_000,
            mode: Mode::FT8,
            message: "CQ TEST FN42".into(),
            snr_db: -10,
            time_offset_seconds: 0.0,
            audio_frequency_hz: 1000.0,
            clock: Clock {
                timestamp_basis: "windows_endpoint_capture".into(),
                host_mapping_uncertainty_us: 10,
                calibration_uncertainty_us: 10,
                utc_sync_accuracy_us: None,
                remote_rf_latency_us: None,
                sample_alignment_offset_ns: 0,
                resampler_center_offset_ns: 0,
            },
        });
        bridge.disconnect();
        assert!(state.expire());
        assert_eq!(state.epoch, 4);
        assert!(state.view.route.is_none());
        assert!(state.view.rows.is_empty());
        assert!(!state.accepts(4));
    }

    pub(super) fn row(state: &State, id: &str, call: &str) -> Row {
        Row {
            decode_id: id.into(),
            context: NeedsContext {
                worker_instance: "fixture".into(),
                worker_epoch: state.epoch,
                server_instance: "worker-fixture".into(),
                workspace_id: "worker-workspace".into(),
                route: state.view.route.clone().unwrap(),
                decoder_mode: Mode::FT8,
            },
            sender_callsign: Some(call.into()),
            needs: Default::default(),
            fact: None,
            needs_fallback: "native_unclassified",
            host_period_start_unix_us: 1_790_000_000_000_000,
            mode: Mode::FT8,
            message: format!("CQ {call} FN42"),
            snr_db: -10,
            time_offset_seconds: 0.0,
            audio_frequency_hz: 1000.0,
            clock: Clock {
                timestamp_basis: "windows_endpoint_capture".into(),
                host_mapping_uncertainty_us: 10,
                calibration_uncertainty_us: 10,
                utc_sync_accuracy_us: None,
                remote_rf_latency_us: None,
                sample_alignment_offset_ns: 0,
                resampler_center_offset_ns: 0,
            },
        }
    }
    pub(super) fn state(batch: Arc<AudioBatch>) -> State {
        State {
            view: View {
                enabled: true,
                mode: Some(Mode::FT8),
                route: batch.descriptor.clone(),
                ..Default::default()
            },
            epoch: 1,
            authority: Some(batch),
            needs_blocked: Arc::new(AtomicBool::new(false)),
            spectrum: None,
            spectrum_mailbox: Arc::new(spectrum::Mailbox::default()),
            spectrum_identity: (
                "fixture".into(),
                "worker-fixture".into(),
                "worker-workspace".into(),
            ),
        }
    }
    fn needs_batch(
        queries: &[NeedsQuery],
        lifetime: u64,
    ) -> (
        xnexus_xlog_client::BridgeClient,
        NeedsClient,
        Arc<NeedsBatch>,
    ) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let queries = queries.to_vec();
        let replies = queries.clone();
        let server = std::thread::spawn(move || {
            for step in 0..3 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut headers = Vec::new();
                while !headers.ends_with(b"\r\n\r\n") {
                    let mut b = [0];
                    stream.read_exact(&mut b).unwrap();
                    headers.push(b[0]);
                    assert!(headers.len() < 16384);
                }
                let headers = String::from_utf8(headers).unwrap();
                let length = headers
                    .lines()
                    .find_map(|l| {
                        l.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .and_then(|v| v.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                let mut body = vec![0; length];
                stream.read_exact(&mut body).unwrap();
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                let authority = serde_json::json!({"logbook_revision":1,"classification_revision":1,"country_reference_revision":"xlog-country-facts-v3:fixture","policy_revision":"xlog-call-dxcc-needs-v1"});
                let value = match step {
                    0 => {
                        serde_json::json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","capabilities":["snapshot.read","logbook.classify"]})
                    }
                    1 => {
                        serde_json::json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","revision":1,"generated_at_unix_ms":now,"max_age_ms":2000,"station":{"connected":false,"radio_id":null,"frequency_hz":null,"mode":null,"transmitting":null},"rotor":{"connected":false,"rotor_id":null,"azimuth_deg":null,"moving":null},"spots":[],"needs":{"generated_at_unix_ms":now,"max_age_ms":2000,"readiness":"ready","authority":authority}})
                    }
                    _ => {
                        let sent: serde_json::Value = serde_json::from_slice(&body).unwrap();
                        assert_eq!(sent["queries"], serde_json::to_value(&replies).unwrap());
                        let results:Vec<_>=replies.iter().rev().map(|q|serde_json::json!({"query_id":q.query_id,"normalized_callsign":q.callsign,"dial_frequency_hz":q.dial_frequency_hz,"decoder_mode":q.mode,"classification_mode":"DIGI","band":"20m","country":"United States","adif":291,"country_resolution":"resolved","call_status":"New Call","dxcc_status":"ATNO","need":{"status":"needed","score":100,"reason_code":null,"reasons":["Owner fixture new call"]}})).collect();
                        serde_json::json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","generated_at_unix_ms":now,"max_age_ms":lifetime,"readiness":"ready","authority":authority,"results":results})
                    }
                };
                let body = value.to_string();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
        });
        let mut client = xnexus_xlog_client::BridgeClient::new(
            &endpoint,
            "synthetic-needs-worker-pairing-marker",
        )
        .unwrap();
        client.connect().unwrap();
        client.poll().unwrap();
        let mut needs = client.needs_handle().unwrap();
        let batch = Arc::new(needs.query(&queries).unwrap());
        server.join().unwrap();
        // A handle is intentionally kept alive until query completion; facts retain
        // the parent lease independently, just as the production lookup worker does.
        (client, needs, batch)
    }
    fn query(candidate: &Candidate) -> NeedsQuery {
        NeedsQuery {
            query_id: candidate.decode_id.clone(),
            callsign: candidate.callsign.clone(),
            dial_frequency_hz: candidate.context.route.frequency_hz,
            mode: candidate.context.decoder_mode(),
        }
    }
    #[test]
    fn replies_attach_by_immutable_id_not_order_and_reject_trim_or_route_changes() {
        let (_audio_bridge, _audio, batch) = live_authority();
        let mut state = state(batch);
        state.view.rows = vec![row(&state, "row_a", "K1ABC"), row(&state, "row_b", "W9XYZ")];
        let a = state.view.rows[0].candidate().unwrap();
        let b = state.view.rows[1].candidate().unwrap();
        let (_owner, _needs, facts) = needs_batch(&[query(&a), query(&b)], 2000);
        let fact = Fact::new(facts.clone(), a.decode_id.clone()).unwrap();
        assert!(
            !state.attach(&b, fact.clone()),
            "query identity cannot substitute another literal call"
        );
        state.view.rows.reverse();
        assert!(state.attach(&a, fact.clone()));
        assert_eq!(
            state.view.rows[1]
                .needs
                .owner
                .as_ref()
                .unwrap()
                .normalized_callsign,
            "K1ABC"
        );
        assert_eq!(
            state.view.rows[0].needs.status,
            xnexus_xlog_client::NeedStatus::Unknown
        );
        state.view.rows.retain(|row| row.decode_id != a.decode_id);
        assert!(!state.attach(&a, fact));
        let fact_b = Fact::new(facts, b.decode_id.clone()).unwrap();
        state.view.route.as_mut().unwrap().frequency_hz = 7074000;
        assert!(!state.attach(&b, fact_b.clone()));
        state.view.route = Some(b.context.route.clone());
        state.view.mode = Some(Mode::FT4);
        assert!(!state.attach(&b, fact_b.clone()));
        state.view.mode = Some(Mode::FT8);
        state.epoch += 1;
        assert!(!state.attach(&b, fact_b));
    }
    #[test]
    fn cached_facts_keep_original_ttl_and_parent_revocation() {
        let (_audio_bridge, _audio, batch) = live_authority();
        let state = state(batch);
        let candidate = row(&state, "row_a", "K1ABC").candidate().unwrap();
        let (mut owner, _needs, facts) = needs_batch(&[query(&candidate)], 100);
        let fact = Fact::new(facts, candidate.decode_id.clone()).unwrap();
        let mut cache = Cache::default();
        cache.insert(candidate.clone(), fact.clone());
        assert!(cache.find(&candidate).is_some());
        let mut changed = candidate.clone();
        changed.context.route.mapping_generation += 1;
        assert!(cache.find(&changed).is_none());
        let deadline = Instant::now() + Duration::from_secs(1);
        while fact.view().is_some() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(cache.find(&candidate).is_none());
        owner.disconnect();
        let (mut owner, _needs, facts) = needs_batch(&[query(&candidate)], 2000);
        let fact = Fact::new(facts, candidate.decode_id.clone()).unwrap();
        assert!(fact.view().is_some());
        owner.disconnect();
        assert!(fact.view().is_none());
        let (owner, _needs, facts) = needs_batch(&[query(&candidate)], 2000);
        let fact = Fact::new(facts, candidate.decode_id.clone()).unwrap();
        owner.invalidate_needs();
        assert!(
            fact.view().is_none(),
            "pending local writes must revoke cached facts"
        );
    }
    #[test]
    fn scheduler_visits_every_retained_key_despite_expiry_and_prunes_old_contexts() {
        let (_audio_bridge, _audio, batch) = live_authority();
        let state = state(batch);
        for count in [80usize, 200] {
            let retained = (0..count)
                .map(|n| {
                    row(&state, &format!("row_{n}"), &format!("K{n}ABC"))
                        .candidate()
                        .unwrap()
                })
                .collect::<Vec<_>>();
            let mut scheduler = Scheduler::default();
            let mut visited = std::collections::HashSet::new();
            // Treat all facts as expired every tick: the strongest starvation case.
            for tick in 0..count.div_ceil(32) {
                let mut eligible = retained.iter().rev().cloned().collect::<Vec<_>>();
                scheduler.order(&retained, &mut eligible);
                let keys = &eligible[..32.min(eligible.len())];
                if tick == 0 {
                    assert_eq!(keys[0].decode_id, format!("row_{}", count - 1));
                }
                for key in keys {
                    visited.insert(key.decode_id.clone());
                }
                scheduler.attempted(keys);
            }
            assert_eq!(visited.len(), count, "all retained calls must get a turn");
            let mut eligible = retained.iter().rev().cloned().collect::<Vec<_>>();
            scheduler.order(&retained, &mut eligible);
            // Previously retried last-batch keys sort behind less recently attempted keys.
            assert_ne!(eligible[0].decode_id, retained.last().unwrap().decode_id);
            // Trimming prunes attempt metadata; a returning row is genuinely new.
            let survivor = retained[0].clone();
            scheduler.order(std::slice::from_ref(&survivor), &mut []);
            let mut restored = vec![survivor.clone(), retained[1].clone()];
            scheduler.order(&retained, &mut restored);
            assert_eq!(restored[0].decode_id, retained[1].decode_id);
            let mut replacement = survivor.clone();
            replacement.context.route.mapping_generation += 1;
            let mut eligible = vec![survivor, replacement.clone()];
            scheduler.order(&eligible.clone(), &mut eligible);
            assert_eq!(
                eligible[0], replacement,
                "new route has independent attempt history"
            );
        }
    }
    #[test]
    fn stalled_lookup_never_holds_receive_state_or_stop_and_batch_is_bounded() {
        let (_audio_bridge, _audio, batch) = live_authority();
        let mut state = state(batch);
        state.view.rows = (0..80)
            .map(|n| row(&state, &format!("row_{n}"), &format!("K{n}ABC")))
            .collect();
        let state = Arc::new(Mutex::new(state));
        let worker = Worker {
            state: state.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
            needs_wake: None,
            needs_observation: Arc::new(Observation::default()),
            spectrum_wake: mpsc::sync_channel(1).0,
            spectrum_thread: Mutex::new(None),
        };
        let (wake, rx) = mpsc::sync_channel(1);
        wake.try_send(()).unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let cancel = worker.cancel.clone();
        let handle = std::thread::spawn(move || {
            lookup_loop(
                |queries| {
                    entered_tx.send(queries.len()).unwrap();
                    release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                    Lookup::Attempt(Err(LookupFailure::Retry))
                },
                state,
                cancel,
                rx,
            )
        });
        assert_eq!(entered_rx.recv_timeout(Duration::from_secs(1)).unwrap(), 32);
        // The PCM-side state lane can publish while HTTP remains deliberately held.
        worker.state.try_lock().unwrap().view.phase = "pcm-progress".into();
        assert_eq!(worker.view().unwrap().phase, "pcm-progress");
        worker.stop();
        assert!(!worker.view().unwrap().enabled);
        assert!(!handle.is_finished());
        release_tx.send(()).unwrap();
        handle.join().unwrap();
        assert!(worker.view().unwrap().rows.is_empty());
    }

    #[test]
    fn old_lookup_retains_active_lease_until_exit_without_delaying_stop() {
        let _serial = ACTIVE_TEST_LOCK.lock().unwrap();
        let (_owner, audio, batch) = live_authority();
        let mut state = state(batch);
        state.view.rows = vec![row(&state, "row_a", "K1ABC")];
        let worker = Worker {
            state: Arc::new(Mutex::new(state)),
            cancel: Arc::new(AtomicBool::new(false)),
            needs_wake: None,
            needs_observation: Arc::new(Observation::default()),
            spectrum_wake: mpsc::sync_channel(1).0,
            spectrum_thread: Mutex::new(None),
        };
        assert!(ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok());
        let lease = Arc::new(Lease);
        let (wake, rx) = mpsc::sync_channel(1);
        wake.try_send(()).unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let handle = spawn_lookup(
            move |_| {
                entered_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
                Lookup::Attempt(Err(LookupFailure::Retry))
            },
            worker.state.clone(),
            worker.cancel.clone(),
            rx,
            lease.clone(),
        )
        .unwrap();
        drop(lease);
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.stop();
        assert!(!worker.view().unwrap().enabled);
        assert!(ACTIVE.load(Ordering::Acquire));
        assert!(Worker::start(
            audio,
            "worker-fixture".into(),
            "worker-workspace".into(),
            Mode::FT8
        )
        .is_err());
        release_tx.send(()).unwrap();
        handle.join().unwrap();
        assert!(!ACTIVE.load(Ordering::Acquire));
    }

    #[test]
    fn lookup_error_clears_previous_facts_without_stopping_receive() {
        let (_audio_owner, _audio, batch) = live_authority();
        let mut state = state(batch);
        state.view.rows = vec![row(&state, "row_a", "K1ABC"), row(&state, "row_b", "W9XYZ")];
        let candidate = state.view.rows[0].candidate().unwrap();
        let (_owner, _needs, facts) = needs_batch(&[query(&candidate)], 2000);
        assert!(state.attach(
            &candidate,
            Fact::new(facts, candidate.decode_id.clone()).unwrap()
        ));
        let worker = Worker {
            state: Arc::new(Mutex::new(state)),
            cancel: Arc::new(AtomicBool::new(false)),
            needs_wake: None,
            needs_observation: Arc::new(Observation::default()),
            spectrum_wake: mpsc::sync_channel(1).0,
            spectrum_thread: Mutex::new(None),
        };
        let (wake, rx) = mpsc::sync_channel(1);
        wake.try_send(()).unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let state = worker.state.clone();
        let cancel = worker.cancel.clone();
        let handle = std::thread::spawn(move || {
            lookup_loop(
                |_| {
                    entered_tx.send(()).unwrap();
                    Lookup::Attempt(Err(LookupFailure::Retry))
                },
                state,
                cancel,
                rx,
            )
        });
        entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            let view = worker.view().unwrap();
            assert!(view.enabled && view.rows.len() == 2);
            if view.rows[0].needs.status == xnexus_xlog_client::NeedStatus::Unknown {
                assert!(view.rows[0].needs.score.is_none());
                assert_eq!(
                    view.rows[0].needs.reason_code.as_deref(),
                    Some("native_lookup_unavailable")
                );
                break;
            }
            assert!(
                Instant::now() < deadline,
                "lookup error retained previous facts"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        worker.stop();
        handle.join().unwrap();
    }

    #[test]
    fn absent_capability_is_unknown_and_never_infers_from_audio() {
        let (owner, _audio, batch) = live_authority();
        assert!(matches!(
            owner.needs_handle(),
            Err(xnexus_xlog_client::Error::Capability)
        ));
        let state = state(batch);
        let mut row = row(&state, "row_a", "K1ABC");
        row.needs_fallback = "native_capability_absent";
        row.refresh_needs();
        assert_eq!(row.needs.status, xnexus_xlog_client::NeedStatus::Unknown);
        assert_eq!(
            row.needs.reason_code.as_deref(),
            Some("native_capability_absent")
        );
        assert!(row.needs.score.is_none() && row.needs.owner.is_none());
        assert_eq!(row.needs.remaining_valid_ms, 0);
    }

    #[test]
    fn cancelled_start_releases_lookup_lease_without_issuing_a_request() {
        let _serial = ACTIVE_TEST_LOCK.lock().unwrap();
        let (_owner, _audio, batch) = live_authority();
        let state = Arc::new(Mutex::new(state(batch)));
        assert!(ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok());
        let lease = Arc::new(Lease);
        let (wake, rx) = mpsc::sync_channel(1);
        let cancel = Arc::new(AtomicBool::new(true));
        let handle = spawn_lookup(
            |_| panic!("Cancelled startup must not query"),
            state,
            cancel,
            rx,
            lease.clone(),
        )
        .unwrap();
        drop(lease);
        drop(wake);
        handle.join().unwrap();
        assert!(!ACTIVE.load(Ordering::Acquire));
    }

    #[test]
    fn durable_pending_fence_reconstructs_after_restart_and_is_workspace_specific() {
        use xnexus_xlog_client::outbox::{LogbookMutation, Outbox};
        let root = std::env::temp_dir().join(format!(
            "xnexus-native-needs-outbox-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let request = "00000000-0000-4000-8000-000000000123";
        let workspace = "worker-workspace";
        let mutation:LogbookMutation = serde_json::from_value(serde_json::json!({"kind":"logbook.create","qso":{"contact_id":"00000000-0000-4000-8000-000000000124","callsign":"K1ABC","band":"20m","mode":"FT8","started_at_unix_ms":1790000000000u64,"frequency_hz":14074000,"rst_sent":"-10","rst_received":"-12","grid":"FN42","comment":"disposable needs gate fixture"}})).unwrap();
        {
            let mut queue = Outbox::open(&root).unwrap();
            queue.enqueue(workspace, request, mutation).unwrap();
            assert!(needs_blocked_by_outbox(&queue.entries(), workspace));
            assert!(!needs_blocked_by_outbox(
                &queue.entries(),
                "another-workspace"
            ));
        }
        {
            let mut queue = Outbox::open(&root).unwrap();
            assert!(needs_blocked_by_outbox(&queue.entries(), workspace));
            queue.mark_attempted(request, workspace).unwrap();
        }
        {
            let mut queue = Outbox::open(&root).unwrap();
            assert!(matches!(
                queue.entries()[0].state,
                OutboxState::NeedsReceipt
            ));
            assert!(needs_blocked_by_outbox(&queue.entries(), workspace));
            queue.settle(serde_json::from_value(serde_json::json!({"request_id":request,"server_instance":"worker-fixture","workspace_id":workspace,"status":"rejected","message":"synthetic rejection"})).unwrap()).unwrap();
        }
        let queue = Outbox::open(&root).unwrap();
        assert!(!needs_blocked_by_outbox(&queue.entries(), workspace));
        println!("XNEXUS_NEEDS_OUTBOX_FIXTURE {}", root.display());
    }
    #[test]
    fn pending_write_rejects_fresh_facts_and_settlement_requires_new_query() {
        let (_audio_owner, _audio, audio_batch) = live_authority();
        let mut state = state(audio_batch);
        state.view.rows = vec![row(&state, "row_a", "K1ABC")];
        let candidate = state.view.rows[0].candidate().unwrap();
        let (owner, _needs, batch) = needs_batch(&[query(&candidate)], 2000);
        let fact = Fact::new(batch, candidate.decode_id.clone()).unwrap();
        assert!(state.attach(&candidate, fact.clone()));
        state.needs_blocked.store(true, Ordering::Release);
        owner.invalidate_needs();
        // Even a newly published, authenticated owner snapshot/response cannot
        // override the durable local Pending/NeedsReceipt gate.
        let (_fresh_owner, _fresh_needs, fresh_batch) = needs_batch(&[query(&candidate)], 2000);
        let fresh_fact = Fact::new(fresh_batch, candidate.decode_id.clone()).unwrap();
        assert!(!state.attach(&candidate, fresh_fact));
        state.refresh_needs();
        assert_eq!(
            state.view.rows[0].needs.reason_code.as_deref(),
            Some("native_write_pending")
        );
        assert!(state.view.rows[0].needs.owner.is_none());
        assert!(
            state.accepts(candidate.context.worker_epoch),
            "PCM authority stays valid"
        );
        state.needs_blocked.store(false, Ordering::Release);
        state.refresh_needs();
        assert!(state.view.rows[0].needs.owner.is_none());
        assert!(
            !state.attach(&candidate, fact),
            "pre-write fact remains revoked after settlement"
        );
        let (_settled_owner, _settled_needs, new_batch) = needs_batch(&[query(&candidate)], 2000);
        assert!(state.attach(
            &candidate,
            Fact::new(new_batch, candidate.decode_id.clone()).unwrap()
        ));
        state.needs_blocked.store(true, Ordering::Release);
        let worker = Worker {
            state: Arc::new(Mutex::new(state)),
            cancel: Arc::new(AtomicBool::new(false)),
            needs_wake: None,
            needs_observation: Arc::new(Observation::default()),
            spectrum_wake: mpsc::sync_channel(1).0,
            spectrum_thread: Mutex::new(None),
        };
        assert_eq!(worker.view().unwrap().rows[0].needs.remaining_valid_ms, 0);
        worker.stop();
        assert!(worker.view().unwrap().rows.is_empty());
    }
    #[test]
    fn native_serialization_fixture_preserves_exact_owner_and_unknown_fields() {
        let (_audio_owner, _audio, batch) = live_authority();
        let mut state = state(batch);
        state.view.phase = "receiving".into();
        state.view.rows = vec![row(&state, "row_a", "K1ABC"), row(&state, "row_b", "W9XYZ")];
        let candidate = state.view.rows[0].candidate().unwrap();
        let (_owner, _needs, facts) = needs_batch(&[query(&candidate)], 2000);
        assert!(state.attach(
            &candidate,
            Fact::new(facts, candidate.decode_id.clone()).unwrap()
        ));
        let value = serde_json::to_value(&state.view).unwrap();
        assert_eq!(value["rows"][0]["needs"]["status"], "needed");
        assert_eq!(
            value["rows"][0]["needs"]["owner"]["normalized_callsign"],
            "K1ABC"
        );
        assert_eq!(value["rows"][1]["needs"]["status"], "unknown");
        assert!(value["rows"][1]["needs"]["score"].is_null());
        assert_eq!(value["rows"][1]["needs"]["remaining_valid_ms"], 0);
        assert!(value["rows"][0].get("fact").is_none());
        println!(
            "XNEXUS_DECODED_NEEDS_FIXTURE {}",
            serde_json::to_string(&value).unwrap()
        );
    }
}
