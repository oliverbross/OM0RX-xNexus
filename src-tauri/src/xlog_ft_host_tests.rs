use super::*;
use crate::xlog_decode::{DecodeContext, DecoderMode};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
};
use xnexus_xlog_rx::{ClockProvenance, OwnerIdentity, TimestampBasis};
const S: &str = "11111111-1111-4111-8111-111111111111";
const W: &str = "22222222-2222-4222-8222-222222222222";
const C: &str = "33333333-3333-4333-8333-333333333333";
const K: &str = "44444444-4444-4444-8444-444444444444";
struct FakeState {
    ready: AtomicBool,
    fail_context: AtomicBool,
    failed_context_requests: AtomicU64,
    audio_generation: AtomicU64,
    enabled: AtomicBool,
    receipt: Mutex<Option<Value>>,
    terminal: AtomicBool,
    uncertain: AtomicBool,
    heartbeats: AtomicU64,
    slot: AtomicU64,
    prepare_blocked: AtomicBool,
    prepare_entered: AtomicBool,
    prepare_requests: AtomicU64,
    drop_prepare_response: AtomicBool,
    max_prepare_bytes: AtomicU64,
    changed_tx_route: AtomicBool,
    tune_count: AtomicU64,
    tune_gain: AtomicU64,
    stop_blocked: AtomicBool,
    stop_entered: AtomicBool,
}
struct Fake {
    url: String,
    state: Arc<FakeState>,
    cancel: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}
impl Fake {
    fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let state = Arc::new(FakeState {
            ready: AtomicBool::new(true),
            fail_context: AtomicBool::new(false),
            failed_context_requests: AtomicU64::new(0),
            audio_generation: AtomicU64::new(1),
            enabled: AtomicBool::new(false),
            receipt: Mutex::new(None),
            terminal: AtomicBool::new(false),
            uncertain: AtomicBool::new(false),
            heartbeats: AtomicU64::new(0),
            slot: AtomicU64::new(0),
            prepare_blocked: AtomicBool::new(false),
            prepare_entered: AtomicBool::new(false),
            prepare_requests: AtomicU64::new(0),
            drop_prepare_response: AtomicBool::new(false),
            max_prepare_bytes: AtomicU64::new(4194304),
            changed_tx_route: AtomicBool::new(false),
            tune_count: AtomicU64::new(0),
            tune_gain: AtomicU64::new(0),
            stop_blocked: AtomicBool::new(false),
            stop_entered: AtomicBool::new(false),
        });
        let cancel = Arc::new(AtomicBool::new(false));
        let a = state.clone();
        let stop = cancel.clone();
        let thread = thread::spawn(move || {
            let mut jobs = vec![];
            while !stop.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        assert!(jobs.len() < 200);
                        let a = a.clone();
                        jobs.push(thread::spawn(move || serve(stream, &a)));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2))
                    }
                    Err(e) => panic!("{e}"),
                }
            }
            for job in jobs {
                job.join().unwrap();
            }
        });
        Self {
            url,
            state,
            cancel,
            thread: Some(thread),
        }
    }
}
impl Drop for Fake {
    fn drop(&mut self) {
        self.state.prepare_blocked.store(false, Ordering::Release);
        self.state.stop_blocked.store(false, Ordering::Release);
        self.cancel.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn context(state: &FakeState) -> Value {
    json!({"schema_version":1,"binding":{"server_instance":S,"workspace_id":W,"context_id":C,"clock_epoch":K},"ready":state.ready.load(Ordering::Acquire),"reason":if state.ready.load(Ordering::Acquire){None}else{Some("not_ready")},"local_enabled":true,"client_enabled":state.enabled.load(Ordering::Acquire),"identity":{"mycall":"K1ABC","mygrid":"FN42","worker_generation":1,"settings_revision":1,"identity_epoch":C,"identity_revision":1},"route":{"radio_id":"radio","receiver_id":"A","transmit_slice":"A","device_id":if state.changed_tx_route.load(Ordering::Acquire){"other-output"}else{"synthetic-output"},"route_epoch":K,"sample_rate_hz":48000,"channels":2,"selected_channels":[0],"frequency_hz":14074000,"radio_mode":"DIGU"},"generated_at_unix_ms":wall_us().unwrap()/1000,"max_age_ms":1000,"heartbeat_interval_ms":200,"heartbeat_timeout_ms":1000,"max_prepare_bytes":state.max_prepare_bytes.load(Ordering::Acquire),"min_offset_hz":100,"max_offset_hz":5000,"max_abs_sample_f32_bits":0.95f32.to_bits()})
}
fn serve(mut stream: TcpStream, state: &FakeState) {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut bytes = vec![];
    let mut block = [0u8; 8192];
    let header = loop {
        let n = stream.read(&mut block).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&block[..n]);
        if let Some(p) = bytes.windows(4).position(|x| x == b"\r\n\r\n") {
            break p + 4;
        }
        assert!(bytes.len() < 32768)
    };
    let text = String::from_utf8_lossy(&bytes[..header]);
    let path = text
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_owned();
    assert!(text.to_ascii_lowercase().contains("authorization: bearer "));
    let size = text
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|s| s.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    assert!(size <= 4194304);
    while bytes.len() < header + size {
        let n = stream.read(&mut block).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&block[..n]);
    }
    let body = &bytes[header..header + size];
    let response = if path == "/xlog/v1/hello" {
        json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":S,"workspace_id":W,"capabilities":["snapshot.read","digital.tx","audio.rx"]})
    } else if path.starts_with("/xlog/v1/audio?") {
        let after = path.split('=').last().unwrap().parse::<u64>().unwrap();
        let generation = state.audio_generation.load(Ordering::Acquire);
        let capture = if generation == 1 {
            C.to_owned()
        } else {
            W.to_owned()
        };
        let epoch = if generation == 1 {
            K.to_owned()
        } else {
            S.to_owned()
        };
        json!({"protocol_version":1,"server_instance":S,"workspace_id":W,"generated_at_unix_ms":wall_us().unwrap()/1000,"max_age_ms":1000,"state":"running","reason":null,"descriptor":{"capture_id":capture,"epoch":epoch,"radio_id":"radio","receiver_id":"A","mapping_generation":generation,"device_id":"synthetic-input","slice":"A","dax_channel":1,"frequency_hz":14074000,"mode":"DIGU","source_channels":1,"selected_channel":0,"sample_rate":12000,"sample_format":"s16le","timestamp_basis":"windows_endpoint_capture","remote_rf_latency_us":null,"utc_sync_accuracy_us":null},"oldest_sequence":after+1,"next_sequence":after+2,"gap":false,"chunks":[{"sequence":after+1,"first_frame":after,"frame_count":1,"sample0_unix_us":wall_us().unwrap(),"calibration_uncertainty_us":10,"pcm_base64":"AAA="}]})
    } else if path.ends_with("/context") {
        if state.fail_context.load(Ordering::Acquire) {
            state.failed_context_requests.fetch_add(1, Ordering::AcqRel);
            stream
                .write_all(
                    b"HTTP/1.1 503 Unavailable\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
                )
                .unwrap();
            return;
        }

        context(state)
    } else if path.ends_with("/heartbeat") {
        state.heartbeats.fetch_add(1, Ordering::AcqRel);
        context(state)
    } else if path.ends_with("/enable") {
        let b: Value = serde_json::from_slice(body).unwrap();
        state
            .enabled
            .store(b["enabled"].as_bool().unwrap(), Ordering::Release);
        context(state)
    } else if path.ends_with("/prepare") || path.ends_with("/tune") {
        if path.ends_with("/prepare") {
            state.prepare_requests.fetch_add(1, Ordering::AcqRel);
            if state.drop_prepare_response.load(Ordering::Acquire) {
                return;
            }
        }
        let (d, digest) = if path.ends_with("/prepare") {
            let n = u32::from_le_bytes(body[..4].try_into().unwrap()) as usize;
            (
                serde_json::from_slice::<Value>(&body[4..4 + n]).unwrap(),
                sha2::Sha256::digest(body),
            )
        } else {
            (
                serde_json::from_slice(body).unwrap(),
                sha2::Sha256::digest(body),
            )
        };
        if !path.ends_with("/tune") {
            state
                .slot
                .store(d["slot_start_unix_us"].as_u64().unwrap(), Ordering::Release);
        }
        let digest: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        let tune = path.ends_with("/tune");
        if tune {
            state.tune_count.fetch_add(1, Ordering::AcqRel);
            let gain = d["gain_f32_bits"].as_u64().unwrap();
            assert!(f32::from_bits(gain as u32) > 0.0 && f32::from_bits(gain as u32) <= 0.8);
            state.tune_gain.store(gain, Ordering::Release);
        }
        let r = json!({"schema_version":1,"server_instance":S,"workspace_id":W,"operation_id":d["operation_id"],"body_sha256":digest,"context_id":C,"clock_epoch":K,"event_id":d["event_id"],"phase":if tune{"active"}else{"prepared"},"outcome":if tune{"uncertain"}else{"no_start"},"actual_start_unix_us":if tune{Some(wall_us().unwrap())}else{None},"release_observed_unix_us":null,"reason":null});
        *state.receipt.lock().unwrap() = Some(r.clone());
        state.prepare_entered.store(true, Ordering::Release);
        let start = Instant::now();
        while state.prepare_blocked.load(Ordering::Acquire)
            && start.elapsed() < Duration::from_secs(2)
        {
            thread::sleep(Duration::from_millis(5));
        }
        state.receipt.lock().unwrap().clone().unwrap()
    } else if path.ends_with("/commit") {
        let mut r = state.receipt.lock().unwrap();
        let r = r.as_mut().unwrap();
        let input: Value = serde_json::from_slice(body).unwrap();
        assert_eq!(input["operation_id"], r["operation_id"]);
        // Deliberately synthetic actual-start proof; no playback or physical event.
        r["actual_start_unix_us"] = json!(state.slot.load(Ordering::Acquire));
        if state.terminal.load(Ordering::Acquire) {
            r["release_observed_unix_us"] = json!(state.slot.load(Ordering::Acquire) + 1);
        }
        r["phase"] = json!(if state.terminal.load(Ordering::Acquire) {
            "terminal"
        } else {
            "active"
        });
        r["outcome"] = json!(if state.terminal.load(Ordering::Acquire) {
            "release_observed"
        } else {
            "uncertain"
        });
        r.clone()
    } else if path.ends_with("/stop") {
        state.stop_entered.store(true, Ordering::Release);
        let deadline = Instant::now() + Duration::from_secs(2);
        while state.stop_blocked.load(Ordering::Acquire) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        let mut r = state.receipt.lock().unwrap();
        let r = r.as_mut().unwrap();
        r["phase"] = json!("terminal");
        if state.uncertain.load(Ordering::Acquire) {
            r["outcome"] = json!("uncertain");
            r["release_observed_unix_us"] = Value::Null
        } else if r["actual_start_unix_us"].is_null() {
            r["outcome"] = json!("no_start")
        } else {
            r["outcome"] = json!("release_observed");
            r["release_observed_unix_us"] = json!(r["actual_start_unix_us"].as_u64().unwrap() + 1)
        }
        r.clone()
    } else if path.starts_with("/xlog/v1/tx/status?") {
        let id = path.split('=').last().unwrap();
        json!({"schema_version":1,"server_instance":S,"workspace_id":W,"operation_id":id,"receipt":state.receipt.lock().unwrap().clone()})
    } else {
        panic!("unexpected fixture path {path}")
    };
    let bytes = serde_json::to_vec(&response).unwrap();
    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",bytes.len()).unwrap();
    stream.write_all(&bytes).unwrap();
}
fn setup() -> (
    Fake,
    xnexus_xlog_client::BridgeClient,
    Host,
    xnexus_xlog_client::audio::AudioClient,
) {
    let fake = Fake::new();
    let mut bridge = xnexus_xlog_client::BridgeClient::new(
        &fake.url,
        "synthetic-ft-token-abcdefghijklmnopqrstuvwxyz",
    )
    .unwrap();
    bridge.connect().unwrap();
    let client_id = uuid().unwrap();
    let tx = bridge.tx_handle(&client_id).unwrap();
    let host = Host {
        inner: allocate(tx, client_id, "local-token".into()),
    };
    let c = host.inner.client.context().unwrap();
    lock(&host.inner.state).unwrap().context = Some(Arc::new(c));
    let audio = bridge.audio_handle().unwrap();
    (fake, bridge, host, audio)
}
use sha2::Digest;
#[test]
fn action_contract_keeps_manual_target_auto_cycle_hold_and_zero_drive_distinct() {
    assert!(matches!(
        serde_json::from_value::<Action>(json!({"kind":"cq","direction":"DX"})).unwrap(),
        Action::Cq { direction: Some(_) }
    ));
    assert!(matches!(
        serde_json::from_value::<Action>(json!({"kind":"auto_cycle","enabled":true})).unwrap(),
        Action::AutoCycle { enabled: true }
    ));
    assert!(serde_json::from_value::<Action>(json!({"kind":"work","call":"W9XYZ"})).is_err());
    let (_fake, _bridge, host, _audio) = setup();
    assert!(!host.view().unwrap().armed);
    host.action(Action::Hold { enabled: true }).unwrap();
    assert!(host.view().unwrap().hold);
    assert!(!host.inner.stop_requested.load(Ordering::Acquire));
    host.action(Action::Drive { gain: 0.0 }).unwrap();
    assert_eq!(host.view().unwrap().gain, 0.0);
    assert!(host.action(Action::Tune { enabled: true }).is_err());
}
#[test]
fn actual_sdk_tune_stop_is_independent_and_unknown_keeps_exact_obligation() {
    let (fake, bridge, host, _audio) = setup();
    host.action(Action::Tune { enabled: true }).unwrap();
    assert!(host.view().unwrap().tuning);
    let id = host.view().unwrap().operation_id.unwrap();
    fake.state.uncertain.store(true, Ordering::Release);
    let start = Instant::now();
    host.stop();
    assert!(start.elapsed() < Duration::from_millis(50));
    let (stop, event) = lock(&host.inner.active)
        .unwrap()
        .as_ref()
        .map(|a| (a.stop_client.clone(), a.stop_event.clone()))
        .unwrap();
    let p = stop.stop(&event).unwrap();
    apply_proof(&host.inner, p).unwrap();
    assert_eq!(
        host.view().unwrap().operation_id.as_deref(),
        Some(id.as_str())
    );
    drop(bridge);
    fake.state.uncertain.store(false, Ordering::Release);
    let p = stop.stop(&event).unwrap();
    apply_proof(&host.inner, p).unwrap();
    assert!(!host.view().unwrap().release_pending);
    assert!(!OPERATION_HELD.load(Ordering::Acquire));
}
#[test]
fn independent_heartbeat_progresses_while_ordinary_prepare_is_blocked() {
    let (fake, _bridge, host, _audio) = setup();
    let c = host.inner.current().unwrap();
    let c = host
        .inner
        .client
        .enable(&c, &uuid().unwrap(), true)
        .unwrap();
    lock(&host.inner.state).unwrap().context = Some(Arc::new(c));
    host.inner.future_disarmed.store(false, Ordering::Release);
    fake.state.prepare_blocked.store(true, Ordering::Release);
    let worker = host.inner.clone();
    let heartbeat = thread::spawn(move || heartbeat_loop(worker));
    let worker = host.inner.clone();
    let epoch = worker.epoch.load(Ordering::Acquire);
    let tune = thread::spawn(move || start_tune(&worker, epoch));
    let start = Instant::now();
    while !fake.state.prepare_entered.load(Ordering::Acquire)
        && start.elapsed() < Duration::from_secs(3)
    {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(fake.state.prepare_entered.load(Ordering::Acquire));
    thread::sleep(Duration::from_millis(450));
    assert!(fake.state.heartbeats.load(Ordering::Acquire) >= 2);
    host.stop();
    fake.state.prepare_blocked.store(false, Ordering::Release);
    let _ = tune.join().unwrap();
    let (stop, event) = lock(&host.inner.active)
        .unwrap()
        .as_ref()
        .map(|a| (a.stop_client.clone(), a.stop_event.clone()))
        .unwrap();
    let p = stop.stop(&event).unwrap();
    apply_proof(&host.inner, p).unwrap();
    host.revoke();
    heartbeat.join().unwrap();
    assert!(!OPERATION_HELD.load(Ordering::Acquire));
}

fn accepted(
    host: &Host,
    audio: &mut xnexus_xlog_client::audio::AudioClient,
) -> Arc<NativeDecodedPeriod> {
    accepted_generation(host, audio, 1, None)
}
fn accepted_generation(
    host: &Host,
    audio: &mut xnexus_xlog_client::audio::AudioClient,
    generation: u64,
    period_start: Option<u64>,
) -> Arc<NativeDecodedPeriod> {
    let b = Arc::new(audio.poll().unwrap());
    host.feed().touch(b.clone());
    let descriptor = b.descriptor.as_ref().unwrap();
    let owner = OwnerIdentity {
        server_instance: S.into(),
        workspace_id: W.into(),
        capture_id: descriptor.capture_id.clone(),
        epoch: descriptor.epoch.clone(),
        radio_id: "radio".into(),
        receiver_id: "A".into(),
        mapping_generation: descriptor.mapping_generation,
        device_id: "synthetic-input".into(),
        slice: "A".into(),
        dax_channel: 1,
        frequency_hz: 14074000,
        radio_mode: "DIGU".into(),
        sample_rate: 12000,
        source_channels: 1,
        selected_channel: 0,
    };
    let start =
        period_start.unwrap_or_else(|| wall_us().unwrap() / 7_500_000 * 7_500_000 - 7_500_000);
    let p = Arc::new(NativeDecodedPeriod {
        context: DecodeContext {
            stream_id: descriptor.capture_id.clone(),
            epoch: descriptor.epoch.clone(),
            mapping_generation: descriptor.mapping_generation,
            generation,
            radio_id: "radio".into(),
            receiver_id: "A".into(),
            mode: DecoderMode::Ft4,
            dial_hz: 14074000,
        },
        owner,
        host_period_start_unix_us: start,
        frame_time_ms: 1000,
        clock: ClockProvenance {
            basis: TimestampBasis::WindowsEndpointCapture,
            host_mapping_uncertainty_us: 10,
            calibration_uncertainty_us: 10,
            utc_sync_accuracy_us: None,
            remote_rf_latency_us: None,
            sample_alignment_offset_ns: 0,
            resampler_center_offset_ns: 0,
        },
        rows: vec![tempo_core::modes::Decode {
            message: "CQ W9XYZ EN50".into(),
            sync: 1.0,
            snr: -10,
            dt: 0.0,
            freq: 1500.0,
            nap: 0,
            qual: 1.0,
            rv: None,
            mode: Some(tempo_core::modes::ModeKind::Ft4),
            raw: None,
        }],
    });
    host.feed().publish(
        p.clone(),
        vec![Some(if generation == 1 {
            "native-row".into()
        } else {
            format!("native-row-{generation}")
        })],
        b,
    );
    consume(&host.inner).unwrap();
    p
}
#[test]
fn temporary_audio_expiry_preserves_qso_but_not_work_admission() {
    let (_fake, _bridge, host, mut audio) = setup();
    accepted(&host, &mut audio);
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    audio.disconnect(); // Retires the actual sample authority, not owner TX context.
    consume(&host.inner).unwrap();
    assert_eq!(host.view().unwrap().dx_call.as_deref(), Some("W9XYZ"));
    assert!(lock(&host.inner.state).unwrap().coordinator.is_some());
    assert!(host
        .action(Action::Work {
            decode_id: "native-row".into()
        })
        .is_err());
    host.feed().retire(); // Genuine route/full Stop still retires the sequencer.
    consume(&host.inner).unwrap();
    assert!(lock(&host.inner.state).unwrap().coordinator.is_none());
}
#[test]
fn capture_repair_preserves_qso_log_and_rejects_work_while_release_is_pending() {
    let (fake, _bridge, host, mut audio) = setup();
    let original = accepted(&host, &mut audio);
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    let log = host.log_draft().unwrap();
    let before = host.view().unwrap();
    let (operation, stop) = {
        let mut state = lock(&host.inner.state).unwrap();
        let coordinator = state.coordinator.as_mut().unwrap();
        let now = Instant::now();
        let wall = wall_us().unwrap();
        let mut slot = (wall / 7_500_000 + 1) * 7_500_000;
        if slot / 7_500_000 % 2 != u64::from(!coordinator.view().unwrap().tx_even) {
            slot += 7_500_000;
        }
        let plan = coordinator.plan(slot, slot + 50_000, wall, now).unwrap();
        let operation = kernel::Operation {
            key: plan.key().clone(),
            id: "original-owned-operation".into(),
            body_digest: [7; 32],
        };
        coordinator.bind(&plan, operation.clone(), now).unwrap();
        (operation, coordinator.stop_handle())
    };
    fake.state.audio_generation.store(2, Ordering::Release);
    // An actual SDK audio reply changes all stream coordinates. The complete
    // accepted period is new; old decoded records are not replayed as input.
    let repaired = accepted_generation(
        &host,
        &mut audio,
        2,
        Some(original.host_period_start_unix_us + 7_500_000),
    );
    let after = host.view().unwrap();
    assert_eq!(after.qso_state, before.qso_state);
    assert_eq!(after.dx_call, before.dx_call);
    assert_eq!(host.log_draft().unwrap(), log);
    assert_eq!(
        lock(&host.inner.state)
            .unwrap()
            .key
            .as_ref()
            .unwrap()
            .receive,
        repaired.context
    );
    assert!(host
        .action(Action::Work {
            decode_id: "native-row".into()
        })
        .is_err());
    assert_eq!(stop.release().unwrap().as_deref(), Some(&operation));
    host.stop();
    assert_eq!(stop.release().unwrap().as_deref(), Some(&operation));
    assert_eq!(stop.stop().unwrap().as_deref(), Some(&operation));
    assert_eq!(
        operation.key.context.owner.capture_id,
        original.owner.capture_id
    );
    assert_ne!(
        operation.key.context.owner.capture_id,
        repaired.owner.capture_id
    );
}
#[test]
fn explicit_work_selects_retained_message_after_stream_repair_not_other_station() {
    let (fake, _bridge, host, mut audio) = setup();
    let original = accepted(&host, &mut audio);
    fake.state.audio_generation.store(2, Ordering::Release);
    let repaired = accepted_generation(
        &host,
        &mut audio,
        2,
        Some(original.host_period_start_unix_us + 7_500_000),
    );
    assert_ne!(original.owner.capture_id, repaired.owner.capture_id);
    assert_ne!(original.owner.epoch, repaired.owner.epoch);
    assert_ne!(
        original.owner.mapping_generation,
        repaired.owner.mapping_generation
    );
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    assert_eq!(host.view().unwrap().dx_call.as_deref(), Some("W9XYZ"));
    assert!(host.view().unwrap().armed);
    assert_eq!(host.view().unwrap().tx_count, 0);
    assert!(host.view().unwrap().operation_id.is_none());
    assert_eq!(
        lock(&host.inner.state)
            .unwrap()
            .key
            .as_ref()
            .unwrap()
            .receive,
        repaired.context
    );

    // The selected record is never published again as a decoded period, nor
    // allowed to borrow current authority for another physical station/mode.
    for change in 0..6 {
        let mut row = NativeDecodedPeriod {
            context: original.context.clone(),
            owner: original.owner.clone(),
            host_period_start_unix_us: original.host_period_start_unix_us,
            frame_time_ms: original.frame_time_ms,
            clock: original.clock.clone(),
            rows: original.rows.clone(),
        };
        match change {
            0 => {
                row.context.dial_hz += 1000;
                row.owner.frequency_hz += 1000;
            }
            1 => row.context.mode = DecoderMode::Ft8,
            2 => row.owner.workspace_id = C.into(),
            3 => row.owner.device_id = "different-input".into(),
            4 => {
                row.context.radio_id = "another-radio".into();
                row.owner.radio_id = "another-radio".into();
            }
            _ => {
                row.context.receiver_id = "B".into();
                row.owner.receiver_id = "B".into();
            }
        }
        lock(&host.inner.state).unwrap().retained.push_back((
            "other-station".into(),
            Arc::new(row),
            0,
        ));
        assert_eq!(
            host.action(Action::Work {
                decode_id: "other-station".into()
            })
            .unwrap_err(),
            "Selected decode context changed"
        );
        lock(&host.inner.state).unwrap().retained.pop_back();
    }
    assert_eq!(host.view().unwrap().dx_call.as_deref(), Some("W9XYZ"));
    host.feed().retire();
    assert!(host
        .action(Action::Work {
            decode_id: "native-row".into()
        })
        .is_err());
}
#[test]
fn enable_context_publication_does_not_cancel_operator_intent() {
    let (_fake, _bridge, host, mut audio) = setup();
    accepted(&host, &mut audio);
    let epoch = host.inner.epoch.load(Ordering::Acquire);
    let lane = lock(&host.inner.context_lane).unwrap();
    let old = host.inner.current().unwrap();
    let enabled = host
        .inner
        .client
        .enable(&old, &uuid().unwrap(), true)
        .unwrap();
    assert!(!old.is_fresh(), "actual SDK retires the old disabled lease");
    assert!(enabled.is_fresh());
    consume(&host.inner).unwrap();
    assert_eq!(host.inner.epoch.load(Ordering::Acquire), epoch);
    assert!(!host.inner.stop_requested.load(Ordering::Acquire));
    lock(&host.inner.state).unwrap().context = Some(Arc::new(enabled));
    drop(lane);
    consume(&host.inner).unwrap();
    assert_eq!(host.inner.epoch.load(Ordering::Acquire), epoch);
    host.action(Action::Enable { enabled: true }).unwrap();
    assert!(host.view().unwrap().armed);
    assert!(!host.view().unwrap().keyed);
    assert!(host.view().unwrap().operation_id.is_none());
}
#[test]
fn explicit_enable_fetches_current_owner_context_after_cached_view_expires() {
    let (fake, _bridge, host, mut audio) = setup();
    accepted(&host, &mut audio);
    fake.state.ready.store(false, Ordering::Release);
    let unavailable = Arc::new(host.inner.client.context().unwrap());
    assert!(unavailable.is_fresh());
    assert!(!unavailable.context().ready);
    lock(&host.inner.state).unwrap().context = Some(unavailable);
    assert!(!host.view().unwrap().available);
    fake.state.ready.store(true, Ordering::Release);
    host.action(Action::Enable { enabled: true }).unwrap();
    assert!(host.view().unwrap().armed);
    let old = host.inner.current().unwrap();
    thread::sleep(Duration::from_millis(1050));
    assert!(!old.is_fresh());
    assert!(!host.view().unwrap().available);
    host.action(Action::Enable { enabled: true }).unwrap();
    assert!(!old.is_fresh(), "old observation was not extended");
    assert!(host.view().unwrap().armed);
    assert!(host.view().unwrap().available);
    assert!(!host.view().unwrap().keyed);
    assert!(host.view().unwrap().operation_id.is_none());
}
#[test]
fn native_record_mailbox_work_auto_parity_hold_and_retirement() {
    let (_fake, _bridge, host, mut audio) = setup();
    let p = accepted(&host, &mut audio);
    assert!(host
        .action(Action::Work {
            decode_id: "invented".into()
        })
        .is_err());
    host.action(Action::Hold { enabled: true }).unwrap();
    host.action(Action::Offset { hz: 2100.0 }).unwrap();
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    let v = host.view().unwrap();
    assert_eq!(v.dx_call.as_deref(), Some("W9XYZ"));
    assert_eq!(v.offset_hz, 2100.0);
    assert!(v.hold);
    assert_eq!(v.tx_even, p.host_period_start_unix_us / 7_500_000 % 2 != 0);
    host.action(Action::Parity { even: true }).unwrap();
    assert!(!host.view().unwrap().tx_cycle_auto);
    host.feed().retire();
    consume(&host.inner).unwrap();
    assert!(!host.view().unwrap().armed);
    assert!(host
        .action(Action::Work {
            decode_id: "native-row".into()
        })
        .is_err());
}
#[test]
fn terminal_first_sealed_proof_accounts_start_once_then_release_and_stable_log() {
    let (fake, _bridge, host, mut audio) = setup();
    accepted(&host, &mut audio);
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    fake.state.terminal.store(true, Ordering::Release);
    let worker = host.inner.clone();
    let heartbeat = thread::spawn(move || heartbeat_loop(worker));
    let wall = wall_us().unwrap();
    let mut slot = (wall / 7_500_000 + 2) * 7_500_000;
    let (plan, cfg, c, epoch) = {
        let mut s = lock(&host.inner.state).unwrap();
        let cfg = s.config.clone();
        if slot / 7_500_000 % 2 != u64::from(!cfg.even) {
            slot += 7_500_000;
        }
        let plan = s
            .coordinator
            .as_mut()
            .unwrap()
            .plan(slot, slot + 50_000, wall, Instant::now())
            .unwrap();
        (
            plan,
            cfg,
            s.context.clone().unwrap(),
            host.inner.epoch.load(Ordering::Acquire),
        )
    };
    prepare_operation(&host.inner, plan, c, cfg, epoch).unwrap();
    assert_eq!(host.view().unwrap().tx_count, 1);
    assert!(!host.view().unwrap().release_pending);
    let a = host.log_draft().unwrap();
    let b = host.log_draft().unwrap();
    assert_eq!(a, b);
    assert_eq!(a.1.callsign, "W9XYZ");
    assert_eq!(a.1.band, "20m");
    host.stop();
    assert_eq!(host.log_draft().unwrap(), a);
    host.revoke();
    heartbeat.join().unwrap();
    assert!(!OPERATION_HELD.load(Ordering::Acquire));
}

#[test]
fn duplicate_terminal_proof_never_stops_or_releases_a_replacement() {
    let (_fake, _bridge, host, _audio) = setup();
    host.action(Action::Tune { enabled: true }).unwrap();
    let (stop, event) = lock(&host.inner.active)
        .unwrap()
        .as_ref()
        .map(|a| (a.stop_client.clone(), a.stop_event.clone()))
        .unwrap();
    host.stop();
    let terminal = stop.stop(&event).unwrap();
    apply_proof(&host.inner, terminal.clone()).unwrap();
    apply_proof(&host.inner, terminal.clone()).unwrap();
    // This fixture intentionally has no background heartbeat. A new operator
    // intent obtains a new owner observation; it never renews the old lease.
    let fresh = host.inner.client.context().unwrap();
    lock(&host.inner.state).unwrap().context = Some(Arc::new(fresh));
    host.action(Action::Tune { enabled: true }).unwrap();
    let replacement = host.view().unwrap().operation_id.unwrap();
    apply_proof(&host.inner, terminal).unwrap();
    assert_eq!(
        host.view().unwrap().operation_id.as_deref(),
        Some(replacement.as_str())
    );
    assert!(!host.inner.stop_requested.load(Ordering::Acquire));
    let (stop, event) = lock(&host.inner.active)
        .unwrap()
        .as_ref()
        .map(|a| (a.stop_client.clone(), a.stop_event.clone()))
        .unwrap();
    host.stop();
    apply_proof(&host.inner, stop.stop(&event).unwrap()).unwrap();
    assert!(!OPERATION_HELD.load(Ordering::Acquire));
}

fn transport_threads(host: &Host) -> [thread::JoinHandle<()>; 2] {
    let i = host.inner.clone();
    let h = thread::spawn(move || heartbeat_loop(i));
    let i = host.inner.clone();
    let r = thread::spawn(move || release_loop(i));
    [h, r]
}
fn wait_released(host: &Host) {
    let until = Instant::now() + Duration::from_secs(3);
    while host.has_release() && Instant::now() < until {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(!host.has_release());
}
#[test]
fn tune_drive_releases_exact_old_operation_before_explicit_gain_replacement() {
    let (fake, _bridge, host, _audio) = setup();
    host.action(Action::Tune { enabled: true }).unwrap();
    let first = host.view().unwrap().operation_id.unwrap();
    let tasks = transport_threads(&host);
    host.action(Action::Drive { gain: 0.25 }).unwrap();
    assert_eq!(fake.state.tune_count.load(Ordering::Acquire), 2);
    assert_eq!(
        fake.state.tune_gain.load(Ordering::Acquire),
        u64::from(0.25f32.to_bits())
    );
    assert_ne!(
        host.view().unwrap().operation_id.as_deref(),
        Some(first.as_str())
    );
    host.action(Action::Drive { gain: 0.0 }).unwrap();
    wait_released(&host);
    assert_eq!(fake.state.tune_count.load(Ordering::Acquire), 2);
    host.revoke();
    for t in tasks {
        t.join().unwrap();
    }
}
#[test]
fn tune_drive_unknown_release_never_rekeys() {
    let (fake, _bridge, host, _audio) = setup();
    host.action(Action::Tune { enabled: true }).unwrap();
    let tasks = transport_threads(&host);
    fake.state.uncertain.store(true, Ordering::Release);
    assert!(host.action(Action::Drive { gain: 0.3 }).is_err());
    assert_eq!(fake.state.tune_count.load(Ordering::Acquire), 1);
    assert!(host.has_release());
    fake.state.uncertain.store(false, Ordering::Release);
    wait_released(&host);
    host.revoke();
    for t in tasks {
        t.join().unwrap();
    }
}
#[test]
fn urgent_stop_cancels_user_drive_restart_while_old_release_is_pending() {
    let (fake, _bridge, host, _audio) = setup();
    let host = Arc::new(host);
    host.action(Action::Tune { enabled: true }).unwrap();
    let tasks = transport_threads(&host);
    fake.state.stop_blocked.store(true, Ordering::Release);
    let h = host.clone();
    let action = thread::spawn(move || h.action(Action::Drive { gain: 0.4 }));
    let until = Instant::now() + Duration::from_secs(2);
    while !fake.state.stop_entered.load(Ordering::Acquire) && Instant::now() < until {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(fake.state.stop_entered.load(Ordering::Acquire));
    host.stop();
    fake.state.stop_blocked.store(false, Ordering::Release);
    assert!(action.join().unwrap().is_err());
    wait_released(&host);
    assert_eq!(fake.state.tune_count.load(Ordering::Acquire), 1);
    host.revoke();
    for t in tasks {
        t.join().unwrap();
    }
}

#[test]
fn commissioned_tune_is_available_before_receive_but_cq_waits() {
    let (_fake, _bridge, host, _audio) = setup();
    let view = host.view().unwrap();
    assert!(view.available);
    assert_eq!(view.mode, None);
    assert!(!view.keyed);
    assert!(host.action(Action::Cq { direction: None }).is_err());
    assert!(lock(&host.inner.state).unwrap().coordinator.is_none());
}
#[test]
fn retained_original_ids_are_bounded_selectable_and_retired_without_replay() {
    let (_fake, _bridge, host, mut audio) = setup();
    let p = accepted(&host, &mut audio);
    {
        let mut s = lock(&host.inner.state).unwrap();
        let r = s.receive.as_ref().unwrap().clone();
        for n in 0..201 {
            let mut selection = r.clone();
            selection.ids = vec![Some(format!("retained-{n}"))];
            s.retain_receive(&selection);
        }
        assert_eq!(s.retained.len(), 200);
        assert!(s
            .retained
            .iter()
            .all(|(_, period, _)| Arc::ptr_eq(period, &p)));
        assert_eq!(s.coordinator.as_ref().unwrap().view().unwrap().tx_count, 0);
    }
    assert!(host
        .action(Action::Work {
            decode_id: "retained-0".into()
        })
        .is_err());
    host.action(Action::Work {
        decode_id: "retained-1".into(),
    })
    .unwrap();
    assert_eq!(host.view().unwrap().dx_call.as_deref(), Some("W9XYZ"));
    host.feed().retire();
    // Immediate fence works even before the control loop consumes retirement.
    assert!(host
        .action(Action::Work {
            decode_id: "retained-200".into()
        })
        .is_err());
    consume(&host.inner).unwrap();
    assert!(lock(&host.inner.state).unwrap().retained.is_empty());
}
#[test]
fn manual_log_current_unkeyed_target_is_stable_and_never_previous_qso() {
    let (_fake, _bridge, host, mut audio) = setup();
    accepted(&host, &mut audio);
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    let first = host.log_draft().unwrap();
    assert_eq!(first.1.callsign, "W9XYZ");
    assert!(first.1.rst_sent.is_empty());
    assert!(!host.view().unwrap().keyed);
    assert_eq!(host.log_draft().unwrap(), first);
    host.action(Action::Override {
        message: "W1AW K1ABC FN42".into(),
        call: Some("W1AW".into()),
        grid: Some("FN31".into()),
    })
    .unwrap();
    let second = host.log_draft().unwrap();
    assert_eq!(second.1.callsign, "W1AW");
    assert_eq!(second.1.grid, "FN31");
    assert!(second.1.rst_sent.is_empty());
    assert_ne!(second.0, first.0);
    host.action(Action::Cq { direction: None }).unwrap();
    assert!(host.log_draft().is_err());
}

// Exercise the same publication/SDK dispatch function used by prepare_operation,
// with actual immutable encoder output and the actual SDK's captured context lease.
fn dispatch_fixture(host: &Host) -> (kernel::Plan, Arc<PreparedOperation>, Arc<ContextLease>) {
    let wall = wall_us().unwrap();
    let mut slot = (wall / 7_500_000 + 2) * 7_500_000;
    let (plan, cfg) = {
        let mut s = lock(&host.inner.state).unwrap();
        let cfg = s.config.clone();
        if slot / 7_500_000 % 2 != u64::from(!cfg.even) {
            slot += 7_500_000;
        }
        let p = s
            .coordinator
            .as_mut()
            .unwrap()
            .plan(slot, slot + 50_000, wall, Instant::now())
            .unwrap();
        (p, cfg)
    };
    let source = plan.prepare(cfg.offset).unwrap();
    let output = xlog_ft_output::prepare(&source, cfg.gain, plan.deadline()).unwrap();
    let c = Arc::new(host.inner.client.context().unwrap());
    let d = wire::Prepare {
        schema_version: 2,
        binding: c.context().binding.clone(),
        client_id: host.inner.client_id.clone(),
        operation_id: uuid().unwrap(),
        event_id: uuid().unwrap(),
        slot_start_unix_us: plan.key().slot_start_us,
        latest_start_unix_us: plan.key().latest_start_us,
        manifest: serde_json::from_slice(output.manifest()).unwrap(),
    };
    let op = Arc::new(
        host.inner
            .client
            .build_prepared(&c, d, output.pcm())
            .unwrap(),
    );
    (plan, op, c)
}
fn refresh_dispatch_fixture(host: &Host) {
    let current = Arc::new(host.inner.client.context().unwrap());
    let mut s = lock(&host.inner.state).unwrap();
    let key = s.key.clone().unwrap();
    s.coordinator
        .as_mut()
        .unwrap()
        .refresh(
            &key,
            Instant::now() + Duration::from_millis(current.remaining_valid_ms()),
            Instant::now(),
        )
        .unwrap();
    s.context = Some(current);
}
#[test]
fn never_dispatched_prepare_does_not_retain_release_or_replay_slot() {
    let (fake, _bridge, host, mut audio) = setup();
    accepted(&host, &mut audio);
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    let (plan, op, original) = dispatch_fixture(&host);
    thread::sleep(Duration::from_millis(original.remaining_valid_ms() + 25));
    assert!(!original.is_fresh());
    refresh_dispatch_fixture(&host); // Fresh current authority does not renew the old operation's lease.
    let epoch = host.inner.epoch.load(Ordering::Acquire);
    let reason =
        publish_and_prepare(&host.inner, &plan, op.clone(), epoch, 1000, &original).unwrap_err();
    assert!(reason.contains("Stale"));
    assert!(!op.initial_dispatch_attempted());
    assert_eq!(fake.state.prepare_requests.load(Ordering::Acquire), 0);
    host.inner.fail(reason.clone());
    assert!(
        !host.view().unwrap().release_pending,
        "never-sent prepare must not invent an owner release obligation"
    );
    assert_eq!(host.view().unwrap().reason, reason);
    assert!(!OPERATION_HELD.load(Ordering::Acquire));
    let mut s = lock(&host.inner.state).unwrap();
    let c = s.coordinator.as_mut().unwrap();
    assert!(c.stop_handle().release().unwrap().is_none());
    assert_eq!(c.view().unwrap().tx_count, 0);
    c.apply(kernel::Intent::Enable(true), Instant::now())
        .unwrap();
    assert!(matches!(
        c.plan(
            plan.key().slot_start_us,
            plan.key().latest_start_us,
            wall_us().unwrap(),
            Instant::now()
        ),
        Err(kernel::Error::SlotUsed)
    ));
}
#[test]
fn dispatched_prepare_failure_retains_exact_release_and_first_reason() {
    let (fake, _bridge, host, mut audio) = setup();
    accepted(&host, &mut audio);
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    let (plan, op, original) = dispatch_fixture(&host);
    refresh_dispatch_fixture(&host);
    fake.state
        .drop_prepare_response
        .store(true, Ordering::Release);
    let epoch = host.inner.epoch.load(Ordering::Acquire);
    let reason = publish_and_prepare(
        &host.inner,
        &plan,
        op.clone(),
        epoch,
        original.remaining_valid_ms(),
        &host.inner.current().unwrap(),
    )
    .unwrap_err();
    assert!(reason.contains("OutcomeUnknown"));
    assert!(op.initial_dispatch_attempted());
    assert_eq!(fake.state.prepare_requests.load(Ordering::Acquire), 1);
    host.inner.fail(reason.clone());
    host.inner
        .fail("Owner receipt missing; exact release remains pending".into());
    assert!(host.view().unwrap().release_pending);
    assert_eq!(
        host.view().unwrap().reason,
        reason,
        "generic polling must not erase the original prepare failure"
    );
    let active = lock(&host.inner.active).unwrap();
    let a = active.as_ref().unwrap();
    assert_eq!(a.operation.key(), op.key());
    assert!(a.query_ready);
    assert_eq!(a.native.as_ref().unwrap().id, op.key().operation_id);
    drop(active);
    // Test-process teardown only: no owner terminal receipt or RF outcome is fabricated.
    *lock(&host.inner.active).unwrap() = None;
    OPERATION_HELD.store(false, Ordering::Release);
}

#[test]
fn fresh_matching_admission_dispatches_immutable_body_once_after_original_expires() {
    let (fake, _bridge, host, mut audio) = setup();
    accepted(&host, &mut audio);
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    let (plan, op, original) = dispatch_fixture(&host);
    let original_body = op.body().to_vec();
    thread::sleep(Duration::from_millis(original.remaining_valid_ms() + 25));
    assert!(matches!(
        host.inner.client.prepare(&op),
        Err(xnexus_xlog_client::Error::Stale)
    ));
    assert_eq!(fake.state.prepare_requests.load(Ordering::Acquire), 0);
    assert!(!op.initial_dispatch_attempted());
    fake.state
        .max_prepare_bytes
        .store(4194303, Ordering::Release);
    let changed = host.inner.client.context().unwrap();
    assert!(host
        .inner
        .client
        .prepare_with_context(&changed, &op)
        .is_err());
    fake.state
        .max_prepare_bytes
        .store(4194304, Ordering::Release);
    fake.state.changed_tx_route.store(true, Ordering::Release);
    let changed = host.inner.client.context().unwrap();
    assert!(host
        .inner
        .client
        .prepare_with_context(&changed, &op)
        .is_err());
    assert_eq!(fake.state.prepare_requests.load(Ordering::Acquire), 0);
    assert!(!op.initial_dispatch_attempted());
    fake.state.changed_tx_route.store(false, Ordering::Release);
    refresh_dispatch_fixture(&host);
    let current = host.inner.current().unwrap();
    assert!(current.is_fresh());
    let epoch = host.inner.epoch.load(Ordering::Acquire);
    publish_and_prepare(&host.inner, &plan, op.clone(), epoch, 1000, &current).unwrap();
    assert_eq!(op.body(), original_body);
    assert!(op.initial_dispatch_attempted());
    assert_eq!(fake.state.prepare_requests.load(Ordering::Acquire), 1);
    assert!(host
        .inner
        .client
        .prepare_with_context(&current, &op)
        .is_err());
    assert_eq!(fake.state.prepare_requests.load(Ordering::Acquire), 1);
    assert_eq!(host.view().unwrap().tx_count, 0); // Prepared is not actual start.
    let (stop, event) = lock(&host.inner.active)
        .unwrap()
        .as_ref()
        .map(|a| (a.stop_client.clone(), a.stop_event.clone()))
        .unwrap();
    apply_proof(&host.inner, stop.stop(&event).unwrap()).unwrap();
    assert!(!host.view().unwrap().release_pending);
    assert!(!OPERATION_HELD.load(Ordering::Acquire));
}

#[test]
fn failed_post_body_context_cancels_only_unbound_plan() {
    let (fake, _bridge, host, mut audio) = setup();
    accepted(&host, &mut audio);
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    let wall = wall_us().unwrap();
    let mut slot = (wall / 7_500_000 + 2) * 7_500_000;
    let (plan, cfg, context, epoch) = {
        let mut s = lock(&host.inner.state).unwrap();
        let cfg = s.config.clone();
        if slot / 7_500_000 % 2 != u64::from(!cfg.even) {
            slot += 7_500_000;
        }
        let p = s
            .coordinator
            .as_mut()
            .unwrap()
            .plan(slot, slot + 50_000, wall, Instant::now())
            .unwrap();
        (
            p,
            cfg,
            s.context.clone().unwrap(),
            host.inner.epoch.load(Ordering::Acquire),
        )
    };
    fake.state.fail_context.store(true, Ordering::Release);
    let reason = prepare_operation(&host.inner, plan, context, cfg, epoch).unwrap_err();
    assert_eq!(
        fake.state.failed_context_requests.load(Ordering::Acquire),
        1,
        "actual fresh GET after body construction must have failed"
    );
    assert_eq!(fake.state.prepare_requests.load(Ordering::Acquire), 0);
    host.inner.fail(reason);
    assert!(!host.view().unwrap().release_pending);
    assert!(!OPERATION_HELD.load(Ordering::Acquire));
    fake.state.fail_context.store(false, Ordering::Release);
    audio = _bridge.audio_handle().unwrap();
    host.feed().touch(Arc::new(audio.poll().unwrap()));
    host.action(Action::Work {
        decode_id: "native-row".into(),
    })
    .unwrap();
    assert!(
        host.view().unwrap().armed,
        "a new explicit operator intent is not stuck Busy"
    );
    assert_eq!(host.view().unwrap().tx_count, 0);
}
