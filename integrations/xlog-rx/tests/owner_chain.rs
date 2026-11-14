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

// Deliberately synthetic classification facts; production Buffer, bridge input/
// reply validation and SDK transports remain real. No Swift/database owner claim.
#[cfg(xnexus_rx_needs)]
fn synthetic_authority() -> serde_json::Value {
    serde_json::json!({"logbook_revision":1,"classification_revision":1,"country_reference_revision":"xlog-country-facts-v3:synthetic-chain","policy_revision":"xlog-call-dxcc-needs-v1"})
}
#[cfg(xnexus_rx_needs)]
fn synthetic_snapshot() -> serde_json::Value {
    let now = now_us() / 1000;
    serde_json::json!({"revision":now,"generated_at_unix_ms":now,"max_age_ms":10000,
      "station":{"connected":false,"radio_id":null,"frequency_hz":null,"mode":null,"transmitting":null},
      "rotor":{"connected":false,"rotor_id":null,"azimuth_deg":null,"moving":null},"spots":[],
      "needs":{"generated_at_unix_ms":now,"max_age_ms":2000,"readiness":"ready","authority":synthetic_authority()}})
}
#[cfg(xnexus_rx_needs)]
fn synthetic_needs(batch: &serde_json::Value) -> serde_json::Value {
    let results:Vec<_> = batch["queries"].as_array().unwrap().iter().map(|q|{
        assert_eq!(q["callsign"],"K1ABC","lookup must use the actual decoded literal sender");
        assert_eq!(q["mode"],"FT4");
        serde_json::json!({"query_id":q["query_id"],"normalized_callsign":q["callsign"],"dial_frequency_hz":q["dial_frequency_hz"],"decoder_mode":q["mode"],"classification_mode":"DIGI","band":"20m","country":"United States","adif":291,"country_resolution":"resolved","call_status":"New Call","dxcc_status":"ATNO","need":{"status":"needed","score":100,"reason_code":null,"reasons":["Synthetic classification fixture"]}})
    }).collect();
    serde_json::json!({"generated_at_unix_ms":now_us()/1000,"max_age_ms":2000,"readiness":"ready","authority":synthetic_authority(),"results":results})
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

fn run_chain(mode: RxMode, _worker: bool) {
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
                #[cfg(xnexus_rx_needs)]
                return Ok(synthetic_snapshot());
                #[cfg(not(xnexus_rx_needs))]
                return Ok(serde_json::json!({}));
            };
            #[cfg(xnexus_rx_needs)]
            if let Some(batch) = request.get("needs_query") {
                return Ok(synthetic_needs(batch));
            }
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
    let end_us = target_slot
        + mode.period_us()
        + if cfg!(xnexus_rx_needs) {
            12_000_000
        } else {
            5_000_000
        };
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
    #[cfg(xnexus_rx_needs)]
    if _worker {
        run_needs_worker(&mut client, audio, &bridge, target_slot, deadline);
        return;
    }
    #[cfg(xnexus_rx_worker)]
    if _worker {
        run_worker(&mut client, audio, &bridge, mode, target_slot, deadline);
        return;
    }
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
    run_chain(RxMode::Ft8, false);
}
#[test]
fn complete_ft4_owner_http_chain() {
    run_chain(RxMode::Ft4, false);
}

#[cfg(xnexus_rx_worker)]
fn run_worker(
    client: &mut BridgeClient,
    audio: xnexus_xlog_client::audio::AudioClient,
    bridge: &Bridge,
    mode: RxMode,
    target_slot: u64,
    deadline: Instant,
) {
    use xlog_rx_worker::{Mode, Worker};
    let mode = match mode {
        RxMode::Ft8 => Mode::FT8,
        RxMode::Ft4 => Mode::FT4,
    };
    let worker = Worker::start(
        audio,
        bridge.server_instance.clone(),
        bridge.workspace_id.clone(),
        mode,
    )
    .unwrap();
    let mut found = false;
    while Instant::now() < deadline {
        let view = worker.view().unwrap();
        assert!(view.enabled, "worker stopped: {}", view.reason);
        if let Some(row) = view.rows.iter().find(|row| {
            row.host_period_start_unix_us == target_slot && row.message.contains("K1ABC")
        }) {
            assert_eq!(row.clock.utc_sync_accuracy_us, None);
            assert_eq!(row.clock.remote_rf_latency_us, None);
            assert_eq!(view.route.as_ref().unwrap().selected_channel, 1);
            found = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        found,
        "actual worker did not publish the paced decoded message: {}",
        worker.view().unwrap().reason
    );
    worker.stop();
    for _ in 0..5 {
        let view = worker.view().unwrap();
        assert!(!view.enabled && view.rows.is_empty() && view.route.is_none());
        std::thread::sleep(Duration::from_millis(20));
    }
    // The old polling/decoder threads must release their exclusive lease before
    // a fresh worker can start. A restart must not inherit any old decoded row.
    let restart_deadline = Instant::now() + Duration::from_secs(3);
    let restarted = loop {
        match Worker::start(
            client.audio_handle().unwrap(),
            bridge.server_instance.clone(),
            bridge.workspace_id.clone(),
            mode,
        ) {
            Ok(worker) => break worker,
            Err(error) => {
                assert!(
                    Instant::now() < restart_deadline,
                    "worker lease not released: {error}"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    };
    let view = restarted.view().unwrap();
    assert!(view.enabled && view.rows.is_empty());
    restarted.stop();
    assert!(!restarted.view().unwrap().enabled);
    assert!(worker.view().unwrap().rows.is_empty());
    println!("Actual Worker::start decoded CQ K1ABC FN42; Stop cleared rows; exclusive lease released and restart had no old result");
}

#[cfg(xnexus_rx_worker)]
#[test]
fn complete_ft4_worker_owner_http_lifecycle() {
    run_chain(RxMode::Ft4, true);
}

#[cfg(xnexus_rx_needs)]
fn observe_needs(client: &mut BridgeClient, worker: &xlog_rx_worker::Worker) {
    let snapshot = client.poll().unwrap();
    worker.needs_snapshot_observed(snapshot.needs.as_ref().unwrap().generated_at_unix_ms);
}

#[cfg(xnexus_rx_needs)]
fn run_needs_worker(
    client: &mut BridgeClient,
    audio: xnexus_xlog_client::audio::AudioClient,
    bridge: &Bridge,
    target_slot: u64,
    deadline: Instant,
) {
    use xlog_rx_worker::{Mode, Worker};
    use xnexus_xlog_client::outbox::{LogbookMutation, Outbox};
    use xnexus_xlog_client::{CommandReceipt, NeedStatus};
    let started = Instant::now();
    client.poll().unwrap();
    let pending = Arc::new(AtomicBool::new(false));
    let worker = Worker::start_with_needs_gate(
        audio,
        bridge.server_instance.clone(),
        bridge.workspace_id.clone(),
        Mode::FT4,
        Some(client.needs_handle().unwrap()),
        pending.clone(),
    )
    .unwrap();
    let mut next_poll = Instant::now();
    let initial = loop {
        assert!(
            Instant::now() < deadline,
            "real FT4 decode did not reach authenticated known facts: {}",
            worker.view().unwrap().reason
        );
        if Instant::now() >= next_poll {
            observe_needs(client, &worker);
            next_poll = Instant::now() + Duration::from_secs(1);
        }
        let view = worker.view().unwrap();
        assert!(view.enabled);
        if let Some(row) = view.rows.into_iter().find(|row| {
            row.host_period_start_unix_us == target_slot
                && row.message == "CQ K1ABC FN42"
                && row.needs.status == NeedStatus::Needed
        }) {
            break row;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let decode_id = initial.decode_id.clone();
    assert_eq!(initial.sender_callsign.as_deref(), Some("K1ABC"));
    assert_eq!(
        initial.needs.owner.as_ref().unwrap().normalized_callsign,
        "K1ABC"
    );
    assert_eq!(initial.needs.score, Some(100));
    assert_eq!(initial.context.server_instance, bridge.server_instance);
    assert_eq!(initial.context.workspace_id, bridge.workspace_id);
    assert_eq!(initial.context.decoder_mode, Mode::FT4);
    assert!(decode_id.starts_with('w') && decode_id.contains(&format!("_p{target_slot}_")));
    assert_eq!(initial.clock.utc_sync_accuracy_us, None);
    assert_eq!(initial.clock.remote_rf_latency_us, None);
    let known_at = started.elapsed().as_millis();

    // Real disposable client outbox, synthetic rejection receipt. This neither
    // submits a QSO nor pretends that the Swift logbook owner committed anything.
    let root = std::path::PathBuf::from(
        std::env::var_os("XNEXUS_RX_CHAIN_EVIDENCE").expect("isolated evidence root"),
    )
    .join("pending-outbox");
    let mut outbox = Outbox::open(&root).unwrap();
    let request = "00000000-0000-4000-8000-000000000123";
    let mutation:LogbookMutation=serde_json::from_value(serde_json::json!({"kind":"logbook.create","qso":{"contact_id":"00000000-0000-4000-8000-000000000124","callsign":"K1ABC","band":"20m","mode":"FT4","started_at_unix_ms":now_us()/1000,"frequency_hz":14074000,"rst_sent":"-10","rst_received":"-12","grid":"FN42","comment":"Disposable generated-decode needs fixture"}})).unwrap();
    pending.store(true, Ordering::Release);
    client.invalidate_needs();
    outbox
        .enqueue(&bridge.workspace_id, request, mutation)
        .unwrap();
    pending.store(
        xlog_rx_worker::needs_blocked_by_outbox(&outbox.entries(), &bridge.workspace_id),
        Ordering::Release,
    );
    observe_needs(client, &worker);
    let view = worker.view().unwrap();
    let blocked = view
        .rows
        .iter()
        .find(|row| row.decode_id == decode_id)
        .unwrap();
    assert_eq!(blocked.needs.status, NeedStatus::Unknown);
    assert_eq!(
        blocked.needs.reason_code.as_deref(),
        Some("native_write_pending")
    );
    assert!(blocked.needs.owner.is_none() && blocked.needs.score.is_none());
    assert!(view.enabled && view.route.is_some());

    outbox
        .mark_attempted(request, &bridge.workspace_id)
        .unwrap();
    let receipt:CommandReceipt=serde_json::from_value(serde_json::json!({"request_id":request,"server_instance":bridge.server_instance,"workspace_id":bridge.workspace_id,"status":"rejected","message":"Synthetic fixture settlement; no owner QSO submitted"})).unwrap();
    outbox.settle(receipt).unwrap();
    client.invalidate_needs();
    pending.store(
        xlog_rx_worker::needs_blocked_by_outbox(&outbox.entries(), &bridge.workspace_id),
        Ordering::Release,
    );
    assert!(!pending.load(Ordering::Acquire));
    assert!(worker
        .view()
        .unwrap()
        .rows
        .iter()
        .find(|row| row.decode_id == decode_id)
        .unwrap()
        .needs
        .owner
        .is_none());
    observe_needs(client, &worker);
    let recovery_deadline = Instant::now() + Duration::from_secs(4);
    let recovered = loop {
        assert!(
            Instant::now() < recovery_deadline,
            "settled row did not acquire new authenticated facts"
        );
        if Instant::now() >= next_poll {
            observe_needs(client, &worker);
            next_poll = Instant::now() + Duration::from_secs(1);
        }
        let view = worker.view().unwrap();
        if let Some(row) = view
            .rows
            .into_iter()
            .find(|row| row.decode_id == decode_id && row.needs.status == NeedStatus::Needed)
        {
            break row;
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(recovered.context, initial.context);
    assert!(
        recovered.needs.generated_at_unix_ms > initial.needs.generated_at_unix_ms,
        "settlement cannot renew original facts"
    );
    let recovered_at = started.elapsed().as_millis();

    // Keep the real PCM producer/poller running but stop main-snapshot refresh.
    // Query replies may still arrive; captured SDK authority must expire anyway.
    let expiry = Instant::now() + Duration::from_millis(2500);
    loop {
        let view = worker.view().unwrap();
        assert!(
            view.enabled && view.route.is_some(),
            "audio must stay fresh during needs-only expiry"
        );
        let row = view
            .rows
            .iter()
            .find(|row| row.decode_id == decode_id)
            .expect("decoded row retained while PCM continues");
        if row.needs.status == NeedStatus::Unknown {
            assert!(row.needs.owner.is_none() && row.needs.score.is_none());
            assert_eq!(row.needs.remaining_valid_ms, 0);
            break;
        }
        assert!(
            Instant::now() < expiry,
            "facts survived original snapshot authority expiry"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    let expired_at = started.elapsed().as_millis();
    worker.stop();
    assert!(!worker.view().unwrap().enabled && worker.view().unwrap().rows.is_empty());
    let restart_deadline = Instant::now() + Duration::from_secs(3);
    let restarted = loop {
        match Worker::start(
            client.audio_handle().unwrap(),
            bridge.server_instance.clone(),
            bridge.workspace_id.clone(),
            Mode::FT4,
        ) {
            Ok(worker) => break worker,
            Err(error) => {
                assert!(
                    Instant::now() < restart_deadline,
                    "worker lease leaked: {error}"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    };
    assert!(restarted.view().unwrap().rows.is_empty());
    restarted.stop();
    // Wait for the restarted no-needs worker to retire before ending the fixture.
    std::thread::sleep(Duration::from_millis(150));
    assert!(worker.view().unwrap().rows.is_empty());
    println!("FT4 decode→owner HTTP needs: id={decode_id}, known_ms={known_at}, recovered_ms={recovered_at}, expired_ms={expired_at}; durable pending fence/TTL/Stop passed; synthetic classification provider, no Swift/database/hardware");
}

#[cfg(xnexus_rx_needs)]
#[test]
fn complete_ft4_decoded_needs_owner_http_lifecycle() {
    run_chain(RxMode::Ft4, true);
}
