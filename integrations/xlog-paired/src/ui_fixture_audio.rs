// SPDX-License-Identifier: GPL-3.0-only
//! Paced silence for disposable native UI lifecycle checks; never opens a device.
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
    pub fn start() -> Self {
        let binding = Binding {
            device_id: "synthetic-silence-no-device".into(),
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
                    producer_buffer
                        .lock()
                        .unwrap()
                        .stop("Synthetic producer missed its deadline");
                    break;
                }
                let sample_ticks = first * 10_000_000 / 48_000;
                if producer_buffer
                    .lock()
                    .unwrap()
                    .push(
                        first,
                        10_000_000 + sample_ticks,
                        origin_us * 10 + sample_ticks,
                        100,
                        &[0; 960],
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
