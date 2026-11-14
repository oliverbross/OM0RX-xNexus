// SPDX-License-Identifier: GPL-3.0-only
//! Paced silence or an explicit generated FT4 period; never opens a device.
use crate::companion_rx_buffer::{Binding, Buffer, Route};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub struct Audio {
    pub buffer: Arc<Mutex<Buffer>>,
    stop: Arc<AtomicBool>,
    producer: Option<std::thread::JoinHandle<()>>,
}
impl Audio {
    pub fn start() -> Self { Self::start_period(None) }
    /// The only waveform accepted is one 7.5-second, mono48k generated period.
    pub fn start_period(period: Option<Vec<i16>>) -> Self {
        if let Some(samples) = &period { assert_eq!(samples.len(), 360_000); }
        let binding = Binding {
            device_id: if period.is_some() { "synthetic-generated-ft4-no-device" } else { "synthetic-silence-no-device" }.into(),
            radio_id: "radio1".into(),
            receiver_id: "synthetic-slice-a".into(),
            dax_channel: 2,
            sample_rate: 48_000,
            source_channels: 1,
            selected_channel: 0,
            operator_confirmed: true,
        };
        let route = Route {
            radio_id: binding.radio_id.clone(),
            receiver_id: binding.receiver_id.clone(),
            connection_generation: 1,
            selection_generation: 1,
            dax_channel: 2,
            frequency_hz: 14_074_000,
            mode: "DIGU".into(),
        };
        let mut buffer = Buffer::default();
        buffer.start(&binding, &route, 1).expect("synthetic route");
        let buffer = Arc::new(Mutex::new(buffer));
        let stop = Arc::new(AtomicBool::new(false));
        let producer_stop = stop.clone();
        let producer_buffer = buffer.clone();
        let producer = std::thread::spawn(move || {
            let start = Instant::now() + Duration::from_millis(200);
            let origin_us = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64
                + 200_000;
            let mut first = 0_u64;
            while !producer_stop.load(Ordering::Acquire) {
                let deadline = start + Duration::from_micros((first + 960) * 1_000_000 / 48_000);
                if let Some(wait) = deadline.checked_duration_since(Instant::now()) {
                    std::thread::sleep(wait);
                }
                if producer_stop.load(Ordering::Acquire) {
                    break;
                }
                // Never catch up a badly delayed producer by publishing a stale burst.
                if Instant::now().saturating_duration_since(deadline) > Duration::from_millis(250) {
                    eprintln!("synthetic callback deadline missed: first_frame={first}, late_us={}, elapsed_us={}",
                        Instant::now().saturating_duration_since(deadline).as_micros(),
                        Instant::now().saturating_duration_since(start).as_micros());
                    producer_buffer
                        .lock()
                        .unwrap()
                        .stop("Synthetic producer missed its deadline");
                    break;
                }
                let sample_ticks = first * 10_000_000 / 48_000;
                let samples = callback_samples(period.as_deref(), origin_us, first);
                if producer_buffer
                    .lock()
                    .unwrap()
                    .push(
                        first,
                        10_000_000 + sample_ticks,
                        origin_us * 10 + sample_ticks,
                        100,
                        &samples,
                    )
                    .is_err()
                {
                    break;
                }
                first += 960;
            }
        });
        Self {
            buffer,
            stop,
            producer: Some(producer),
        }
    }
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.buffer
            .lock()
            .unwrap()
            .stop("Synthetic sharing stopped");
        if let Some(producer) = self.producer.take() {
            producer.join().expect("synthetic producer");
        }
    }
}
impl Drop for Audio {
    fn drop(&mut self) {
        self.stop();
    }
}

// UTC-aligned periods allow the operator to pair/start RX after fixture startup.
fn callback_samples(period: Option<&[i16]>, origin_us: u64, first: u64) -> Vec<i16> {
    (0..960).map(|offset| period.map_or(0, |samples| {
        let origin_frame = (origin_us % 7_500_000) * 48_000 / 1_000_000;
        samples[((origin_frame + first + offset) % 360_000) as usize]
    })).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn period_wrap_and_origin_are_sample_aligned() {
        let mut period = vec![0; 360_000]; period[0] = 123; period[1] = -456;
        let chunk = callback_samples(Some(&period), 7_499_000, 0);
        assert_eq!(&chunk[48..50], &[123, -456]);
        assert_eq!(callback_samples(Some(&period), 0, 360_000)[0], 123);
        assert!(callback_samples(None, 0, 0).iter().all(|s| *s == 0));
    }
}
