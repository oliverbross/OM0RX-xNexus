// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Bounded, receive-only PCM from xLog's selected Windows capture endpoint.
//! Keep this handle on its own worker, outside the observation/Stop mutexes.
//! Host clock correlation is NOT proof of RF latency or UTC synchronization.
//! Every error invalidates this handle; reconnect explicitly. Consumers discard
//! partial windows on `reset`, expiry, disconnect or any error. PCM stays native.
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};

const SAFE: u64 = 9_007_199_254_740_990;
const RESPONSE_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct AudioDescriptor {
    pub capture_id: String,
    pub epoch: String,
    pub radio_id: String,
    pub receiver_id: String,
    pub mapping_generation: u64,
    pub device_id: String,
    pub slice: String,
    pub dax_channel: u8,
    pub frequency_hz: u64,
    pub mode: String,
    pub source_channels: u16,
    pub selected_channel: u16,
    pub sample_rate: u32,
    pub sample_format: String,
    pub timestamp_basis: String,
    pub remote_rf_latency_us: Option<i64>,
    pub utc_sync_accuracy_us: Option<u32>,
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use serde_json::{json, Value};
    const NOW: u64 = 1_790_000_000_000;
    pub(crate) fn fixture(now: u64) -> Value {
        json!({"protocol_version":1,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace",
            "generated_at_unix_ms":now,"max_age_ms":1000,"state":"running","reason":null,
            "descriptor":{"capture_id":"00000000-0000-4000-8000-000000000001","epoch":"00000000-0000-4000-8000-000000000002",
            "radio_id":"fake-flex","receiver_id":"fake-client/slice-a","mapping_generation":1,"device_id":"fake-dax-endpoint",
            "slice":"A","dax_channel":2,"frequency_hz":14074000,"mode":"DIGU","source_channels":2,"selected_channel":0,
            "sample_rate":48000,"sample_format":"s16le","timestamp_basis":"windows_endpoint_capture",
            "remote_rf_latency_us":null,"utc_sync_accuracy_us":null},"oldest_sequence":1,"next_sequence":2,"gap":false,
            "chunks":[{"sequence":1,"first_frame":0,"frame_count":3,"sample0_unix_us":now*1000-10000,
            "calibration_uncertainty_us":10,"pcm_base64":STANDARD.encode([0u8,128,0,0,255,127])}]})
    }
    fn client() -> AudioClient {
        let mut bridge = BridgeClient::new("http://127.0.0.1:1", &"synthetic".repeat(5)).unwrap();
        bridge.hello = Some(Hello {
            application: "OM0RX-xLog".into(),
            protocol_version: 1,
            server_instance: "synthetic-server".into(),
            workspace_id: "synthetic-workspace".into(),
            capabilities: vec!["audio.rx".into()],
        });
        let mut audio = bridge.audio_handle().unwrap();
        // Pure validator tests own a standalone synthetic session.
        audio.session = Arc::new(AtomicBool::new(true));
        audio
    }
    fn accept(c: &mut AudioClient, v: Value) -> Result<AudioBatch, Error> {
        c.accept(serde_json::from_value(v).unwrap(), NOW)
    }
    #[test]
    fn selected_mono_samples_preserve_unknown_clock_accuracy_and_cursor() {
        let mut c = client();
        let b = accept(&mut c, fixture(NOW)).unwrap();
        assert!(b.reset && b.is_fresh());
        assert_eq!(b.chunks[0].samples, [-32768, 0, 32767]);
        let d = b.descriptor.unwrap();
        assert_eq!(d.dax_channel, 2);
        assert_eq!(d.utc_sync_accuracy_us, None);
        assert_eq!(d.remote_rf_latency_us, None);
        assert_eq!(c.after, 1);
        let mut next = fixture(NOW);
        next["chunks"][0]["sequence"] = json!(2);
        next["next_sequence"] = json!(3);
        next["chunks"][0]["first_frame"] = json!(3);
        assert!(!accept(&mut c, next).unwrap().reset);
        assert_eq!(c.next_frame, Some(6));
    }
    #[test]
    fn missing_repeated_and_reordered_samples_never_extend_a_window() {
        for (sequence, frame) in [(1, 3), (3, 3), (2, 2), (2, 4)] {
            let mut c = client();
            accept(&mut c, fixture(NOW)).unwrap();
            let mut v = fixture(NOW);
            v["chunks"][0]["sequence"] = json!(sequence);
            v["next_sequence"] = json!(sequence + 1);
            v["chunks"][0]["first_frame"] = json!(frame);
            assert!(matches!(accept(&mut c, v), Err(Error::Invalid)));
            assert_eq!(c.after, 1);
            assert_eq!(c.next_frame, Some(3));
        }
    }
    #[test]
    fn gap_route_change_and_stopped_require_window_reset() {
        let mut c = client();
        accept(&mut c, fixture(NOW)).unwrap();
        let mut v = fixture(NOW);
        v["gap"] = json!(true);
        v["oldest_sequence"] = json!(9);
        v["next_sequence"] = json!(10);
        v["chunks"][0]["sequence"] = json!(9);
        v["chunks"][0]["first_frame"] = json!(500);
        assert!(accept(&mut c, v).unwrap().reset);
        let mut v = fixture(NOW);
        v["descriptor"]["frequency_hz"] = json!(7074000);
        v["chunks"][0]["sequence"] = json!(10);
        v["next_sequence"] = json!(11);
        assert!(accept(&mut c, v).unwrap().reset);
        let mut v = fixture(NOW);
        v["state"] = json!("stopped");
        v["descriptor"] = Value::Null;
        v["chunks"] = json!([]);
        v["next_sequence"] = json!(11);
        assert!(accept(&mut c, v).unwrap().reset);
        assert_eq!(c.next_frame, None);
    }
    #[test]
    fn invalid_route_pcm_clock_and_freshness_are_rejected_atomically() {
        let mutations = [
            ("/descriptor/selected_channel", json!(2)),
            ("/descriptor/source_channels", json!(17)),
            ("/descriptor/sample_rate", json!(192001)),
            ("/descriptor/dax_channel", json!(0)),
            ("/descriptor/capture_id", json!("not-uuid")),
            ("/descriptor/mapping_generation", json!(0)),
            ("/descriptor/utc_sync_accuracy_us", json!(10)),
            ("/descriptor/remote_rf_latency_us", json!(0)),
            ("/chunks/0/pcm_base64", json!("AAAA")),
            ("/chunks/0/pcm_base64", json!("%%%=")),
            ("/chunks/0/frame_count", json!(4097)),
            ("/chunks/0/calibration_uncertainty_us", json!(50001)),
            ("/chunks/0/first_frame", json!(SAFE)),
            ("/chunks/0/sample0_unix_us", json!(NOW * 1000 - 3000001)),
            ("/next_sequence", json!(3)),
            ("/max_age_ms", json!(1001)),
            ("/generated_at_unix_ms", json!(NOW - 1000)),
            ("/generated_at_unix_ms", json!(NOW + 51)),
            ("/workspace_id", json!("other")),
            ("/state", json!("stopped")),
        ];
        for (path, value) in mutations {
            let mut c = client();
            let mut v = fixture(NOW);
            *v.pointer_mut(path).unwrap() = value;
            assert!(accept(&mut c, v).is_err(), "{path}");
            assert_eq!(c.after, 0);
            assert!(c.descriptor.is_none());
        }
    }
    #[test]
    fn empty_response_does_not_advance_or_extend_sample_continuity() {
        let mut c = client();
        accept(&mut c, fixture(NOW)).unwrap();
        let mut v = fixture(NOW);
        v["chunks"] = json!([]);
        assert!(!accept(&mut c, v).unwrap().reset);
        assert_eq!(c.after, 1);
        assert_eq!(c.next_frame, Some(3));
    }
    #[test]
    fn queued_audio_expires_and_loses_authority_when_receiver_stops() {
        let mut c = client();
        let mut b = accept(&mut c, fixture(NOW)).unwrap();
        assert!(b.is_fresh());
        let remaining = b.remaining_valid_ms();
        assert!(remaining > 0);
        assert!(b.remaining_valid_ms() <= remaining);
        b.expires = Instant::now() + Duration::from_nanos(999_999);
        assert_eq!(
            b.remaining_valid_ms(),
            0,
            "submillisecond lease is not display authority"
        );
        b.expires = Instant::now();
        assert!(!b.is_fresh());
        assert_eq!(b.remaining_valid_ms(), 0);
        let mut c = client();
        let b = accept(&mut c, fixture(NOW)).unwrap();
        assert!(b.is_fresh());
        c.disconnect();
        assert!(!b.is_fresh());
        assert_eq!(b.remaining_valid_ms(), 0);
    }
}
impl AudioDescriptor {
    fn valid(&self) -> bool {
        commands::valid_request_id(&self.capture_id)
            && commands::valid_request_id(&self.epoch)
            && bounded(&self.radio_id, 256) && bounded(&self.receiver_id, 512)
            && (1..=SAFE).contains(&self.mapping_generation)
            && bounded(&self.device_id, 2048) && self.slice == "A"
            && (1..=8).contains(&self.dax_channel) && frequency(self.frequency_hz)
            && bounded(&self.mode, 32) && (1..=16).contains(&self.source_channels)
            && self.selected_channel < self.source_channels
            && (8000..=192000).contains(&self.sample_rate)
            && self.sample_format == "s16le"
            && self.timestamp_basis == "windows_endpoint_capture"
            // This protocol revision has no qualified measurement of either.
            && self.remote_rf_latency_us.is_none() && self.utc_sync_accuracy_us.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioState {
    Running,
    Stopped,
    Unavailable,
}

#[derive(Deserialize, Serialize)]
struct WireChunk {
    sequence: u64,
    first_frame: u64,
    frame_count: u32,
    sample0_unix_us: u64,
    calibration_uncertainty_us: u32,
    pcm_base64: String,
}
#[derive(Deserialize, Serialize)]
struct WireBatch {
    protocol_version: u32,
    server_instance: String,
    workspace_id: String,
    generated_at_unix_ms: u64,
    max_age_ms: u64,
    state: AudioState,
    reason: Option<String>,
    descriptor: Option<AudioDescriptor>,
    oldest_sequence: u64,
    next_sequence: u64,
    gap: bool,
    chunks: Vec<WireChunk>,
}

/// Selected-channel mono samples, with their original endpoint clock evidence.
pub struct AudioChunk {
    pub sequence: u64,
    pub first_frame: u64,
    pub sample0_unix_us: u64,
    pub calibration_uncertainty_us: u32,
    pub samples: Vec<i16>,
}
pub struct AudioBatch {
    pub state: AudioState,
    pub reason: Option<String>,
    pub descriptor: Option<AudioDescriptor>,
    pub reset: bool,
    pub chunks: Vec<AudioChunk>,
    expires: Instant,
    session: Arc<AtomicBool>,
    receiver: Arc<AtomicBool>,
}
impl AudioBatch {
    /// Original authenticated lease, floored to milliseconds. Reading it never
    /// refreshes samples or extends the owner/session authority.
    pub fn remaining_valid_ms(&self) -> u64 {
        if !self.is_fresh() {
            return 0;
        }
        self.expires
            .saturating_duration_since(Instant::now())
            .as_millis()
            .min(u128::from(u64::MAX)) as u64
    }
    /// Check again immediately before consuming a queued batch.
    pub fn is_fresh(&self) -> bool {
        self.session.load(Ordering::Acquire)
            && self.receiver.load(Ordering::Acquire)
            && Instant::now() < self.expires
    }
}

/// Native credentials are inherited from a paired bridge. No Debug/Serialize.
pub struct AudioClient {
    http: Client,
    endpoint: String,
    hello: Hello,
    alive: Arc<AtomicBool>,
    session: Arc<AtomicBool>,
    after: u64,
    descriptor: Option<AudioDescriptor>,
    next_frame: Option<u64>,
}
impl BridgeClient {
    pub fn audio_handle(&self) -> Result<AudioClient, Error> {
        let hello = self.hello.as_ref().ok_or(Error::Disconnected)?;
        if !hello.capabilities.iter().any(|v| v == "audio.rx") {
            return Err(Error::Capability);
        }
        Ok(AudioClient {
            http: self.http.clone(),
            endpoint: self.endpoint.clone(),
            hello: hello.clone(),
            alive: Arc::new(AtomicBool::new(true)),
            session: self.audio_session.clone(),
            after: 0,
            descriptor: None,
            next_frame: None,
        })
    }
}
impl AudioClient {
    pub fn disconnect(&mut self) {
        self.alive.store(false, Ordering::Release);
        self.descriptor = None;
        self.next_frame = None;
    }
    /// One read, no retry/reconnect. Never starts or configures capture at xLog.
    pub fn poll(&mut self) -> Result<AudioBatch, Error> {
        if !self.alive.load(Ordering::Acquire) || !self.session.load(Ordering::Acquire) {
            return Err(Error::Disconnected);
        }
        let result = (|| {
            let response = self
                .http
                .get(format!(
                    "{}/xlog/v1/audio?after={}",
                    self.endpoint, self.after
                ))
                .timeout(Duration::from_secs(1))
                .send()
                .map_err(|_| Error::Transport)?;
            let wire: WireBatch = read_response(response, RESPONSE_BYTES)?;
            if !self.session.load(Ordering::Acquire) {
                return Err(Error::Disconnected);
            }
            self.accept(wire, now_ms()?)
        })();
        if matches!(
            &result,
            Err(Error::Transport | Error::Stale | Error::Http(408 | 429 | 500 | 502 | 503 | 504))
        ) {
            // Expire captured samples without retiring the authenticated reader.
            // The next batch starts a new partial window at the retained cursor.
            self.alive.store(false, Ordering::Release);
            self.alive = Arc::new(AtomicBool::new(true));
            self.descriptor = None;
            self.next_frame = None;
        } else if result.is_err() {
            self.disconnect();
        }
        result
    }
    fn accept(&mut self, wire: WireBatch, now: u64) -> Result<AudioBatch, Error> {
        if wire.protocol_version != 1 {
            return Err(Error::Incompatible);
        }
        if wire.server_instance != self.hello.server_instance
            || wire.workspace_id != self.hello.workspace_id
        {
            return Err(Error::IdentityChanged);
        }
        let age = now.saturating_sub(wire.generated_at_unix_ms);
        if !(1..=1000).contains(&wire.max_age_ms)
            || wire.generated_at_unix_ms > now.saturating_add(50)
            || age >= wire.max_age_ms
        {
            return Err(Error::Stale);
        }
        if wire.reason.as_ref().is_some_and(|s| !bounded(s, 512))
            || !(1..=SAFE).contains(&wire.oldest_sequence)
            || !(1..=SAFE).contains(&wire.next_sequence)
            || wire.chunks.len() > 32
        {
            return Err(Error::Invalid);
        }
        if wire.state == AudioState::Running {
            if !wire.descriptor.as_ref().is_some_and(AudioDescriptor::valid) {
                return Err(Error::Invalid);
            }
        } else if wire.descriptor.is_some() || !wire.chunks.is_empty() {
            return Err(Error::Invalid);
        }
        let reset =
            wire.gap || wire.state != AudioState::Running || self.descriptor != wire.descriptor;
        let mut expected_frame = if reset { None } else { self.next_frame };
        let mut expected_sequence = if self.after == 0 || wire.gap {
            Some(wire.oldest_sequence)
        } else {
            Some(self.after + 1)
        };
        let mut chunks = Vec::with_capacity(wire.chunks.len());
        let mut bytes_total = 0usize;
        let mut last_sequence = None;
        for c in wire.chunks {
            if !(1..=SAFE - 1).contains(&c.sequence)
                || c.sequence < wire.oldest_sequence
                || c.sequence <= self.after
                || expected_sequence.is_some_and(|s| c.sequence != s)
                || !(1..=4096).contains(&c.frame_count)
                || c.first_frame > SAFE - u64::from(c.frame_count)
                || expected_frame.is_some_and(|f| c.first_frame != f)
                || c.calibration_uncertainty_us > 50_000
                || c.sample0_unix_us == 0
                || c.sample0_unix_us > now.saturating_add(50).saturating_mul(1000)
                || now.saturating_mul(1000).saturating_sub(c.sample0_unix_us) > 3_000_000
                || c.pcm_base64.len() > 10924
            {
                return Err(Error::Invalid);
            }
            let bytes = STANDARD.decode(c.pcm_base64).map_err(|_| Error::Invalid)?;
            bytes_total += bytes.len();
            if bytes.len() != c.frame_count as usize * 2 || bytes_total > 262144 {
                return Err(Error::Invalid);
            }
            if bytes_total / 2
                > wire.descriptor.as_ref().ok_or(Error::Invalid)?.sample_rate as usize * 2
            {
                return Err(Error::Invalid);
            }
            expected_frame = Some(c.first_frame + u64::from(c.frame_count));
            expected_sequence = Some(c.sequence + 1);
            last_sequence = Some(c.sequence);
            chunks.push(AudioChunk {
                sequence: c.sequence,
                first_frame: c.first_frame,
                sample0_unix_us: c.sample0_unix_us,
                calibration_uncertainty_us: c.calibration_uncertainty_us,
                samples: bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|p| i16::from_le_bytes([p[0], p[1]]))
                    .collect(),
            });
        }
        if wire.next_sequence != last_sequence.unwrap_or(self.after) + 1 {
            return Err(Error::Invalid);
        }
        if !wire.gap && self.after != 0 && self.after < wire.oldest_sequence.saturating_sub(1) {
            return Err(Error::Invalid);
        }
        self.after = last_sequence.unwrap_or(if wire.gap { 0 } else { self.after });
        self.descriptor = wire.descriptor.clone();
        self.next_frame = expected_frame;
        Ok(AudioBatch {
            state: wire.state,
            reason: wire.reason,
            descriptor: wire.descriptor,
            reset,
            chunks,
            expires: Instant::now() + Duration::from_millis(wire.max_age_ms - age),
            session: self.session.clone(),
            receiver: self.alive.clone(),
        })
    }
}

impl Drop for AudioClient {
    fn drop(&mut self) {
        self.alive.store(false, Ordering::Release);
    }
}
