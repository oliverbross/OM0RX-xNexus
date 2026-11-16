//! Receive-only DSP seam for owner-authorized, host-clocked endpoint PCM.
//! No Engine, StationCore, device, socket, persistence or transmit handler exists here.
//! Call on a bounded worker, never on the UI or audio callback thread.

use tempo_core::modes::{self, DecodeRequest, NativeSource, SignalSource};
use xnexus_xlog_rx::{
    ClockProvenance, CompletePeriod, OwnerIdentity, ReceiveContext, RxMode, TimestampBasis,
    MAX_HOST_UNCERTAINTY_US,
};

pub const SAMPLE_RATE: u32 = 12_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum DecoderMode {
    Ft8,
    Ft4,
}

impl DecoderMode {
    fn receive_mode(self) -> RxMode {
        match self {
            Self::Ft8 => RxMode::Ft8,
            Self::Ft4 => RxMode::Ft4,
        }
    }
    fn kind(self) -> modes::ModeKind {
        match self {
            Self::Ft8 => modes::ModeKind::Ft8,
            Self::Ft4 => modes::ModeKind::Ft4,
        }
    }

    pub fn frame_samples(self) -> usize {
        self.kind().frame_samples()
    }

    pub fn period_ms(self) -> u64 {
        match self {
            Self::Ft8 => 15_000,
            Self::Ft4 => 7_500,
        }
    }
}

/// Every coordinate is supplied by xLog's owner; never infer a radio from an audio device.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct DecodeContext {
    pub stream_id: String,
    pub epoch: String,
    pub mapping_generation: u64,
    pub generation: u64,
    pub radio_id: String,
    pub receiver_id: String,
    pub mode: DecoderMode,
    pub dial_hz: u64,
}

/// An assembler must prove continuity before constructing this complete period.
/// FT4 consumes the mode's frame size from the START of its 7.5 s period, not its tail.
pub struct CompleteFrame {
    pub context: DecodeContext,
    pub owner: OwnerIdentity,
    pub host_period_start_unix_us: u64,
    /// Owner sample-counter time, independent of the host wall clock.
    pub frame_time_ms: i64,
    pub clock: ClockProvenance,
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

impl CompleteFrame {
    pub fn from_period(period: CompletePeriod) -> Result<Self, String> {
        let owner = period.context.owner;
        let mode = match period.context.mode {
            RxMode::Ft8 => DecoderMode::Ft8,
            RxMode::Ft4 => DecoderMode::Ft4,
        };
        let frame = Self {
            context: DecodeContext {
                stream_id: owner.capture_id.clone(),
                epoch: owner.epoch.clone(),
                mapping_generation: owner.mapping_generation,
                generation: period.generation,
                radio_id: owner.radio_id.clone(),
                receiver_id: owner.receiver_id.clone(),
                mode,
                dial_hz: owner.frequency_hz,
            },
            owner,
            host_period_start_unix_us: period.host_period_start_unix_us,
            frame_time_ms: period.frame_time_ms,
            clock: period.clock,
            sample_rate: SAMPLE_RATE,
            channels: 1,
            samples: period.samples,
        };
        frame.validate()?;
        Ok(frame)
    }

    pub fn validate(&self) -> Result<(), String> {
        ReceiveContext {
            owner: self.owner.clone(),
            mode: self.context.mode.receive_mode(),
        }
        .validate()
        .map_err(|_| "missing or invalid owner audio identity")?;
        if self.context.stream_id != self.owner.capture_id
            || self.context.epoch != self.owner.epoch
            || self.context.mapping_generation != self.owner.mapping_generation
            || self.context.generation == 0
            || self.context.radio_id != self.owner.radio_id
            || self.context.receiver_id != self.owner.receiver_id
            || self.context.dial_hz != self.owner.frequency_hz
        {
            return Err("decode context does not match owner audio identity".into());
        }
        if self.sample_rate != SAMPLE_RATE || self.channels != 1 {
            return Err("decoder requires owner 12000 Hz mono PCM".into());
        }
        if self.samples.len() != self.context.mode.frame_samples() {
            return Err("incomplete or oversized decode frame; padding is forbidden".into());
        }
        let expected_center_ns = if self.owner.sample_rate == SAMPLE_RATE {
            0
        } else {
            63_000_000_000_u64.div_ceil(2 * u64::from(self.owner.sample_rate))
        };
        if self.clock.basis != TimestampBasis::WindowsEndpointCapture
            || self.clock.host_mapping_uncertainty_us > MAX_HOST_UNCERTAINTY_US
            || self.clock.calibration_uncertainty_us > self.clock.host_mapping_uncertainty_us
            || self.clock.utc_sync_accuracy_us.is_some()
            || self.clock.remote_rf_latency_us.is_some()
            || self.clock.sample_alignment_offset_ns > 83_334
            || self.clock.resampler_center_offset_ns != expected_center_ns
            || self.host_period_start_unix_us == 0
            || self.host_period_start_unix_us % (self.context.mode.period_ms() * 1000) != 0
            || self.frame_time_ms < 0
        {
            return Err("owner audio has no valid host-clock period anchor; UTC/RF timing remains unverified".into());
        }
        Ok(())
    }
}

/// Presentation preserves unknown timing facts rather than projecting a calibrated RF clock.
#[derive(Debug, serde::Serialize)]
pub struct DecodeClock {
    pub timestamp_basis: &'static str,
    pub host_mapping_uncertainty_us: u32,
    pub calibration_uncertainty_us: u32,
    pub utc_sync_accuracy_us: Option<u32>,
    pub remote_rf_latency_us: Option<i64>,
    pub sample_alignment_offset_ns: u64,
    pub resampler_center_offset_ns: u64,
}

impl From<ClockProvenance> for DecodeClock {
    fn from(clock: ClockProvenance) -> Self {
        Self {
            timestamp_basis: "windows_endpoint_capture",
            host_mapping_uncertainty_us: clock.host_mapping_uncertainty_us,
            calibration_uncertainty_us: clock.calibration_uncertainty_us,
            utc_sync_accuracy_us: clock.utc_sync_accuracy_us,
            remote_rf_latency_us: clock.remote_rf_latency_us,
            sample_alignment_offset_ns: clock.sample_alignment_offset_ns,
            resampler_center_offset_ns: clock.resampler_center_offset_ns,
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DecodeRow {
    pub message: String,
    pub snr_db: i32,
    pub time_offset_seconds: f32,
    pub audio_frequency_hz: f32,
}

#[derive(Debug, serde::Serialize)]
pub struct DecodedPeriod {
    pub context: DecodeContext,
    pub host_period_start_unix_us: u64,
    pub clock: DecodeClock,
    pub rows: Vec<DecodeRow>,
}

/// Actual native records and their exact receive provenance. This type is not an
/// IPC DTO: it deliberately has no serialization or clone implementation. Future
/// native consumers must run only after the worker accepts the job's authority.
#[derive(Debug)]
pub struct NativeDecodedPeriod {
    pub context: DecodeContext,
    pub owner: OwnerIdentity,
    pub host_period_start_unix_us: u64,
    pub frame_time_ms: i64,
    pub clock: ClockProvenance,
    pub rows: Vec<modes::Decode>,
}

impl NativeDecodedPeriod {
    /// Borrowed UI projection while the exact native period is retained in the
    /// bounded TX inbox. No decode metric is reconstructed from this projection.
    pub fn presentation(&self) -> DecodedPeriod {
        DecodedPeriod {
            context: self.context.clone(),
            host_period_start_unix_us: self.host_period_start_unix_us,
            clock: self.clock.clone().into(),
            rows: self
                .rows
                .iter()
                .map(|d| DecodeRow {
                    message: d.message.clone(),
                    snr_db: d.snr,
                    time_offset_seconds: d.dt,
                    audio_frequency_hz: d.freq,
                })
                .collect(),
        }
    }
    /// Consume the typed period at the existing presentation boundary. Raw decode
    /// metrics and owner coordinates are never reconstructed from this UI DTO.
    pub fn into_presentation(self) -> DecodedPeriod {
        DecodedPeriod {
            context: self.context,
            host_period_start_unix_us: self.host_period_start_unix_us,
            clock: self.clock.into(),
            rows: self
                .rows
                .into_iter()
                .map(|d| DecodeRow {
                    message: d.message,
                    snr_db: d.snr,
                    time_offset_seconds: d.dt,
                    audio_frequency_hz: d.freq,
                })
                .collect(),
        }
    }
}

#[derive(Default)]
pub struct Decoder {
    source: Option<NativeSource>,
    context: Option<DecodeContext>,
    owner: Option<OwnerIdentity>,
    last_period: Option<(u64, i64)>,
}

impl Decoder {
    /// Drop the current stream after disconnect, QSY, clock/generation change or a gap.
    pub fn reset(&mut self) {
        self.source = None;
        self.context = None;
        self.owner = None;
        self.last_period = None;
        // This function takes the existing process-wide modem guard internally.
        modes::reset_ft8_a7();
    }

    pub fn decode(&mut self, frame: CompleteFrame) -> Result<DecodedPeriod, String> {
        self.decode_native(frame)
            .map(NativeDecodedPeriod::into_presentation)
    }

    pub fn decode_native(&mut self, frame: CompleteFrame) -> Result<NativeDecodedPeriod, String> {
        if let Err(error) = frame.validate() {
            self.reset();
            return Err(error);
        }
        if self.context.as_ref() != Some(&frame.context)
            || self.owner.as_ref() != Some(&frame.owner)
        {
            self.reset();
        }
        if let Some((host_time, monotonic)) = self.last_period {
            if frame.host_period_start_unix_us <= host_time || frame.frame_time_ms <= monotonic {
                self.reset();
                return Err("duplicate or reordered decode period".into());
            }
            if frame.host_period_start_unix_us - host_time != frame.context.mode.period_ms() * 1000
            {
                self.reset();
            }
        }
        let source = self
            .source
            .get_or_insert_with(|| NativeSource::from_kind(frame.context.mode.kind()));
        let mut request = DecodeRequest::full_band(&frame.samples);
        request.frame_time_ms = frame.frame_time_ms;
        // Initial receive-only operation has no QSO/AP context and no cross-cycle a7
        // replay. It must not borrow calls from another stream's sequencing state.
        request.ap = false;
        request.ndepth = 1;
        let decoded =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| source.decode(&request)));
        let decoded = match decoded {
            Ok(rows) => rows,
            Err(_) => {
                self.reset();
                return Err("native decoder failed; receive context was discarded".into());
            }
        };
        self.context = Some(frame.context.clone());
        self.owner = Some(frame.owner.clone());
        self.last_period = Some((frame.host_period_start_unix_us, frame.frame_time_ms));
        Ok(NativeDecodedPeriod {
            context: frame.context,
            owner: frame.owner,
            host_period_start_unix_us: frame.host_period_start_unix_us,
            frame_time_ms: frame.frame_time_ms,
            clock: frame.clock,
            rows: decoded,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(mode: DecoderMode) -> CompleteFrame {
        CompleteFrame {
            context: DecodeContext {
                stream_id: "owner-stream".into(),
                epoch: "owner-epoch".into(),
                mapping_generation: 1,
                generation: 1,
                radio_id: "owner-radio".into(),
                receiver_id: "owner-receiver".into(),
                mode,
                dial_hz: 14_074_000,
            },
            owner: owner(SAMPLE_RATE),
            host_period_start_unix_us: 1_800_000_000,
            frame_time_ms: 1_000,
            clock: ClockProvenance {
                basis: TimestampBasis::WindowsEndpointCapture,
                host_mapping_uncertainty_us: 20_000,
                calibration_uncertainty_us: 20_000,
                utc_sync_accuracy_us: None,
                remote_rf_latency_us: None,
                sample_alignment_offset_ns: 0,
                resampler_center_offset_ns: 0,
            },
            sample_rate: SAMPLE_RATE,
            channels: 1,
            samples: vec![0; mode.frame_samples()],
        }
    }

    fn owner(sample_rate: u32) -> OwnerIdentity {
        OwnerIdentity {
            server_instance: "owner-server".into(),
            workspace_id: "owner-workspace".into(),
            capture_id: "owner-stream".into(),
            epoch: "owner-epoch".into(),
            radio_id: "owner-radio".into(),
            receiver_id: "owner-receiver".into(),
            mapping_generation: 1,
            device_id: "owner-selected-endpoint".into(),
            slice: "A".into(),
            dax_channel: 1,
            frequency_hz: 14_074_000,
            radio_mode: "DIGU".into(),
            sample_rate,
            source_channels: 2,
            selected_channel: 0,
        }
    }

    #[test]
    fn exact_mode_frames_are_accepted_without_assuming_slot_sample_count() {
        for mode in [DecoderMode::Ft8, DecoderMode::Ft4] {
            assert_eq!(mode.frame_samples(), mode.receive_mode().frame_samples());
            let mut f = frame(mode);
            assert!(f.validate().is_ok());
            f.samples.pop();
            assert!(f.validate().is_err());
            f.samples.extend([0, 0]);
            assert!(f.validate().is_err());
        }
        assert_eq!(DecoderMode::Ft8.frame_samples(), 180_000);
        assert_ne!(DecoderMode::Ft4.frame_samples(), 90_000);
    }

    #[test]
    fn owner_coordinates_and_clock_cannot_be_inferred() {
        let mut f = frame(DecoderMode::Ft8);
        f.context.radio_id.clear();
        assert!(f.validate().is_err());
        f = frame(DecoderMode::Ft8);
        f.clock.host_mapping_uncertainty_us = MAX_HOST_UNCERTAINTY_US + 1;
        assert!(f.validate().is_err());
        f = frame(DecoderMode::Ft8);
        f.host_period_start_unix_us += 1;
        assert!(f.validate().is_err());
        f = frame(DecoderMode::Ft8);
        f.clock.utc_sync_accuracy_us = Some(1);
        assert!(f.validate().is_err());
        f = frame(DecoderMode::Ft8);
        f.clock.remote_rf_latency_us = Some(0);
        assert!(f.validate().is_err());
        f = frame(DecoderMode::Ft8);
        f.sample_rate = 48_000;
        assert!(f.validate().is_err());
        f = frame(DecoderMode::Ft8);
        f.clock.resampler_center_offset_ns = 1;
        assert!(f.validate().is_err());
    }

    #[test]
    fn rejected_frame_discards_previous_decoder_identity_and_timing() {
        let mut f = frame(DecoderMode::Ft8);
        let mut decoder = Decoder {
            source: None,
            context: Some(f.context.clone()),
            owner: Some(f.owner.clone()),
            last_period: Some((15_000_000, 1000)),
        };
        f.context.radio_id.clear();
        assert!(decoder.decode(f).is_err());
        assert!(decoder.context.is_none());
        assert!(decoder.owner.is_none());
        assert!(decoder.last_period.is_none());
    }

    fn generated_frame(mode: DecoderMode) -> CompleteFrame {
        let modem = modes::make_mode(mode.kind());
        let tones = modem.encode("CQ K1ABC FN42");
        assert!(!tones.is_empty());
        let wave = modem.gen_wave(&tones, SAMPLE_RATE as f32, 1_500.0);
        let mut frame = frame(mode);
        for (dest, sample) in frame.samples.iter_mut().zip(wave) {
            *dest = (sample * 1000.0).clamp(-32768.0, 32767.0) as i16;
        }
        frame
    }

    #[test]
    fn actual_native_records_preserve_owner_context_and_clock_before_presentation() {
        for mode in [DecoderMode::Ft8, DecoderMode::Ft4] {
            let mut frame = generated_frame(mode);
            frame.owner = owner(48_000);
            frame.owner.workspace_id = "exact-owner-workspace".into();
            frame.owner.device_id = "exact-selected-device".into();
            frame.owner.dax_channel = 3;
            frame.owner.selected_channel = 1;
            frame.frame_time_ms = 987_654_321;
            frame.clock.host_mapping_uncertainty_us = 23_456;
            frame.clock.calibration_uncertainty_us = 17;
            frame.clock.sample_alignment_offset_ns = 31_234;
            frame.clock.resampler_center_offset_ns = 656_250;
            frame.validate().unwrap();
            let expected_owner = frame.owner.clone();
            let owner_device_allocation = frame.owner.device_id.as_ptr();
            let expected_context = frame.context.clone();
            let expected_clock = frame.clock.clone();
            let host_start = frame.host_period_start_unix_us;
            let frame_time = frame.frame_time_ms;
            // Independent actual decoder result includes metrics absent from the
            // UI DTO. This second DSP invocation is a test oracle only.
            let mut reference = NativeSource::from_kind(mode.kind());
            let mut request = DecodeRequest::full_band(&frame.samples);
            request.frame_time_ms = frame_time;
            request.ap = false;
            request.ndepth = 1;
            let expected_records = reference.decode(&request);
            assert!(expected_records
                .iter()
                .any(|row| row.message.contains("K1ABC")));
            let mut decoder = Decoder::default();
            let native = decoder.decode_native(frame).unwrap();
            assert_eq!(native.rows, expected_records);
            assert!(native
                .rows
                .iter()
                .all(|row| row.mode == Some(mode.kind()) && row.nap == 0));
            assert_eq!(native.owner, expected_owner);
            assert_eq!(native.owner.device_id.as_ptr(), owner_device_allocation);
            assert_eq!(decoder.owner.as_ref(), Some(&expected_owner));
            assert_eq!(native.context, expected_context);
            assert_eq!(native.host_period_start_unix_us, host_start);
            assert_eq!(native.frame_time_ms, frame_time);
            assert_eq!(native.clock, expected_clock);
            assert_eq!(decoder.last_period, Some((host_start, frame_time)));

            let display = native.into_presentation();
            assert_eq!(display.context, expected_context);
            assert_eq!(display.host_period_start_unix_us, host_start);
            assert_eq!(display.rows.len(), expected_records.len());
            for (row, actual) in display.rows.iter().zip(&expected_records) {
                assert_eq!(row.message, actual.message);
                assert_eq!(row.snr_db, actual.snr);
                assert_eq!(row.time_offset_seconds, actual.dt);
                assert_eq!(row.audio_frequency_hz, actual.freq);
            }
            let json = serde_json::to_value(display).unwrap();
            let object = json.as_object().unwrap();
            assert_eq!(object.len(), 4);
            for field in ["context", "host_period_start_unix_us", "clock", "rows"] {
                assert!(object.contains_key(field));
            }
            for row in json["rows"].as_array().unwrap() {
                assert_eq!(row.as_object().unwrap().len(), 4);
                for field in [
                    "message",
                    "snr_db",
                    "time_offset_seconds",
                    "audio_frequency_hz",
                ] {
                    assert!(row.get(field).is_some());
                }
            }
            assert_eq!(
                json["clock"]["utc_sync_accuracy_us"],
                serde_json::Value::Null
            );
            assert_eq!(
                json["clock"]["remote_rf_latency_us"],
                serde_json::Value::Null
            );
            assert_eq!(json["clock"]["host_mapping_uncertainty_us"], 23_456);
            assert_eq!(json["clock"]["calibration_uncertainty_us"], 17);
            assert_eq!(json["clock"]["sample_alignment_offset_ns"], 31_234);
            assert_eq!(json["clock"]["resampler_center_offset_ns"], 656_250);
        }
    }

    #[test]
    fn compatibility_wrapper_preserves_generated_presentation() {
        for mode in [DecoderMode::Ft8, DecoderMode::Ft4] {
            let native = Decoder::default()
                .decode_native(generated_frame(mode))
                .unwrap();
            let expected = serde_json::to_value(native.into_presentation()).unwrap();
            let legacy = Decoder::default().decode(generated_frame(mode)).unwrap();
            assert_eq!(serde_json::to_value(legacy).unwrap(), expected);
        }
    }

    #[test]
    fn native_period_duplicate_reset_and_full_owner_change_keep_existing_fences() {
        let mode = DecoderMode::Ft4;
        let mut decoder = Decoder::default();
        let first = decoder.decode_native(generated_frame(mode)).unwrap();
        assert!(first.rows.iter().any(|row| row.message.contains("K1ABC")));
        assert_eq!(
            decoder.decode_native(generated_frame(mode)).unwrap_err(),
            "duplicate or reordered decode period"
        );
        assert!(decoder.source.is_none());
        assert!(decoder.context.is_none());
        assert!(decoder.owner.is_none());
        assert!(decoder.last_period.is_none());
        // Existing reset semantics permit a new stream to establish its first
        // period; this adapter does not add a cross-reset sequence highwater.
        decoder.decode_native(generated_frame(mode)).unwrap();
        let mut changed_owner = generated_frame(mode);
        changed_owner.owner.workspace_id = "replacement-workspace".into();
        let expected_owner = changed_owner.owner.clone();
        let native = decoder.decode_native(changed_owner).unwrap();
        assert_eq!(native.owner, expected_owner);
        assert_eq!(decoder.owner.as_ref(), Some(&expected_owner));
        let mut invalid = generated_frame(mode);
        invalid.clock.host_mapping_uncertainty_us = MAX_HOST_UNCERTAINTY_US + 1;
        assert!(decoder.decode_native(invalid).is_err());
        assert!(decoder.source.is_none());
        assert!(decoder.owner.is_none());
        assert!(decoder.last_period.is_none());
        decoder.reset();
        assert!(decoder.context.is_none());
    }

    #[test]
    fn generated_ft8_and_ft4_messages_decode_without_a_station() {
        for mode in [DecoderMode::Ft8, DecoderMode::Ft4] {
            let modem = modes::make_mode(mode.kind());
            let tones = modem.encode("CQ K1ABC FN42");
            assert!(!tones.is_empty());
            let wave = modem.gen_wave(&tones, SAMPLE_RATE as f32, 1_500.0);
            let mut f = frame(mode);
            for (dest, sample) in f.samples.iter_mut().zip(wave) {
                *dest = (sample * 1000.0).clamp(-32768.0, 32767.0) as i16;
            }
            let rows = Decoder::default().decode(f).unwrap();
            assert!(rows.rows.iter().any(|d| d.message.contains("K1ABC")));
            assert_eq!(rows.clock.utc_sync_accuracy_us, None);
            assert_eq!(rows.clock.remote_rf_latency_us, None);
        }
    }

    #[test]
    fn generated_native_rate_owner_chunks_assemble_and_decode_without_a_station() {
        use xnexus_xlog_rx::{PcmChunk, RxAssembler};
        for rate in [12_000, 24_000, 44_100, 48_000, 96_000] {
            for mode in [DecoderMode::Ft8, DecoderMode::Ft4] {
                let modem = modes::make_mode(mode.kind());
                let tones = modem.encode("CQ K1ABC FN42");
                assert!(!tones.is_empty());
                // Nexus's FT8/FT4 generator fixes samples-per-symbol at 12 kHz;
                // changing fsample alone would change baud, not the capture rate.
                let wave = modem.gen_wave(&tones, SAMPLE_RATE as f32, 1500.0);
                // Explicit fixture capture starts at host13s, with real background
                // samples before the transmitted waveform's host15s slot boundary.
                let mut capture = vec![0_i16; rate as usize * 18];
                // Independent linear interpolation supplies the synthetic endpoint
                // rate while preserving the real modem's symbol timing. This is a
                // fixture only; the receiver uses the anti-aliased upstream FIR.
                for (index, dest) in capture.iter_mut().skip(rate as usize * 2).enumerate() {
                    let position = index as u64 * u64::from(SAMPLE_RATE);
                    let whole = (position / u64::from(rate)) as usize;
                    if whole + 1 >= wave.len() {
                        break;
                    }
                    let fraction = (position % u64::from(rate)) as f32 / rate as f32;
                    let sample = wave[whole] * (1.0 - fraction) + wave[whole + 1] * fraction;
                    *dest = (sample * 1000.0).round().clamp(-32768.0, 32767.0) as i16;
                }
                let context = ReceiveContext {
                    owner: owner(rate),
                    mode: mode.receive_mode(),
                };
                let mut assembler = RxAssembler::default();
                let mut offset = 0;
                let mut sequence = 1;
                let sizes = [127, 4096, 63, 1];
                let mut completed = None;
                while offset < capture.len() {
                    let count =
                        sizes[(sequence as usize - 1) % sizes.len()].min(capture.len() - offset);
                    completed = assembler
                        .push(
                            &context,
                            PcmChunk {
                                sequence,
                                first_frame: offset as u64,
                                sample0_unix_us: 13_000_000
                                    + offset as u64 * 1_000_000 / u64::from(rate),
                                calibration_uncertainty_us: 10,
                                samples: &capture[offset..offset + count],
                            },
                        )
                        .unwrap();
                    if completed.is_some() {
                        break;
                    }
                    offset += count;
                    sequence += 1;
                }
                let period = completed.expect("complete real PCM period");
                assert_eq!(period.host_period_start_unix_us, 15_000_000);
                let decoded = Decoder::default()
                    .decode(CompleteFrame::from_period(period).unwrap())
                    .unwrap();
                assert!(
                    decoded.rows.iter().any(|row| row.message.contains("K1ABC")),
                    "rate={rate} mode={mode:?} rows={:?}",
                    decoded.rows
                );
                assert_eq!(decoded.clock.utc_sync_accuracy_us, None);
                assert_eq!(decoded.clock.remote_rf_latency_us, None);
            }
        }
    }
}
