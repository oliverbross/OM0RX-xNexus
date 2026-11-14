// SPDX-License-Identifier: GPL-3.0-only
//! Receive-only assembly from owner-authorized native-rate mono PCM. This crate
//! has no device, Engine, logbook, transport, transmit or GUI API. Owner timestamps
//! describe the Windows endpoint's HOST clock; UTC synchronization and RF/DAX
//! latency remain unknown. All emitted samples derive from contiguous owner PCM.
#![forbid(unsafe_code)]

use tempo_codec::capture_resample::CaptureResampler;

pub const OUTPUT_RATE: u32 = 12_000;
pub const MAX_CHUNK_FRAMES: usize = 4096;
pub const MAX_HOST_UNCERTAINTY_US: u32 = 50_000;

fn ceil_div(value: i128, divisor: i128) -> i128 {
    value.div_euclid(divisor) + i128::from(value.rem_euclid(divisor) != 0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RxMode {
    Ft8,
    Ft4,
}

impl RxMode {
    pub const fn period_us(self) -> u64 {
        match self {
            Self::Ft8 => 15_000_000,
            Self::Ft4 => 7_500_000,
        }
    }
    /// Nexus ModeKind's native frame size; FT4's 6.048 s decode window is
    /// deliberately shorter than its 7.5 s slot. The adapter tests this contract.
    pub const fn frame_samples(self) -> usize {
        match self {
            Self::Ft8 => 180_000,
            Self::Ft4 => 72_576,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerIdentity {
    pub server_instance: String,
    pub workspace_id: String,
    pub capture_id: String,
    pub epoch: String,
    pub radio_id: String,
    pub receiver_id: String,
    pub mapping_generation: u64,
    pub device_id: String,
    pub slice: String,
    pub dax_channel: u8,
    pub frequency_hz: u64,
    /// Hardware mode, such as DIGU; it does not select the FT decoder.
    pub radio_mode: String,
    pub sample_rate: u32,
    pub source_channels: u8,
    pub selected_channel: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReceiveContext {
    pub owner: OwnerIdentity,
    /// Explicit local receive decoder selection, independent of the radio mode.
    pub mode: RxMode,
}

impl ReceiveContext {
    pub fn validate(&self) -> Result<(), RxError> {
        let owner = &self.owner;
        for id in [
            &owner.server_instance,
            &owner.workspace_id,
            &owner.capture_id,
            &owner.epoch,
            &owner.radio_id,
            &owner.receiver_id,
            &owner.device_id,
            &owner.radio_mode,
        ] {
            if id.trim().is_empty() || id.len() > 4096 || id.chars().any(char::is_control) {
                return Err(RxError::InvalidContext);
            }
        }
        if !(8000..=192000).contains(&owner.sample_rate)
            || !(1..=16).contains(&owner.source_channels)
            || owner.selected_channel >= owner.source_channels
            || owner.slice != "A"
            || owner.dax_channel == 0
            || owner.frequency_hz == 0
            || owner.mapping_generation == 0
        {
            return Err(RxError::InvalidContext);
        }
        Ok(())
    }
}

/// PCM has already been size-checked and decoded by the authenticated transport.
/// Each element is the explicitly selected channel, never a downmix.
pub struct PcmChunk<'a> {
    pub sequence: u64,
    pub first_frame: u64,
    pub sample0_unix_us: u64,
    pub calibration_uncertainty_us: u32,
    pub samples: &'a [i16],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimestampBasis {
    WindowsEndpointCapture,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClockProvenance {
    pub basis: TimestampBasis,
    /// Includes observed nominal-rate timing residual, not merely the owner's
    /// QPC-to-host-wall calibration bracket. Never describes global UTC accuracy.
    pub host_mapping_uncertainty_us: u32,
    pub calibration_uncertainty_us: u32,
    pub utc_sync_accuracy_us: Option<u32>,
    pub remote_rf_latency_us: Option<i64>,
    /// First real output sample's offset after the nominal HOST slot boundary.
    pub sample_alignment_offset_ns: u64,
    /// Filter's input-center offset, zero for 12 kHz passthrough.
    pub resampler_center_offset_ns: u64,
}

#[derive(Debug)]
pub struct CompletePeriod {
    pub context: ReceiveContext,
    /// Changes on every discard; workers must reject results from an old generation.
    pub generation: u64,
    pub host_period_start_unix_us: u64,
    /// Sample-counter time within this capture epoch, independent of host wall time.
    pub frame_time_ms: i64,
    pub clock: ClockProvenance,
    pub samples: Vec<i16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RxError {
    InvalidContext,
    InvalidChunk,
    ContextChanged,
    SequenceDiscontinuity,
    FrameDiscontinuity,
    ClockDiscontinuity,
    CounterOverflow,
}

struct Pending {
    start_us: u64,
    first_output: u64,
    alignment_ns: u64,
    samples: Vec<i16>,
}

/// A single stream, with at most one native decode frame retained. An error
/// discards the partial period and FIR history; the next chunk re-primes from
/// real input and waits for the next whole host-clock slot. No silence is added.
pub struct RxAssembler {
    context: Option<ReceiveContext>,
    resampler: Option<CaptureResampler>,
    generation: u64,
    exhausted: bool,
    next_sequence: u64,
    next_frame: u64,
    origin_frame: u64,
    origin_us: u64,
    output_count: u64,
    next_period_us: Option<u64>,
    pending: Option<Pending>,
    uncertainty_us: u32,
    calibration_us: u32,
}

impl Default for RxAssembler {
    fn default() -> Self {
        Self {
            context: None,
            resampler: None,
            generation: 1,
            exhausted: false,
            next_sequence: 0,
            next_frame: 0,
            origin_frame: 0,
            origin_us: 0,
            output_count: 0,
            next_period_us: None,
            pending: None,
            uncertainty_us: 0,
            calibration_us: 0,
        }
    }
}

impl RxAssembler {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Call on transport gap/stop/unavailable/expiry/revoke or any owner change.
    pub fn reset(&mut self) {
        self.context = None;
        self.resampler = None;
        self.pending = None;
        self.next_period_us = None;
        self.output_count = 0;
        self.uncertainty_us = 0;
        self.calibration_us = 0;
        match self.generation.checked_add(1) {
            Some(generation) => self.generation = generation,
            None => self.exhausted = true,
        }
    }

    pub fn push(
        &mut self,
        context: &ReceiveContext,
        chunk: PcmChunk<'_>,
    ) -> Result<Option<CompletePeriod>, RxError> {
        let result = self.push_inner(context, chunk);
        if result.is_err() {
            self.reset();
        }
        result
    }

    fn push_inner(
        &mut self,
        context: &ReceiveContext,
        chunk: PcmChunk<'_>,
    ) -> Result<Option<CompletePeriod>, RxError> {
        if self.exhausted {
            return Err(RxError::CounterOverflow);
        }
        context.validate()?;
        if chunk.samples.is_empty()
            || chunk.samples.len() > MAX_CHUNK_FRAMES
            || chunk.sequence == 0
            || chunk.sample0_unix_us == 0
            || chunk.calibration_uncertainty_us > MAX_HOST_UNCERTAINTY_US
        {
            return Err(RxError::InvalidChunk);
        }
        let end_frame = chunk
            .first_frame
            .checked_add(chunk.samples.len() as u64)
            .ok_or(RxError::CounterOverflow)?;
        let next_sequence = chunk
            .sequence
            .checked_add(1)
            .ok_or(RxError::CounterOverflow)?;
        if let Some(previous) = &self.context {
            if previous != context {
                return Err(RxError::ContextChanged);
            }
            if chunk.sequence != self.next_sequence {
                return Err(RxError::SequenceDiscontinuity);
            }
            if chunk.first_frame != self.next_frame {
                return Err(RxError::FrameDiscontinuity);
            }
        } else {
            self.context = Some(context.clone());
            self.resampler = Some(CaptureResampler::new(
                context.owner.sample_rate,
                OUTPUT_RATE,
            ));
            self.origin_frame = chunk.first_frame;
            self.origin_us = chunk.sample0_unix_us;
        }
        let rate = i128::from(context.owner.sample_rate);
        let elapsed_frames = chunk
            .first_frame
            .checked_sub(self.origin_frame)
            .ok_or(RxError::FrameDiscontinuity)?;
        let observed = (i128::from(chunk.sample0_unix_us) - i128::from(self.origin_us)) * rate;
        let predicted = i128::from(elapsed_frames) * 1_000_000;
        let residual_us = ceil_div((observed - predicted).abs(), rate);
        let bound = residual_us + i128::from(chunk.calibration_uncertainty_us);
        if bound > i128::from(MAX_HOST_UNCERTAINTY_US) {
            return Err(RxError::ClockDiscontinuity);
        }
        self.uncertainty_us = self.uncertainty_us.max(bound as u32);
        self.calibration_us = self.calibration_us.max(chunk.calibration_uncertainty_us);
        self.next_sequence = next_sequence;
        self.next_frame = end_frame;
        let input: Vec<f32> = chunk
            .samples
            .iter()
            .map(|&sample| f32::from(sample) / 32768.0)
            .collect();
        let output = self
            .resampler
            .as_mut()
            .expect("initialized resampler")
            .process(&input);
        let period = context.mode.period_us();
        let frame_samples = context.mode.frame_samples();
        let denominator = 2 * rate * i128::from(OUTPUT_RATE);
        let center_twice = if context.owner.sample_rate == OUTPUT_RATE {
            0
        } else {
            63
        };
        let center_ns = ceil_div(i128::from(center_twice) * 1_000_000_000, 2 * rate) as u64;
        let mut completed = None;
        for sample in output {
            // Numerator of the physical input center read by this FIR output.
            let center = i128::from(center_twice) * i128::from(OUTPUT_RATE)
                + 2 * i128::from(self.output_count) * rate;
            let time = i128::from(self.origin_us) * denominator + center * 1_000_000;
            if self.next_period_us.is_none() {
                let start = ceil_div(time, i128::from(period) * denominator) * i128::from(period);
                self.next_period_us =
                    Some(u64::try_from(start).map_err(|_| RxError::CounterOverflow)?);
            }
            let start = self.next_period_us.expect("period initialized");
            let end = start.checked_add(period).ok_or(RxError::CounterOverflow)?;
            if time >= i128::from(end) * denominator {
                if let Some(pending) = self.pending.take() {
                    if pending.samples.len() != frame_samples || completed.is_some() {
                        return Err(RxError::FrameDiscontinuity);
                    }
                    let frame_position =
                        i128::from(self.origin_frame) * 2 * i128::from(OUTPUT_RATE)
                            + i128::from(center_twice) * i128::from(OUTPUT_RATE)
                            + 2 * i128::from(pending.first_output) * rate;
                    let frame_time_ms = frame_position * 1000 / denominator;
                    completed = Some(CompletePeriod {
                        context: context.clone(),
                        generation: self.generation,
                        host_period_start_unix_us: pending.start_us,
                        frame_time_ms: i64::try_from(frame_time_ms)
                            .map_err(|_| RxError::CounterOverflow)?,
                        clock: ClockProvenance {
                            basis: TimestampBasis::WindowsEndpointCapture,
                            host_mapping_uncertainty_us: self.uncertainty_us,
                            calibration_uncertainty_us: self.calibration_us,
                            utc_sync_accuracy_us: None,
                            remote_rf_latency_us: None,
                            sample_alignment_offset_ns: pending.alignment_ns,
                            resampler_center_offset_ns: center_ns,
                        },
                        samples: pending.samples,
                    });
                }
                self.next_period_us = Some(end);
            }
            let start = self.next_period_us.expect("period initialized");
            if time >= i128::from(start) * denominator {
                if self.pending.is_none() {
                    let offset = time - i128::from(start) * denominator;
                    // A missing initial sample cannot be replaced by leading silence.
                    if offset >= denominator * 1_000_000 / i128::from(OUTPUT_RATE) {
                        return Err(RxError::FrameDiscontinuity);
                    }
                    self.pending = Some(Pending {
                        start_us: start,
                        first_output: self.output_count,
                        alignment_ns: u64::try_from(ceil_div(offset * 1000, denominator))
                            .map_err(|_| RxError::CounterOverflow)?,
                        samples: Vec::with_capacity(frame_samples),
                    });
                }
                let pending = self.pending.as_mut().expect("period pending");
                if pending.samples.len() < frame_samples {
                    pending
                        .samples
                        .push((sample * 32768.0).round().clamp(-32768.0, 32767.0) as i16);
                }
            }
            self.output_count = self
                .output_count
                .checked_add(1)
                .ok_or(RxError::CounterOverflow)?;
        }
        Ok(completed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(rate: u32, mode: RxMode) -> ReceiveContext {
        ReceiveContext {
            mode,
            owner: OwnerIdentity {
                server_instance: "server".into(),
                workspace_id: "workspace".into(),
                capture_id: "capture".into(),
                epoch: "epoch".into(),
                radio_id: "radio1".into(),
                receiver_id: "client/sliceA".into(),
                mapping_generation: 1,
                device_id: "explicit-endpoint".into(),
                slice: "A".into(),
                dax_channel: 1,
                frequency_hz: 14_074_000,
                radio_mode: "DIGU".into(),
                sample_rate: rate,
                source_channels: 2,
                selected_channel: 0,
            },
        }
    }

    fn chunk<'a>(
        samples: &'a [i16],
        first_frame: u64,
        sequence: u64,
        rate: u32,
        start: u64,
    ) -> PcmChunk<'a> {
        PcmChunk {
            samples,
            first_frame,
            sequence,
            sample0_unix_us: start + first_frame * 1_000_000 / u64::from(rate),
            calibration_uncertainty_us: 10,
        }
    }

    fn assemble(rate: u32, mode: RxMode, pattern: &[usize]) -> CompletePeriod {
        let context = context(rate, mode);
        let mut assembler = RxAssembler::default();
        // Real fixture samples spanning a pre-roll and a complete period. The
        // assembler itself never creates that pre-roll or fills absent samples.
        let input: Vec<i16> = (0..rate as usize * 18)
            .map(|i| ((i % 997) as i16 - 498) * 20)
            .collect();
        let mut offset = 0;
        let mut index = 0;
        while offset < input.len() {
            let count = pattern[index % pattern.len()].min(input.len() - offset);
            let result = assembler
                .push(
                    &context,
                    chunk(
                        &input[offset..offset + count],
                        offset as u64,
                        index as u64 + 1,
                        rate,
                        13_000_000,
                    ),
                )
                .unwrap();
            if let Some(frame) = result {
                return frame;
            }
            offset += count;
            index += 1;
        }
        panic!("complete captured period was not emitted");
    }

    #[test]
    fn native_rates_are_chunk_invariant_and_preserve_unknown_clock_facts() {
        for rate in [12_000, 24_000, 44_100, 48_000, 96_000] {
            for mode in [RxMode::Ft8, RxMode::Ft4] {
                let whole = assemble(rate, mode, &[4096]);
                let split = assemble(rate, mode, &[1, 63, 64, 127, 4096]);
                assert_eq!(whole.samples, split.samples, "rate={rate} mode={mode:?}");
                assert_eq!(whole.context, context(rate, mode));
                assert_eq!(whole.generation, 1);
                assert_eq!(whole.host_period_start_unix_us, 15_000_000);
                assert_eq!(whole.samples.len(), mode.frame_samples());
                assert_eq!(whole.clock.utc_sync_accuracy_us, None);
                assert_eq!(whole.clock.remote_rf_latency_us, None);
                assert!(whole.clock.sample_alignment_offset_ns <= 83_334);
                assert_eq!(
                    whole.clock.resampler_center_offset_ns,
                    if rate == OUTPUT_RATE {
                        0
                    } else {
                        ceil_div(63_000_000_000, 2 * i128::from(rate)) as u64
                    }
                );
                assert_eq!(whole.frame_time_ms, split.frame_time_ms);
            }
        }
    }

    #[test]
    fn short_and_exact_slot_buffers_are_not_padded_or_emitted_early() {
        for mode in [RxMode::Ft8, RxMode::Ft4] {
            let context = context(12_000, mode);
            let mut assembler = RxAssembler::default();
            let total = (mode.period_us() * 12_000 / 1_000_000) as usize;
            let input = vec![1234; total];
            for (index, samples) in input.chunks(4096).enumerate() {
                assert!(assembler
                    .push(
                        &context,
                        chunk(
                            samples,
                            (index * 4096) as u64,
                            index as u64 + 1,
                            12_000,
                            30_000_000
                        )
                    )
                    .unwrap()
                    .is_none());
            }
            let sequence = input.chunks(4096).count() as u64 + 1;
            let frame = assembler
                .push(
                    &context,
                    chunk(&[1234], total as u64, sequence, 12_000, 30_000_000),
                )
                .unwrap()
                .unwrap();
            assert_eq!(frame.samples, vec![1234; mode.frame_samples()]);
        }
    }

    #[test]
    fn sequence_frame_context_and_clock_failures_discard_partial_audio() {
        let context = context(48_000, RxMode::Ft8);
        let samples = [100; 4096];
        for fault in 0..6 {
            let mut assembler = RxAssembler::default();
            assembler
                .push(&context, chunk(&samples, 0, 1, 48_000, 13_000_000))
                .unwrap();
            let before = assembler.generation();
            let mut next_context = context.clone();
            let mut next = chunk(&samples, 4096, 2, 48_000, 13_000_000);
            let expected = match fault {
                0 => {
                    next.sequence = 3;
                    RxError::SequenceDiscontinuity
                }
                1 => {
                    next.sequence = 1;
                    RxError::SequenceDiscontinuity
                }
                2 => {
                    next.first_frame += 1;
                    RxError::FrameDiscontinuity
                }
                3 => {
                    next_context.owner.epoch = "other".into();
                    RxError::ContextChanged
                }
                4 => {
                    next_context.owner.frequency_hz += 1000;
                    RxError::ContextChanged
                }
                _ => {
                    next.sample0_unix_us += 50_001;
                    RxError::ClockDiscontinuity
                }
            };
            assert_eq!(assembler.push(&next_context, next).unwrap_err(), expected);
            assert!(assembler.context.is_none());
            assert!(assembler.pending.is_none());
            assert!(assembler.resampler.is_none());
            assert!(assembler.generation() > before);
        }
    }

    #[test]
    fn every_owner_identity_change_and_decoder_selection_invalidate_the_window() {
        let base = context(12_000, RxMode::Ft8);
        let samples = [1; 20];
        for field in 0..14 {
            let mut assembler = RxAssembler::default();
            assembler
                .push(&base, chunk(&samples, 0, 1, 12_000, 30_000_000))
                .unwrap();
            let mut changed = base.clone();
            match field {
                0 => changed.owner.server_instance += "2",
                1 => changed.owner.workspace_id += "2",
                2 => changed.owner.capture_id += "2",
                3 => changed.owner.epoch += "2",
                4 => changed.owner.radio_id += "2",
                5 => changed.owner.receiver_id += "2",
                6 => changed.owner.mapping_generation += 1,
                7 => changed.owner.device_id += "2",
                8 => changed.owner.dax_channel += 1,
                9 => changed.owner.frequency_hz += 1000,
                10 => changed.owner.radio_mode = "USB".into(),
                11 => changed.owner.sample_rate = 24_000,
                12 => changed.owner.selected_channel = 1,
                _ => changed.mode = RxMode::Ft4,
            }
            assert_eq!(
                assembler
                    .push(&changed, chunk(&samples, 20, 2, 12_000, 30_000_000))
                    .unwrap_err(),
                RxError::ContextChanged
            );
        }
    }

    #[test]
    fn bounds_and_overflow_are_rejected_before_allocation() {
        let context = context(12_000, RxMode::Ft8);
        let samples = [0; 4097];
        for (count, first, sequence, uncertainty) in [
            (0, 0, 1, 0),
            (4097, 0, 1, 0),
            (1, u64::MAX, 1, 0),
            (1, 0, u64::MAX, 0),
            (1, 0, 0, 0),
            (1, 0, 1, 50_001),
        ] {
            let mut c = chunk(&samples[..count], 0, sequence, 12_000, 30_000_000);
            c.first_frame = first;
            c.calibration_uncertainty_us = uncertainty;
            assert!(RxAssembler::default().push(&context, c).is_err());
        }
        let mut c = chunk(&samples[..1], 0, 1, 12_000, 30_000_000);
        c.calibration_uncertainty_us = 50_000;
        assert!(RxAssembler::default().push(&context, c).is_ok());
        let mut bad_context = context;
        bad_context.owner.sample_rate = 0;
        assert_eq!(
            RxAssembler::default()
                .push(&bad_context, chunk(&samples[..1], 0, 1, 12_000, 30_000_000))
                .unwrap_err(),
            RxError::InvalidContext
        );
    }

    #[test]
    fn external_gap_discards_real_history_and_never_wraps_generation() {
        let mut assembler = RxAssembler::default();
        let context = context(48_000, RxMode::Ft4);
        assembler
            .push(&context, chunk(&[1; 63], 0, 1, 48_000, 30_000_000))
            .unwrap();
        assembler.reset();
        assert!(assembler.resampler.is_none());
        assert_eq!(assembler.output_count, 0);
        assembler.generation = u64::MAX;
        assembler.reset();
        assert_eq!(
            assembler
                .push(&context, chunk(&[1], 0, 1, 48_000, 30_000_000))
                .unwrap_err(),
            RxError::CounterOverflow
        );
    }
}
