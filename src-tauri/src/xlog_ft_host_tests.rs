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
    enabled: AtomicBool,
    receipt: Mutex<Option<Value>>,
    terminal: AtomicBool,
    uncertain: AtomicBool,
    heartbeats: AtomicU64,
    slot: AtomicU64,
    prepare_blocked: AtomicBool,
    prepare_entered: AtomicBool,
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
            enabled: AtomicBool::new(false),
            receipt: Mutex::new(None),
            terminal: AtomicBool::new(false),
            uncertain: AtomicBool::new(false),
            heartbeats: AtomicU64::new(0),
            slot: AtomicU64::new(0),
            prepare_blocked: AtomicBool::new(false),
            prepare_entered: AtomicBool::new(false),
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
        self.cancel.store(true, Ordering::Release);
        self.thread.take().unwrap().join().unwrap();
    }
}
fn context(state: &FakeState) -> Value {
    json!({"schema_version":1,"binding":{"server_instance":S,"workspace_id":W,"context_id":C,"clock_epoch":K},"ready":true,"reason":null,"local_enabled":true,"client_enabled":state.enabled.load(Ordering::Acquire),"identity":{"mycall":"K1ABC","mygrid":"FN42","worker_generation":1,"settings_revision":1,"identity_epoch":C,"identity_revision":1},"route":{"radio_id":"radio","receiver_id":"A","transmit_slice":"A","device_id":"synthetic-output","route_epoch":K,"sample_rate_hz":48000,"channels":2,"selected_channels":[0],"frequency_hz":14074000,"radio_mode":"DIGU"},"generated_at_unix_ms":wall_us().unwrap()/1000,"max_age_ms":1000,"heartbeat_interval_ms":200,"heartbeat_timeout_ms":1000,"max_prepare_bytes":4194304,"min_offset_hz":100,"max_offset_hz":5000,"max_abs_sample_f32_bits":0.95f32.to_bits()})
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
        json!({"protocol_version":1,"server_instance":S,"workspace_id":W,"generated_at_unix_ms":wall_us().unwrap()/1000,"max_age_ms":1000,"state":"running","reason":null,"descriptor":{"capture_id":C,"epoch":K,"radio_id":"radio","receiver_id":"A","mapping_generation":1,"device_id":"synthetic-input","slice":"A","dax_channel":1,"frequency_hz":14074000,"mode":"DIGU","source_channels":1,"selected_channel":0,"sample_rate":12000,"sample_format":"s16le","timestamp_basis":"windows_endpoint_capture","remote_rf_latency_us":null,"utc_sync_accuracy_us":null},"oldest_sequence":after+1,"next_sequence":after+2,"gap":false,"chunks":[{"sequence":after+1,"first_frame":after,"frame_count":1,"sample0_unix_us":wall_us().unwrap(),"calibration_uncertainty_us":10,"pcm_base64":"AAA="}]})
    } else if path.ends_with("/context") {
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
    let b = Arc::new(audio.poll().unwrap());
    host.feed().touch(b.clone());
    let owner = OwnerIdentity {
        server_instance: S.into(),
        workspace_id: W.into(),
        capture_id: C.into(),
        epoch: K.into(),
        radio_id: "radio".into(),
        receiver_id: "A".into(),
        mapping_generation: 1,
        device_id: "synthetic-input".into(),
        slice: "A".into(),
        dax_channel: 1,
        frequency_hz: 14074000,
        radio_mode: "DIGU".into(),
        sample_rate: 12000,
        source_channels: 1,
        selected_channel: 0,
    };
    let start = wall_us().unwrap() / 7_500_000 * 7_500_000 - 7_500_000;
    let p = Arc::new(NativeDecodedPeriod {
        context: DecodeContext {
            stream_id: C.into(),
            epoch: K.into(),
            mapping_generation: 1,
            generation: 1,
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
    host.feed()
        .publish(p.clone(), vec![Some("native-row".into())], b);
    consume(&host.inner).unwrap();
    p
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
