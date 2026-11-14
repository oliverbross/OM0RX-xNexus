// SPDX-License-Identifier: GPL-3.0-only
//! Bounded native-only receive worker. Never owns a device or a transmit path.
//! Polling/assembly and decoding have separate threads and a one-period mailbox.
use crate::xlog_decode::{CompleteFrame, Decoder};
use serde::{Deserialize, Serialize};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc, Mutex,
};
use std::time::Duration;
use xnexus_xlog_client::audio::{AudioBatch, AudioClient, AudioDescriptor, AudioState};
use xnexus_xlog_rx::{
    CompletePeriod, OwnerIdentity, PcmChunk, ReceiveContext, RxAssembler, RxMode,
};

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
    pub host_period_start_unix_us: u64,
    pub mode: Mode,
    pub message: String,
    pub snr_db: i32,
    pub time_offset_seconds: f32,
    pub audio_frequency_hz: f32,
    pub clock: Clock,
}
#[derive(Clone, Serialize)]
pub struct View {
    pub enabled: bool,
    pub mode: Option<Mode>,
    pub phase: String,
    pub reason: String,
    pub route: Option<AudioDescriptor>,
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
            rows: vec![],
        }
    }
}
struct State {
    view: View,
    epoch: u64,
    authority: Option<Arc<AudioBatch>>,
}
impl State {
    fn reset(&mut self, phase: &str, reason: &str) {
        self.epoch = self.epoch.wrapping_add(1);
        self.authority = None;
        self.view.route = None;
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
}
pub struct Worker {
    state: Arc<Mutex<State>>,
    cancel: Arc<AtomicBool>,
}
struct Job {
    epoch: u64,
    period: CompletePeriod,
}
static ACTIVE: AtomicBool = AtomicBool::new(false);
struct Lease;
impl Drop for Lease {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::Release);
    }
}

impl Worker {
    pub fn start(
        mut audio: AudioClient,
        server: String,
        workspace: String,
        mode: Mode,
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
        }));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::sync_channel::<Job>(1);
        let decode_state = state.clone();
        let decode_cancel = cancel.clone();
        let decode_lease = lease.clone();
        std::thread::Builder::new().name("xlog-rx-decode".into()).spawn(move || {
            let _lease = decode_lease;
            let mut decoder = Decoder::default();
            let mut last_epoch = None;
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
                        let clock = Clock { timestamp_basis:period.clock.timestamp_basis.into(), host_mapping_uncertainty_us:period.clock.host_mapping_uncertainty_us, calibration_uncertainty_us:period.clock.calibration_uncertainty_us, utc_sync_accuracy_us:period.clock.utc_sync_accuracy_us, remote_rf_latency_us:period.clock.remote_rf_latency_us, sample_alignment_offset_ns:period.clock.sample_alignment_offset_ns, resampler_center_offset_ns:period.clock.resampler_center_offset_ns };
                        state.view.phase = "receiving".into();
                        state.view.reason = "Receiving xLog audio. Host clock timing; UTC synchronization and RF latency are unknown.".into();
                        for row in period.rows.into_iter().take(100) {
                            if row.message.len() > 512 || !row.time_offset_seconds.is_finite() || !row.audio_frequency_hz.is_finite() { continue; }
                            state.view.rows.push(Row { host_period_start_unix_us:host_time, mode, message:row.message, snr_db:row.snr_db, time_offset_seconds:row.time_offset_seconds, audio_frequency_hz:row.audio_frequency_hz, clock:clock.clone() });
                        }
                        let extra = state.view.rows.len().saturating_sub(200);
                        state.view.rows.drain(..extra);
                    }
                    Err(error) => state.reset("waiting", &format!("Decode discarded: {error}")),
                }
            }
        }).map_err(|_| "Cannot start receive decoder worker")?;
        let poll_state = state.clone();
        let poll_cancel = cancel.clone();
        let started = std::thread::Builder::new().name("xlog-rx-read".into()).spawn(move || {
            let _lease = lease;
            let mut assembler = RxAssembler::default();
            let mut epoch = 1;
            while !poll_cancel.load(Ordering::Acquire) {
                {
                    let Ok(mut state) = poll_state.lock() else { break; };
                    state.expire();
                    if state.epoch != epoch { assembler.reset(); epoch = state.epoch; }
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
                        state.reset("waiting", batch.reason.as_deref().unwrap_or("Audio route changed; waiting for a complete period."));
                    }
                    epoch = state.epoch;
                    if running { state.view.route = batch.descriptor.clone(); state.authority = Some(batch.clone()); }
                }
                if let Some(route) = batch.descriptor.as_ref().filter(|_| running) {
                    let context = context(route, &server, &workspace, mode);
                    for chunk in &batch.chunks {
                        if poll_cancel.load(Ordering::Acquire) || !batch.is_fresh() { break; }
                        match assembler.push(&context, PcmChunk { sequence:chunk.sequence, first_frame:chunk.first_frame, sample0_unix_us:chunk.sample0_unix_us, calibration_uncertainty_us:chunk.calibration_uncertainty_us, samples:&chunk.samples }) {
                            Ok(Some(period)) => match tx.try_send(Job { epoch, period }) {
                                Ok(()) => {},
                                Err(mpsc::TrySendError::Full(_)) => {
                                    assembler.reset();
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
        });
        if started.is_err() {
            cancel.store(true, Ordering::Release);
            return Err("Cannot start receive polling worker".into());
        }
        Ok(Self { state, cancel })
    }
    pub fn view(&self) -> Result<View, String> {
        let mut state = self.state.lock().map_err(|_| "Receive state unavailable")?;
        state.expire();
        Ok(state.view.clone())
    }
    pub fn stop(&self) {
        self.cancel.store(true, Ordering::Release);
        if let Ok(mut state) = self.state.lock() {
            state.reset("off", "Receive decoding is off.");
            state.view.enabled = false;
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.stop();
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
    use super::*;
    fn live_authority() -> (
        xnexus_xlog_client::BridgeClient,
        AudioClient,
        Arc<AudioBatch>,
    ) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for hello in [true, false] {
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
                let value = if hello {
                    serde_json::json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","capabilities":["snapshot.read","audio.rx"]})
                } else {
                    serde_json::json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","generated_at_unix_ms":now,"max_age_ms":1000,"state":"running","reason":null,"descriptor":{"capture_id":"00000000-0000-4000-8000-000000000001","epoch":"00000000-0000-4000-8000-000000000002","radio_id":"radio1","receiver_id":"fixture/slice-a","mapping_generation":1,"device_id":"fixture-dax","slice":"A","dax_channel":2,"frequency_hz":14074000,"mode":"DIGU","source_channels":2,"selected_channel":0,"sample_rate":48000,"sample_format":"s16le","timestamp_basis":"windows_endpoint_capture","remote_rf_latency_us":null,"utc_sync_accuracy_us":null},"oldest_sequence":1,"next_sequence":2,"gap":false,"chunks":[{"sequence":1,"first_frame":0,"frame_count":1,"sample0_unix_us":now*1000,"calibration_uncertainty_us":10,"pcm_base64":"AAA="}]})
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
        let batch = Arc::new(audio.poll().unwrap());
        server.join().unwrap();
        (bridge, audio, batch)
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
        };
        assert!(state.accepts(3));
        state.view.rows.push(Row {
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
}
