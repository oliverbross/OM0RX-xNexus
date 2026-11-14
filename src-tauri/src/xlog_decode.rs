//! Receive-only DSP seam. Dormant until xLog advertises qualified owner PCM.
//! No Engine, StationCore, device, socket, persistence or transmit handler exists here.
//! Call on a bounded worker, never on the UI or audio callback thread.

use tempo_core::modes::{self, DecodeRequest, NativeSource, SignalSource};

pub const SAMPLE_RATE: u32 = 12_000;
pub const MAX_UTC_UNCERTAINTY_MS: u32 = 50;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum DecoderMode {
    Ft8,
    Ft4,
}

impl DecoderMode {
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
    pub period_start_unix_ms: u64,
    /// Owner monotonic frame clock, independently anchored from UTC.
    pub frame_time_ms: i64,
    pub utc_uncertainty_ms: u32,
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<i16>,
}

impl CompleteFrame {
    pub fn validate(&self) -> Result<(), String> {
        for id in [
            &self.context.stream_id,
            &self.context.radio_id,
            &self.context.receiver_id,
        ] {
            if id.trim().is_empty() || id.len() > 128 || id.chars().any(char::is_control) {
                return Err("missing or invalid owner audio identity".into());
            }
        }
        if self.context.dial_hz == 0 {
            return Err("owner dial frequency is unavailable".into());
        }
        if self.sample_rate != SAMPLE_RATE || self.channels != 1 {
            return Err("decoder requires owner 12000 Hz mono PCM".into());
        }
        if self.samples.len() != self.context.mode.frame_samples() {
            return Err("incomplete or oversized decode frame; padding is forbidden".into());
        }
        if self.utc_uncertainty_ms > MAX_UTC_UNCERTAINTY_MS
            || self.period_start_unix_ms == 0
            || self.period_start_unix_ms % self.context.mode.period_ms() != 0
            || self.frame_time_ms < 0
        {
            return Err("owner audio has no qualified UTC period anchor".into());
        }
        Ok(())
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
    pub period_start_unix_ms: u64,
    pub rows: Vec<DecodeRow>,
}

#[derive(Default)]
pub struct Decoder {
    source: Option<NativeSource>,
    context: Option<DecodeContext>,
    last_period: Option<(u64, i64)>,
}

impl Decoder {
    /// Drop the current stream after disconnect, QSY, clock/generation change or a gap.
    pub fn reset(&mut self) {
        self.source = None;
        self.context = None;
        self.last_period = None;
        // This function takes the existing process-wide modem guard internally.
        modes::reset_ft8_a7();
    }

    pub fn decode(&mut self, frame: CompleteFrame) -> Result<DecodedPeriod, String> {
        frame.validate()?;
        if self.context.as_ref() != Some(&frame.context) {
            self.reset();
        }
        if let Some((utc, monotonic)) = self.last_period {
            if frame.period_start_unix_ms <= utc || frame.frame_time_ms <= monotonic {
                return Err("duplicate or reordered decode period".into());
            }
            if frame.period_start_unix_ms - utc != frame.context.mode.period_ms() {
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
        self.last_period = Some((frame.period_start_unix_ms, frame.frame_time_ms));
        Ok(DecodedPeriod {
            context: frame.context,
            period_start_unix_ms: frame.period_start_unix_ms,
            rows: decoded
                .into_iter()
                .map(|d| DecodeRow {
                    message: d.message,
                    snr_db: d.snr,
                    time_offset_seconds: d.dt,
                    audio_frequency_hz: d.freq,
                })
                .collect(),
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
                generation: 1,
                radio_id: "owner-radio".into(),
                receiver_id: "owner-receiver".into(),
                mode,
                dial_hz: 14_074_000,
            },
            period_start_unix_ms: 1_800_000,
            frame_time_ms: 1_000,
            utc_uncertainty_ms: 20,
            sample_rate: SAMPLE_RATE,
            channels: 1,
            samples: vec![0; mode.frame_samples()],
        }
    }

    #[test]
    fn exact_mode_frames_are_accepted_without_assuming_slot_sample_count() {
        for mode in [DecoderMode::Ft8, DecoderMode::Ft4] {
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
        f.utc_uncertainty_ms = MAX_UTC_UNCERTAINTY_MS + 1;
        assert!(f.validate().is_err());
        f = frame(DecoderMode::Ft8);
        f.period_start_unix_ms += 1;
        assert!(f.validate().is_err());
        f = frame(DecoderMode::Ft8);
        f.sample_rate = 48_000;
        assert!(f.validate().is_err());
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
        }
    }
}
