// SPDX-License-Identifier: GPL-3.0-only
//! Paced synthetic capture through the actual owner Buffer/HTTP/client and DSP.
//! Compiled by test-rx-owner-chain.ps1 with pinned recorded dependency artifacts.
include!(env!("XNEXUS_RX_OWNER_MODULES"));
#[path = "../../../src-tauri/src/xlog_decode.rs"]
mod xlog_decode;

use companion_bridge::Bridge;
use companion_rx_buffer::{Binding, Buffer, Route};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tempo_core::modes::{self, ModeKind};
use xnexus_xlog_client::{
    audio::{AudioDescriptor, AudioState},
    BridgeClient,
};
use xnexus_xlog_rx::{OwnerIdentity, PcmChunk, ReceiveContext, RxAssembler, RxMode};

fn now_us() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

struct Cleanup {
    bridge: Arc<Bridge>,
    stop: Arc<AtomicBool>,
    producer: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(producer) = self.producer.take() {
            producer.join().expect("synthetic producer");
        }
        self.bridge.revoke();
    }
}

fn context(descriptor: &AudioDescriptor, bridge: &Bridge, mode: RxMode) -> ReceiveContext {
    ReceiveContext {
        mode,
        owner: OwnerIdentity {
            server_instance: bridge.server_instance.clone(),
            workspace_id: bridge.workspace_id.clone(),
            capture_id: descriptor.capture_id.clone(),
            epoch: descriptor.epoch.clone(),
            radio_id: descriptor.radio_id.clone(),
            receiver_id: descriptor.receiver_id.clone(),
            mapping_generation: descriptor.mapping_generation,
            device_id: descriptor.device_id.clone(),
            slice: descriptor.slice.clone(),
            dax_channel: descriptor.dax_channel,
            frequency_hz: descriptor.frequency_hz,
            radio_mode: descriptor.mode.clone(),
            sample_rate: descriptor.sample_rate,
            source_channels: descriptor.source_channels as u8,
            selected_channel: descriptor.selected_channel as u8,
        },
    }
}

fn run_chain(mode: RxMode) {
    const RATE: u32 = 48_000;
    let kind = match mode {
        RxMode::Ft8 => ModeKind::Ft8,
        RxMode::Ft4 => ModeKind::Ft4,
    };
    let modem = modes::make_mode(kind);
    let tones = modem.encode("CQ K1ABC FN42");
    assert!(!tones.is_empty());
    let wave = modem.gen_wave(&tones, 12_000.0, 1500.0);
    let binding = Binding {
        device_id: "synthetic-endpoint-no-device".into(),
        radio_id: "radio1".into(),
        receiver_id: "synthetic-client-slice-a".into(),
        dax_channel: 2,
        sample_rate: RATE,
        source_channels: 2,
        selected_channel: 1,
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
    buffer.start(&binding, &route, 1).unwrap();
    let buffer = Arc::new(Mutex::new(buffer));
    let provider = buffer.clone();
    let bridge = Bridge::start(
        "synthetic-rx-whole-chain".into(),
        Arc::new(move |request| {
            let Some(request) = request else {
                return Ok(serde_json::json!({}));
            };
            let after = request["audio_after"]
                .as_u64()
                .ok_or("Only audio reads are allowed")?;
            Ok(provider.lock().unwrap().response(after, now_us() / 1000))
        }),
    )
    .unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let mut cleanup = Cleanup {
        bridge: bridge.clone(),
        stop: stop.clone(),
        producer: None,
    };
    let mut client = BridgeClient::new(&bridge.endpoint, &bridge.token).unwrap();
    client.connect().unwrap();
    let mut audio = client.audio_handle().unwrap();

    // Genuine wall-clock pacing is required: the SDK still enforces its <=3s
    // sample age and <=50ms future tolerance, and the owner retains only2s.
    let start = Instant::now() + Duration::from_millis(200);
    let origin_us = now_us() + 200_000;
    let target_slot = (origin_us + 2_000_000).div_ceil(mode.period_us()) * mode.period_us();
    let end_us = target_slot + mode.period_us() + 5_000_000;
    let total = ((end_us - origin_us) * u64::from(RATE) / 1_000_000) as usize;
    let wave_offset = ((target_slot - origin_us) * u64::from(RATE) / 1_000_000) as usize;
    let mut capture = vec![0_i16; total];
    for (index, dest) in capture.iter_mut().skip(wave_offset).enumerate() {
        // Independent fixture interpolation preserves the real12k modem baud.
        let numerator = index as u64 * 12_000;
        let whole = (numerator / u64::from(RATE)) as usize;
        if whole + 1 >= wave.len() {
            break;
        }
        let fraction = (numerator % u64::from(RATE)) as f32 / RATE as f32;
        let sample = wave[whole] * (1.0 - fraction) + wave[whole + 1] * fraction;
        *dest = (sample * 1000.0).round().clamp(-32768.0, 32767.0) as i16;
    }
    let producer_error = Arc::new(Mutex::new(None));
    let error = producer_error.clone();
    let producer_stop = stop.clone();
    cleanup.producer = Some(std::thread::spawn(move || {
        for (index, samples) in capture.chunks(960).enumerate() {
            let first = index as u64 * 960;
            // Publish only after the whole synthetic callback has been captured.
            let deadline = start
                + Duration::from_micros(
                    (first + samples.len() as u64) * 1_000_000 / u64::from(RATE),
                );
            while Instant::now() < deadline && !producer_stop.load(Ordering::Acquire) {
                std::thread::sleep(
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(10)),
                );
            }
            if producer_stop.load(Ordering::Acquire) {
                break;
            }
            let elapsed_100ns = first * 10_000_000 / u64::from(RATE);
            let result = buffer.lock().unwrap().push(
                first,
                1_000_000_000 + elapsed_100ns,
                origin_us * 10 + elapsed_100ns,
                100,
                samples,
            );
            if let Err(problem) = result {
                *error.lock().unwrap() = Some(problem);
                break;
            }
        }
    }));
    let deadline = start + Duration::from_micros(end_us - origin_us) + Duration::from_secs(2);
    let mut assembler = RxAssembler::default();
    let mut observed_context = None;
    let mut target = None;
    let mut chunk_count = 0;
    while Instant::now() < deadline {
        assert!(
            producer_error.lock().unwrap().is_none(),
            "owner rejected synthetic capture"
        );
        let batch = audio.poll().expect("actual owner HTTP audio poll");
        assert!(batch.is_fresh());
        if batch.reset {
            assembler.reset();
        }
        if batch.state != AudioState::Running {
            std::thread::sleep(Duration::from_millis(20));
            continue;
        }
        let descriptor = batch.descriptor.as_ref().unwrap();
        assert_eq!(descriptor.selected_channel, 1);
        let current = context(descriptor, &bridge, mode);
        if let Some(previous) = &observed_context {
            assert_eq!(previous, &current);
        }
        observed_context = Some(current.clone());
        for chunk in &batch.chunks {
            chunk_count += 1;
            if let Some(period) = assembler
                .push(
                    &current,
                    PcmChunk {
                        sequence: chunk.sequence,
                        first_frame: chunk.first_frame,
                        sample0_unix_us: chunk.sample0_unix_us,
                        calibration_uncertainty_us: chunk.calibration_uncertainty_us,
                        samples: &chunk.samples,
                    },
                )
                .expect("owner chunk continuity")
            {
                assert!(period.host_period_start_unix_us <= target_slot);
                if period.host_period_start_unix_us == target_slot {
                    target = Some(period);
                    break;
                }
            }
        }
        if target.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let target = target.expect("complete target period before bounded deadline");
    let result = xlog_decode::Decoder::default()
        .decode(xlog_decode::CompleteFrame::from_period(target).unwrap())
        .unwrap();
    assert!(
        result.rows.iter().any(|row| row.message.contains("K1ABC")),
        "{mode:?}: {:?}",
        result.rows
    );
    assert_eq!(result.clock.utc_sync_accuracy_us, None);
    assert_eq!(result.clock.remote_rf_latency_us, None);
    let final_batch = audio.poll().unwrap();
    assert!(final_batch.is_fresh() && final_batch.state == AudioState::Running);
    assert_eq!(
        context(final_batch.descriptor.as_ref().unwrap(), &bridge, mode),
        observed_context.unwrap()
    );
    assert!(chunk_count > 100, "must traverse genuine paced HTTP chunks");
    println!("{mode:?}: {chunk_count} real owner HTTP chunks; decoded CQ K1ABC FN42 at host period {target_slot}");
}

#[test]
fn complete_ft8_owner_http_chain() {
    run_chain(RxMode::Ft8);
}
#[test]
fn complete_ft4_owner_http_chain() {
    run_chain(RxMode::Ft4);
}
