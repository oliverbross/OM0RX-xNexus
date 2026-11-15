// SPDX-License-Identifier: GPL-3.0-only
//! Real audio waterfall derived from the SAME validated/resampled owner PCM as
//! decoding. One rolling window, one replaceable pending window, one FFT thread.
use super::{Lease, NeedsContext, State};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};
use xnexus_xlog_client::audio::AudioBatch;
use xnexus_xlog_rx::{ValidatedSamples, OUTPUT_RATE};

const WINDOW: usize = 2048;
const BINS: usize = 640;
const HOP: u64 = 1200; // At most ten spectrum windows per second of real samples.
const MAX_SAFE: u64 = 9_007_199_254_740_991;
const MAX_PICTURE_AGE: Duration = Duration::from_millis(500);

#[derive(Clone, Deserialize)]
pub struct Cursor {
    pub worker_instance: String,
    pub worker_epoch: u64,
    pub sequence: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spectrum {
    pub row: Vec<f32>,
    pub lo_hz: f64,
    pub hi_hz: f64,
    pub source: &'static str,
}

#[derive(Clone, Serialize)]
pub struct Frame {
    pub context: NeedsContext,
    pub sequence: u64,
    pub remaining_valid_ms: u64,
    pub sample_rate_hz: u32,
    pub fft_samples: usize,
    pub first_output_index: u64,
    pub source_origin_frame: u64,
    pub source_origin_unix_us: u64,
    pub source_sample_rate_hz: u32,
    pub fir_center_source_frames_twice: u32,
    /// Nominal host sample times, floored to microseconds; end is exclusive.
    /// Nominal rational positions follow origin/rate/index/center. The existing
    /// resampler quantizes FIR phases to 1/256; no exact analog center is claimed.
    pub window_start_unix_us: u64,
    pub window_end_unix_us: u64,
    pub timestamp_basis: &'static str,
    pub host_mapping_uncertainty_us: u32,
    pub calibration_uncertainty_us: u32,
    pub utc_sync_accuracy_us: Option<u32>,
    pub remote_rf_latency_us: Option<i64>,
    pub scale_lo_dbfs: i32,
    pub scale_hi_dbfs: i32,
    pub spectrum: Spectrum,
}

pub(super) struct Published {
    frame: Frame,
    authorities: Vec<Arc<AudioBatch>>,
    accepted: Instant,
}
impl Published {
    pub(super) fn matches(&self, state: &State) -> bool {
        matches_context(&self.frame.context, state)
    }
    pub(super) fn read(&self, after: Option<&Cursor>) -> Option<Frame> {
        let remaining = self
            .authorities
            .iter()
            .map(|batch| batch.remaining_valid_ms())
            .min()
            .unwrap_or(0)
            .min(
                MAX_PICTURE_AGE
                    .saturating_sub(self.accepted.elapsed())
                    .as_millis() as u64,
            );
        if remaining == 0
            || after.is_some_and(|after| {
                self.frame.context.worker_instance == after.worker_instance
                    && self.frame.context.worker_epoch == after.worker_epoch
                    && self.frame.sequence <= after.sequence
            })
        {
            return None;
        }
        let mut frame = self.frame.clone();
        frame.remaining_valid_ms = remaining;
        Some(frame)
    }
}

fn matches_context(context: &NeedsContext, state: &State) -> bool {
    context.worker_instance == state.spectrum_identity.0
        && context.server_instance == state.spectrum_identity.1
        && context.workspace_id == state.spectrum_identity.2
        && context.worker_epoch == state.epoch
        && state.view.route.as_ref() == Some(&context.route)
        && state.view.mode == Some(context.decoder_mode)
}

fn publish(state: &Mutex<State>, cancel: &AtomicBool, job: Window) -> bool {
    if cancel.load(Ordering::Acquire)
        || job.authorities.iter().any(|batch| !batch.is_fresh())
        || job.accepted.elapsed() >= MAX_PICTURE_AGE
    {
        return false;
    }
    let Ok(mut state) = state.lock() else {
        return false;
    };
    if !state.accepts(job.frame.context.worker_epoch)
        || !matches_context(&job.frame.context, &state)
    {
        return false;
    }
    state.spectrum = Some(Published {
        frame: job.frame,
        authorities: job.authorities,
        accepted: job.accepted,
    });
    true
}

struct Window {
    samples: Vec<f32>,
    frame: Frame,
    authorities: Vec<Arc<AudioBatch>>,
    accepted: Instant,
}
#[derive(Default)]
pub(super) struct Mailbox {
    pending: Mutex<Option<Window>>,
}
impl Mailbox {
    pub(super) fn clear(&self) {
        // No FFT/HTTP is ever performed while this very short lock is held.
        if let Ok(mut pending) = self.pending.lock() {
            *pending = None;
        }
    }
}

pub(super) struct Producer {
    mailbox: Arc<Mailbox>,
    wake: mpsc::SyncSender<()>,
    samples: VecDeque<f32>,
    authorities: VecDeque<(u64, Arc<AudioBatch>)>,
    identity: Option<(u64, u64, u64, u32)>,
    next_output: u64,
    next_window: u64,
    sequence: u64,
}
impl Producer {
    pub(super) fn new(mailbox: Arc<Mailbox>, wake: mpsc::SyncSender<()>) -> Self {
        Self {
            mailbox,
            wake,
            samples: VecDeque::with_capacity(WINDOW),
            authorities: VecDeque::new(),
            identity: None,
            next_output: 0,
            next_window: 0,
            sequence: 0,
        }
    }
    pub(super) fn reset(&mut self) {
        self.samples.clear();
        self.authorities.clear();
        self.identity = None;
        self.next_output = 0;
        self.next_window = 0;
        self.mailbox.clear();
        // Sequence is monotonic across epochs within this worker instance.
    }
    /// Track original leases including samples still in the FIR's 64-tap tail.
    /// Called before assembly; assembly failure must reset this producer.
    pub(super) fn input(&mut self, authority: Arc<AudioBatch>, end_frame: u64) {
        if self
            .authorities
            .back()
            .is_some_and(|(_, previous)| Arc::ptr_eq(previous, &authority))
        {
            self.authorities.back_mut().expect("checked last lease").0 = end_frame;
        } else {
            self.authorities.push_back((end_frame, authority));
        }
        // Maximum input span at supported192k ->12k, plus complete FIR history.
        let earliest = end_frame.saturating_sub(WINDOW as u64 * 16 + 64);
        while self
            .authorities
            .front()
            .is_some_and(|(end, _)| *end <= earliest)
        {
            self.authorities.pop_front();
        }
    }
    pub(super) fn push(&mut self, context: NeedsContext, span: ValidatedSamples<'_>) {
        if span.source_rate != context.route.sample_rate
            || span.source_rate == 0
            || span.samples.is_empty()
        {
            self.reset();
            return;
        }
        let identity = (
            context.worker_epoch,
            span.origin_device_frame,
            span.origin_unix_us,
            span.source_rate,
        );
        if context.worker_epoch > MAX_SAFE {
            self.reset();
            return;
        }
        if self.identity.is_some_and(|old| old != identity)
            || (self.identity.is_some() && span.first_output_index != self.next_output)
        {
            self.reset();
            return;
        }
        if self.identity.is_none() {
            // Caller resets input history at every assembler epoch. The first
            // observation must retain the pre-output FIR warmup input leases.
            self.samples.clear();
            self.next_output = 0;
            self.next_window = 0;
            self.mailbox.clear();
            self.identity = Some(identity);
        }
        let Some(end) = span
            .first_output_index
            .checked_add(span.samples.len() as u64)
            .filter(|n| *n <= MAX_SAFE)
        else {
            self.reset();
            return;
        };
        self.next_output = end;
        if span.samples.iter().any(|sample| !sample.is_finite()) {
            self.reset();
            return;
        }
        for &sample in span.samples {
            if self.samples.len() == WINDOW {
                self.samples.pop_front();
            }
            self.samples.push_back(sample);
        }
        let first = end.saturating_sub(WINDOW as u64);
        let source_first = u128::from(span.origin_device_frame)
            + u128::from(first) * u128::from(span.source_rate) / u128::from(OUTPUT_RATE);
        while self
            .authorities
            .front()
            .is_some_and(|(end, _)| u128::from(*end) <= source_first)
        {
            self.authorities.pop_front();
        }
        if self.authorities.is_empty()
            || self.authorities.iter().any(|(_, batch)| !batch.is_fresh())
        {
            self.reset();
            return;
        }
        if self.samples.len() != WINDOW || end < self.next_window {
            return;
        }
        let Some(sequence) = self.sequence.checked_add(1).filter(|n| *n <= MAX_SAFE) else {
            self.reset();
            return;
        };
        self.sequence = sequence;
        let Some(next) = end.checked_add(HOP) else {
            self.reset();
            return;
        };
        self.next_window = next;
        let first = end - WINDOW as u64;
        let center = if span.source_rate == OUTPUT_RATE {
            0
        } else {
            63
        };
        let time = |index: u64| -> Option<u64> {
            let denominator = 2 * u128::from(span.source_rate) * u128::from(OUTPUT_RATE);
            if denominator == 0 {
                return None;
            }
            let delta = (u128::from(center) * u128::from(OUTPUT_RATE)
                + 2 * u128::from(index) * u128::from(span.source_rate))
                * 1_000_000
                / denominator;
            u64::try_from(u128::from(span.origin_unix_us) + delta)
                .ok()
                .filter(|n| *n <= MAX_SAFE)
        };
        let (Some(start_us), Some(end_us)) = (time(first), time(end)) else {
            self.reset();
            return;
        };
        let frame = Frame {
            context,
            sequence,
            remaining_valid_ms: 0,
            sample_rate_hz: OUTPUT_RATE,
            fft_samples: WINDOW,
            first_output_index: first,
            source_origin_frame: span.origin_device_frame,
            source_origin_unix_us: span.origin_unix_us,
            source_sample_rate_hz: span.source_rate,
            fir_center_source_frames_twice: center,
            window_start_unix_us: start_us,
            window_end_unix_us: end_us,
            timestamp_basis: "windows_endpoint_capture",
            host_mapping_uncertainty_us: span.host_mapping_uncertainty_us,
            calibration_uncertainty_us: span.calibration_uncertainty_us,
            utc_sync_accuracy_us: None,
            remote_rf_latency_us: None,
            scale_lo_dbfs: -120,
            scale_hi_dbfs: 0,
            spectrum: Spectrum {
                row: vec![],
                lo_hz: 0.0,
                hi_hz: 5000.0,
                source: "audio",
            },
        };
        // A busy consumer never blocks PCM/decoder/Stop. Replacement drops only
        // a visual window, never assembler samples or an RF receive period.
        if let Ok(mut pending) = self.mailbox.pending.try_lock() {
            *pending = Some(Window {
                samples: self.samples.iter().copied().collect(),
                frame,
                authorities: self
                    .authorities
                    .iter()
                    .map(|(_, batch)| batch.clone())
                    .collect(),
                accepted: Instant::now(),
            });
            let _ = self.wake.try_send(());
        }
    }
}

#[cfg(test)]
#[path = "xlog_spectrum/tests.rs"]
mod tests;

pub(super) fn spawn(
    state: Arc<Mutex<State>>,
    cancel: Arc<AtomicBool>,
    lease: Arc<Lease>,
    mailbox: Arc<Mailbox>,
    wake: mpsc::Receiver<()>,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    spawn_with(state, cancel, lease, mailbox, wake, |samples| {
        tempo_core::spectrum::power_spectrum(samples, OUTPUT_RATE as f32, 0.0, 5000.0, BINS)
    })
}

fn spawn_with(
    state: Arc<Mutex<State>>,
    cancel: Arc<AtomicBool>,
    lease: Arc<Lease>,
    mailbox: Arc<Mailbox>,
    wake: mpsc::Receiver<()>,
    mut compute: impl FnMut(&[f32]) -> Vec<f32> + Send + 'static,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("xlog-rx-spectrum".into())
        .spawn(move || {
            // Retains the existing ACTIVE worker lease through all FFT cleanup; a
            // replacement RX worker cannot overlap this thread after Stop returns.
            let _lease = lease;
            while !cancel.load(Ordering::Acquire) {
                match wake.recv_timeout(Duration::from_millis(100)) {
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Ok(()) => {}
                }
                let job = mailbox
                    .pending
                    .lock()
                    .ok()
                    .and_then(|mut pending| pending.take());
                let Some(mut job) = job else {
                    continue;
                };
                if job.authorities.iter().any(|batch| !batch.is_fresh())
                    || job.accepted.elapsed() >= MAX_PICTURE_AGE
                {
                    continue;
                }
                if !state
                    .lock()
                    .is_ok_and(|mut state| state.accepts(job.frame.context.worker_epoch))
                {
                    continue;
                }
                job.frame.spectrum.row = compute(&job.samples);
                if job.frame.spectrum.row.len() != BINS
                    || job
                        .frame
                        .spectrum
                        .row
                        .iter()
                        .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                {
                    continue;
                }
                publish(&state, &cancel, job);
            }
            mailbox.clear();
        })
}
