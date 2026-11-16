use super::*;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
};
const SERVER: &str = "11111111-1111-4111-8111-111111111111";
const WORKSPACE: &str = "22222222-2222-4222-8222-222222222222";
const CONTEXT: &str = "33333333-3333-4333-8333-333333333333";
const CLOCK: &str = "44444444-4444-4444-8444-444444444444";
const CLIENT: &str = "55555555-5555-4555-8555-555555555555";
const OP: &str = "66666666-6666-4666-8666-666666666666";
const EVENT: &str = "77777777-7777-4777-8777-777777777777";
const ACTION: &str = "88888888-8888-4888-8888-888888888888";
const OTHER: &str = "99999999-9999-4999-8999-999999999999";
fn binding() -> wire::Binding {
    wire::Binding {
        server_instance: SERVER.into(),
        workspace_id: WORKSPACE.into(),
        context_id: CONTEXT.into(),
        clock_epoch: CLOCK.into(),
    }
}
fn context() -> wire::Context {
    wire::Context {
        schema_version: 1,
        binding: binding(),
        ready: true,
        reason: None,
        local_enabled: true,
        client_enabled: true,
        identity: Some(wire::Identity {
            mycall: "OM0RX".into(),
            mygrid: "JN88AA".into(),
            worker_generation: 1,
            settings_revision: 1,
            identity_epoch: EVENT.into(),
            identity_revision: 1,
        }),
        route: Some(wire::OutputRoute {
            radio_id: "radio1".into(),
            receiver_id: "A".into(),
            transmit_slice: "A".into(),
            device_id: "synthetic-output".into(),
            route_epoch: OP.into(),
            sample_rate_hz: 48000,
            channels: 2,
            selected_channels: vec![0],
            frequency_hz: 14_074_000,
            radio_mode: "DIGU".into(),
        }),
        generated_at_unix_ms: now_ms().unwrap(),
        max_age_ms: 1000,
        heartbeat_interval_ms: 200,
        heartbeat_timeout_ms: 1000,
        max_prepare_bytes: MAX_PREPARE,
        min_offset_hz: 100,
        max_offset_hz: 3000,
        max_abs_sample_f32_bits: 0.95f32.to_bits(),
    }
}
fn hello() -> Hello {
    Hello {
        application: "xLog".into(),
        protocol_version: 1,
        server_instance: SERVER.into(),
        workspace_id: WORKSPACE.into(),
        capabilities: vec!["snapshot.read".into(), "digital.tx".into()],
    }
}
fn bridge(endpoint: &str) -> BridgeClient {
    // Authentication and HTTP stack are actual SDK; hello admission is isolated from unrelated snapshot APIs.
    let mut b = BridgeClient::new(endpoint, "abcdefghijklmnopqrstuvwxyz0123456789").unwrap();
    b.hello = Some(hello());
    b.audio_session.store(true, Ordering::Release);
    b
}
fn local() -> (BridgeClient, TxClient, ContextLease) {
    let b = bridge("http://127.0.0.1:1");
    let c = b.tx_handle(CLIENT).unwrap();
    let l = ContextLease {
        context: context(),
        expires: Instant::now() + Duration::from_secs(1),
        live: Arc::new(AtomicBool::new(true)),
        session: b.audio_session.clone(),
        issuer: c.issuer.clone(),
    };
    (b, c, l)
}
fn tune(c: &TxClient, l: &ContextLease) -> PreparedOperation {
    c.build_tune(
        l,
        wire::Tune {
            binding: l.context.binding.clone(),
            client_id: CLIENT.into(),
            operation_id: OP.into(),
            event_id: EVENT.into(),
            tone_hz_f32_bits: 1500f32.to_bits(),
            duration_ms: 100,
        },
    )
    .unwrap()
}
fn pcm(c: &TxClient, l: &ContextLease, mode: wire::Mode) -> PreparedOperation {
    let mut bytes = vec![0; mode.frames() as usize * 4];
    bytes[1024..1028].copy_from_slice(&0.25f32.to_le_bytes());
    let message = "CQ OM0RX JN88";
    let mut hash = Sha256::new();
    hash.update(b"xnexus-final-period-v2\0");
    hash.update([if mode == wire::Mode::FT8 { 8 } else { 4 }]);
    hash.update(48000u32.to_le_bytes());
    hash.update(1u16.to_le_bytes());
    hash.update(b"f32le\0");
    hash.update(1500f32.to_bits().to_le_bytes());
    hash.update((message.len() as u32).to_le_bytes());
    hash.update(message.as_bytes());
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(&bytes);
    let d = wire::Prepare {
        schema_version: 2,
        binding: l.context.binding.clone(),
        client_id: CLIENT.into(),
        operation_id: OP.into(),
        event_id: EVENT.into(),
        slot_start_unix_us: mode.period_us() * 100,
        latest_start_unix_us: mode.period_us() * 100 + 50_000,
        manifest: wire::Manifest {
            schema_version: 2,
            mode,
            message_form: "wsjt77".into(),
            message: message.into(),
            format: wire::Format {
                sample_rate_hz: 48000,
                channels: 1,
                encoding: "f32le".into(),
            },
            offset_hz_f32_bits: 1500f32.to_bits(),
            frames: mode.frames(),
            waveform_frames: 257,
            filter_id: "kaiser-513-fc6000-beta10-x4-candidate1".into(),
            coefficients_sha256: "b3b266e2346da0272cdb1910943a6ba8020d17a56e55a7b0eb0f12cd2c73c764"
                .into(),
            group_delay_frames: 256,
            support_start_frame: 256,
            gain_f32_bits: 0.5f32.to_bits(),
            pcm_byte_count: bytes.len() as u32,
            period_duration_us: mode.period_us(),
            pcm_sha256: validate::digest(&bytes).1,
            content_sha256: format!("{:x}", hash.finalize()),
        },
    };
    c.build_prepared(l, d, &bytes).unwrap()
}
fn receipt(
    op: &PreparedOperation,
    phase: wire::Phase,
    outcome: wire::Outcome,
    start: Option<u64>,
    release: Option<u64>,
) -> wire::Receipt {
    let k = op.key();
    wire::Receipt {
        schema_version: 1,
        server_instance: k.binding.server_instance.clone(),
        workspace_id: k.binding.workspace_id.clone(),
        operation_id: k.operation_id.clone(),
        body_sha256: k.body_sha256.clone(),
        context_id: k.binding.context_id.clone(),
        clock_epoch: k.binding.clock_epoch.clone(),
        event_id: k.origin_event_id.clone(),
        phase,
        outcome,
        actual_start_unix_us: start,
        release_observed_unix_us: release,
        reason: None,
    }
}
struct Request {
    path: String,
    body: Vec<u8>,
}
fn request(stream: &mut TcpStream) -> Request {
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    let mut header = Vec::new();
    let mut byte = [0];
    while !header.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        header.push(byte[0]);
        assert!(header.len() < 8192);
    }
    let header = String::from_utf8(header).unwrap();
    assert!(header
        .to_ascii_lowercase()
        .contains("authorization: bearer abcdefghijklmnopqrstuvwxyz0123456789"));
    let len = header
        .lines()
        .find_map(|l| {
            l.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .map(|s| s.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    assert!(len <= MAX_PREPARE);
    let mut body = vec![0; len];
    stream.read_exact(&mut body).unwrap();
    Request {
        path: header
            .lines()
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .into(),
        body,
    }
}
fn reply(stream: &mut TcpStream, status: u16, body: &[u8]) {
    write!(stream,"HTTP/1.1 {status} response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len()).unwrap();
    stream.write_all(body).unwrap();
}
fn server<F>(count: usize, f: F) -> (String, thread::JoinHandle<()>)
where
    F: Fn(Request, &mut TcpStream) + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let f = Arc::new(f);
    let join = thread::spawn(move || {
        let mut children = Vec::new();
        for _ in 0..count {
            let (mut s, _) = listener.accept().unwrap();
            let f = f.clone();
            children.push(thread::spawn(move || {
                let r = request(&mut s);
                f(r, &mut s);
            }));
        }
        for child in children {
            child.join().unwrap();
        }
    });
    (endpoint, join)
}
fn json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

#[test]
fn capability_and_canonical_client_id_are_required() {
    let mut b = bridge("http://127.0.0.1:1");
    b.hello.as_mut().unwrap().capabilities.clear();
    assert!(matches!(b.tx_handle(CLIENT), Err(Error::Capability)));
    b.hello = Some(hello());
    assert!(matches!(
        b.tx_handle("00000000-0000-0000-0000-000000000000"),
        Err(Error::Invalid)
    ));
    assert!(b.tx_handle(CLIENT).is_ok());
}
#[test]
fn context_required_nulls_unknown_fields_and_semantics_fail_closed() {
    let mut v = serde_json::to_value(context()).unwrap();
    v.as_object_mut().unwrap().remove("reason");
    assert!(serde_json::from_value::<wire::Context>(v).is_err());
    let mut c = context();
    c.ready = false;
    assert_eq!(
        validate::context(&c, &hello(), now_ms().unwrap()),
        Err(Error::Invalid)
    );
    c.reason = Some("Local disabled".into());
    assert!(validate::context(&c, &hello(), now_ms().unwrap()).is_ok());
    c.route.as_mut().unwrap().selected_channels = vec![0, 0];
    assert_eq!(
        validate::context(&c, &hello(), now_ms().unwrap()),
        Err(Error::Invalid)
    );
}
#[test]
fn first_enable_does_not_require_existing_client_latch() {
    let (b, c, mut l) = local();
    l.context.client_enabled = false;
    assert!(c.admission(&l, false).is_ok());
    assert_eq!(c.admission(&l, true), Err(Error::Stale));
    drop(b);
}
#[test]
fn original_deadline_and_revoked_session_cannot_be_refreshed_locally() {
    let (b, c, mut l) = local();
    let op = tune(&c, &l);
    l.expires = Instant::now() - Duration::from_millis(1);
    assert!(!l.is_fresh());
    assert_eq!(c.admission(&l, true), Err(Error::Stale));
    drop(b);
    assert!(!op.0.context.is_fresh());
    assert!(matches!(c.tune(&op), Err(Error::Stale)));
}
#[test]
fn repeated_context_timestamp_never_renews_original_expiry() {
    let value = json(&context());
    let (url, j) = server(2, move |_, s| reply(s, 200, &value));
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    let a = c.context().unwrap();
    let fresh = c.context().unwrap();
    assert!(fresh.expires <= a.expires);
    assert!(Arc::ptr_eq(&a.live, &fresh.live));
    j.join().unwrap();
}
#[test]
fn changed_binding_retires_old_admission_but_not_main_session() {
    let n = AtomicBool::new(false);
    let (url, j) = server(2, move |_, s| {
        let mut c = context();
        if n.swap(true, Ordering::AcqRel) {
            c.binding.context_id = OTHER.into();
        }
        reply(s, 200, &json(&c));
    });
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    let a = c.context().unwrap();
    let z = c.context().unwrap();
    assert!(!a.is_fresh());
    assert!(z.is_fresh());
    assert!(b.audio_session.load(Ordering::Acquire));
    j.join().unwrap();
}
#[test]
fn failed_context_retires_permission_and_foreign_owner_retires_session() {
    let n = AtomicBool::new(false);
    let (url, j) = server(2, move |_, s| {
        let mut c = context();
        if n.swap(true, Ordering::AcqRel) {
            c.binding.workspace_id = OTHER.into();
        }
        reply(s, 200, &json(&c));
    });
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    let a = c.context().unwrap();
    assert!(matches!(c.context(), Err(Error::IdentityChanged)));
    assert!(!a.is_fresh());
    assert!(!b.audio_session.load(Ordering::Acquire));
    j.join().unwrap();
}
#[test]
fn binary_body_hash_binds_descriptor_prefix_and_exact_pcm_for_both_modes() {
    for mode in [wire::Mode::FT8, wire::Mode::FT4] {
        let (_b, c, l) = local();
        let op = pcm(&c, &l, mode);
        let length = u32::from_le_bytes(op.body()[..4].try_into().unwrap()) as usize;
        let d: wire::Prepare = serde_json::from_slice(&op.body()[4..4 + length]).unwrap();
        assert_eq!(&d, op.descriptor().unwrap());
        assert_eq!(validate::digest(op.body()).0, op.key().body_digest);
        assert_eq!(op.body().len(), 4 + length + mode.frames() as usize * 4);
        assert_eq!(
            validate::digest(&op.body()[4 + length..]).1,
            d.manifest.pcm_sha256
        );
    }
}

#[test]
fn context_generated_after_dispatch_uses_receipt_clock_but_original_deadline() {
    let (url, j) = server(1, |_, s| {
        thread::sleep(Duration::from_millis(80));
        reply(s, 200, &json(&context()));
    });
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    let started = Instant::now();
    let lease = c.context().unwrap();
    assert!(lease.is_fresh());
    assert!(lease.expires <= started + Duration::from_millis(1005));
    assert!(lease.remaining_valid_ms() < 950);
    j.join().unwrap();
}

#[test]
fn delayed_old_context_cannot_receive_a_new_budget() {
    let mut old = context();
    old.generated_at_unix_ms -= 950;
    let (url, j) = server(1, move |_, s| {
        thread::sleep(Duration::from_millis(80));
        reply(s, 200, &json(&old));
    });
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    assert!(matches!(c.context(), Err(Error::Stale)));
    j.join().unwrap();
}
#[test]
fn pcm_nonfinite_negative_zero_padding_digest_and_provenance_are_rejected() {
    let (_b, c, l) = local();
    let op = pcm(&c, &l, wire::Mode::FT4);
    let d = op.descriptor().unwrap().clone();
    let n = u32::from_le_bytes(op.body()[..4].try_into().unwrap()) as usize;
    let base = &op.body()[4 + n..];
    for bits in [f32::NAN.to_bits(), (-0.0f32).to_bits(), 1.0f32.to_bits()] {
        let mut bytes = base.to_vec();
        bytes[..4].copy_from_slice(&bits.to_le_bytes());
        assert!(matches!(
            c.build_prepared(&l, d.clone(), &bytes),
            Err(Error::Invalid)
        ));
    }
    let mut changed = d.clone();
    changed.manifest.group_delay_frames = 0;
    assert!(matches!(
        c.build_prepared(&l, changed, base),
        Err(Error::Invalid)
    ));
    let mut changed = d;
    changed.manifest.pcm_sha256 = "0".repeat(64);
    assert!(matches!(
        c.build_prepared(&l, changed, base),
        Err(Error::Invalid)
    ));
}
#[test]
fn tune_is_bounded_and_hashes_exact_json_bytes() {
    let (_b, c, l) = local();
    let op = tune(&c, &l);
    assert_eq!(op.key().body_sha256, validate::digest(op.body()).1);
    let mut d: wire::Tune = serde_json::from_slice(op.body()).unwrap();
    d.duration_ms = 30_001;
    assert!(matches!(c.build_tune(&l, d.clone()), Err(Error::Invalid)));
    d.duration_ms = 100;
    d.tone_hz_f32_bits = f32::INFINITY.to_bits();
    assert!(matches!(c.build_tune(&l, d), Err(Error::Invalid)));
}
#[test]
fn malformed_receipt_never_creates_actual_start_proof() {
    let (_b, c, l) = local();
    let op = tune(&c, &l);
    let mut r = receipt(
        &op,
        wire::Phase::Prepared,
        wire::Outcome::NoStart,
        Some(100),
        None,
    );
    assert!(matches!(
        accept(&c.transport, &op, r.clone()),
        Err(Error::Invalid)
    ));
    assert!(op.cached_receipt().is_none());
    r.actual_start_unix_us = None;
    r.event_id = OTHER.into();
    assert!(matches!(
        accept(&c.transport, &op, r),
        Err(Error::IdentityChanged)
    ));
    assert!(op.cached_receipt().is_none());
}
#[test]
fn actual_start_and_release_are_both_retained_when_first_seen_terminal() {
    let (_b, c, l) = local();
    let op = tune(&c, &l);
    let r = receipt(
        &op,
        wire::Phase::Terminal,
        wire::Outcome::ReleaseObserved,
        Some(100),
        Some(200),
    );
    let p = accept(&c.transport, &op, r.clone()).unwrap();
    assert_eq!(p.receipt().actual_start_unix_us, Some(100));
    assert_eq!(p.receipt().release_observed_unix_us, Some(200));
    assert_eq!(accept(&c.transport, &op, r).unwrap().key(), op.key());
}
#[test]
fn proof_progression_never_erases_or_changes_start_and_final_outcomes() {
    let (_b, c, l) = local();
    let op = tune(&c, &l);
    accept(
        &c.transport,
        &op,
        receipt(
            &op,
            wire::Phase::Active,
            wire::Outcome::Uncertain,
            Some(100),
            None,
        ),
    )
    .unwrap();
    for bad in [
        receipt(
            &op,
            wire::Phase::Terminal,
            wire::Outcome::NoStart,
            None,
            None,
        ),
        receipt(
            &op,
            wire::Phase::Active,
            wire::Outcome::Uncertain,
            Some(101),
            None,
        ),
        receipt(
            &op,
            wire::Phase::Starting,
            wire::Outcome::Uncertain,
            None,
            None,
        ),
    ] {
        assert!(accept(&c.transport, &op, bad).is_err());
    }
    accept(
        &c.transport,
        &op,
        receipt(
            &op,
            wire::Phase::Terminal,
            wire::Outcome::ReleaseObserved,
            Some(100),
            Some(200),
        ),
    )
    .unwrap();
    assert!(accept(
        &c.transport,
        &op,
        receipt(
            &op,
            wire::Phase::Terminal,
            wire::Outcome::ReleaseObserved,
            Some(100),
            Some(201)
        )
    )
    .is_err());
}
#[test]
fn terminal_uncertainty_can_refine_release_but_not_no_start() {
    let (_b, c, l) = local();
    let op = tune(&c, &l);
    accept(
        &c.transport,
        &op,
        receipt(
            &op,
            wire::Phase::Terminal,
            wire::Outcome::Uncertain,
            None,
            None,
        ),
    )
    .unwrap();
    assert!(matches!(
        accept(
            &c.transport,
            &op,
            receipt(
                &op,
                wire::Phase::Terminal,
                wire::Outcome::NoStart,
                None,
                None
            )
        ),
        Err(Error::Revision)
    ));
    assert!(accept(
        &c.transport,
        &op,
        receipt(
            &op,
            wire::Phase::Terminal,
            wire::Outcome::ReleaseObserved,
            None,
            Some(200)
        )
    )
    .is_ok());
}
#[test]
fn prepared_start_requires_original_slot_window_not_client_wall_clock() {
    let (_b, c, l) = local();
    let op = pcm(&c, &l, wire::Mode::FT4);
    let (lo, hi) = op.key().slot_window.unwrap();
    assert!(accept(
        &c.transport,
        &op,
        receipt(
            &op,
            wire::Phase::Active,
            wire::Outcome::Uncertain,
            Some(lo - 1),
            None
        )
    )
    .is_err());
    assert!(accept(
        &c.transport,
        &op,
        receipt(
            &op,
            wire::Phase::Active,
            wire::Outcome::Uncertain,
            Some(hi + 1),
            None
        )
    )
    .is_err());
    assert!(accept(
        &c.transport,
        &op,
        receipt(
            &op,
            wire::Phase::Active,
            wire::Outcome::Uncertain,
            Some(lo),
            None
        )
    )
    .is_ok());
}
#[test]
fn sealed_operation_from_another_client_cannot_dispatch() {
    let (_b, c, l) = local();
    let op = tune(&c, &l);
    let (_other, other, _) = local();
    assert!(matches!(other.tune(&op), Err(Error::Invalid)));
    assert!(matches!(other.stop_handle(&op), Err(Error::Invalid)));
}
#[test]
fn stop_is_independent_while_prepare_blocked_and_late_ack_cannot_downgrade() {
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let release_rx = Mutex::new(release_rx);
    let stored = Arc::new(Mutex::new(None::<wire::Prepare>));
    let wire_stored = stored.clone();
    let (url, j) = server(3, move |r, s| match r.path.as_str() {
        "/xlog/v1/tx/context" => reply(s, 200, &json(&context())),
        "/xlog/v1/tx/prepare" => {
            let len = u32::from_le_bytes(r.body[..4].try_into().unwrap()) as usize;
            let d: wire::Prepare = serde_json::from_slice(&r.body[4..4 + len]).unwrap();
            *wire_stored.lock().unwrap() = Some(d.clone());
            entered_tx.send(()).unwrap();
            release_rx
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
            reply(
                s,
                200,
                &json(&wire::Receipt {
                    schema_version: 1,
                    server_instance: SERVER.into(),
                    workspace_id: WORKSPACE.into(),
                    operation_id: OP.into(),
                    body_sha256: validate::digest(&r.body).1,
                    context_id: CONTEXT.into(),
                    clock_epoch: CLOCK.into(),
                    event_id: EVENT.into(),
                    phase: wire::Phase::Prepared,
                    outcome: wire::Outcome::NoStart,
                    actual_start_unix_us: None,
                    release_observed_unix_us: None,
                    reason: None,
                }),
            );
        }
        "/xlog/v1/tx/stop" => {
            let o: wire::Operation = serde_json::from_slice(&r.body).unwrap();
            assert_eq!(o.event_id, ACTION);
            assert!(wire_stored.lock().unwrap().is_some());
            reply(
                s,
                200,
                &json(&wire::Receipt {
                    schema_version: 1,
                    server_instance: SERVER.into(),
                    workspace_id: WORKSPACE.into(),
                    operation_id: OP.into(),
                    body_sha256: o.body_sha256,
                    context_id: CONTEXT.into(),
                    clock_epoch: CLOCK.into(),
                    event_id: EVENT.into(),
                    phase: wire::Phase::Terminal,
                    outcome: wire::Outcome::NoStart,
                    actual_start_unix_us: None,
                    release_observed_unix_us: None,
                    reason: None,
                }),
            );
        }
        _ => panic!("unexpected path"),
    });
    let b = bridge(&url);
    let c = Arc::new(b.tx_handle(CLIENT).unwrap());
    let lease = c.context().unwrap();
    let op = pcm(&c, &lease, wire::Mode::FT4);
    let stop = c.stop_handle(&op).unwrap();
    let cc = c.clone();
    let oo = op.clone();
    let prepare = thread::spawn(move || cc.prepare(&oo));
    entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let proof = stop.stop(ACTION).unwrap();
    assert_eq!(proof.receipt().phase, wire::Phase::Terminal);
    release_tx.send(()).unwrap();
    assert!(matches!(
        prepare.join().unwrap(),
        Err(Error::OutcomeUnknown)
    ));
    assert_eq!(
        op.cached_receipt().unwrap().receipt().phase,
        wire::Phase::Terminal
    );
    j.join().unwrap();
}
#[test]
fn exact_stop_after_context_expiry_and_client_drop_does_not_retarget() {
    let (url, j) = server(1, |r, s| {
        assert_eq!(r.path, "/xlog/v1/tx/stop");
        let o: wire::Operation = serde_json::from_slice(&r.body).unwrap();
        assert_eq!(o.binding, binding());
        reply(
            s,
            200,
            &json(&wire::Receipt {
                schema_version: 1,
                server_instance: SERVER.into(),
                workspace_id: WORKSPACE.into(),
                operation_id: OP.into(),
                body_sha256: o.body_sha256,
                context_id: CONTEXT.into(),
                clock_epoch: CLOCK.into(),
                event_id: EVENT.into(),
                phase: wire::Phase::Terminal,
                outcome: wire::Outcome::NoStart,
                actual_start_unix_us: None,
                release_observed_unix_us: None,
                reason: None,
            }),
        );
    });
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    let lease = ContextLease {
        context: context(),
        expires: Instant::now() + Duration::from_secs(1),
        live: Arc::new(AtomicBool::new(true)),
        session: b.audio_session.clone(),
        issuer: c.issuer.clone(),
    };
    let op = tune(&c, &lease);
    let stop = c.stop_handle(&op).unwrap();
    lease.live.store(false, Ordering::Release);
    drop(c);
    // Ordinary BridgeClient retirement cannot suppress the retained release.
    drop(b);
    assert!(stop.stop(ACTION).is_ok());
    j.join().unwrap();
}

#[test]
fn stop_owner_auth_rejection_retires_only_the_release_transport() {
    let (url, j) = server(1, |r, s| {
        assert_eq!(r.path, "/xlog/v1/tx/stop");
        reply(s, 403, b"revoked");
    });
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    let lease = ContextLease {
        context: context(),
        expires: Instant::now() + Duration::from_secs(1),
        live: Arc::new(AtomicBool::new(true)),
        session: b.audio_session.clone(),
        issuer: c.issuer.clone(),
    };
    let op = tune(&c, &lease);
    let stop = c.stop_handle(&op).unwrap();
    assert!(matches!(stop.stop(ACTION), Err(Error::OutcomeUnknown)));
    assert!(!stop.transport.session.load(Ordering::Acquire));
    assert!(matches!(stop.stop(OTHER), Err(Error::OutcomeUnknown)));
    assert!(op.cached_receipt().is_none());
    j.join().unwrap();
}
#[test]
fn missing_status_is_unknown_and_never_overwrites_retained_evidence() {
    let (url, j) = server(1, |r, s| {
        assert_eq!(r.path, format!("/xlog/v1/tx/status?operation_id={OP}"));
        reply(
            s,
            200,
            &json(&wire::Status {
                schema_version: 1,
                server_instance: SERVER.into(),
                workspace_id: WORKSPACE.into(),
                operation_id: OP.into(),
                receipt: None,
            }),
        );
    });
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    let lease = ContextLease {
        context: context(),
        expires: Instant::now() + Duration::from_secs(1),
        live: Arc::new(AtomicBool::new(true)),
        session: b.audio_session.clone(),
        issuer: c.issuer.clone(),
    };
    let op = tune(&c, &lease);
    accept(
        &c.transport,
        &op,
        receipt(
            &op,
            wire::Phase::Active,
            wire::Outcome::Uncertain,
            Some(100),
            None,
        ),
    )
    .unwrap();
    assert!(c.status(&op).unwrap().is_none());
    assert_eq!(
        op.cached_receipt().unwrap().receipt().actual_start_unix_us,
        Some(100)
    );
    j.join().unwrap();
}
#[test]
fn dispatched_failure_is_uncertain_and_cannot_automatically_replay() {
    let (url, j) = server(1, |r, s| {
        assert_eq!(r.path, "/xlog/v1/tx/tune");
        reply(s, 503, b"busy");
    });
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    let lease = ContextLease {
        context: context(),
        expires: Instant::now() + Duration::from_secs(1),
        live: Arc::new(AtomicBool::new(true)),
        session: b.audio_session.clone(),
        issuer: c.issuer.clone(),
    };
    let op = tune(&c, &lease);
    assert!(matches!(c.tune(&op), Err(Error::OutcomeUnknown)));
    assert!(matches!(c.tune(&op), Err(Error::OutcomeUnknown)));
    assert!(op.cached_receipt().is_none());
    j.join().unwrap();
}
#[test]
fn auth_rejection_revokes_session_without_fabricating_a_release() {
    let (url, j) = server(1, |_, s| reply(s, 403, b"denied"));
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    assert!(matches!(c.context(), Err(Error::Disconnected)));
    assert!(!b.audio_session.load(Ordering::Acquire));
    j.join().unwrap();
}

#[test]
fn authenticated_tune_reply_seals_both_start_and_release_with_exact_body_identity() {
    let (url, j) = server(1, |r, s| {
        assert_eq!(r.path, "/xlog/v1/tx/tune");
        let tune: wire::Tune = serde_json::from_slice(&r.body).unwrap();
        reply(
            s,
            200,
            &json(&wire::Receipt {
                schema_version: 1,
                server_instance: SERVER.into(),
                workspace_id: WORKSPACE.into(),
                operation_id: tune.operation_id,
                body_sha256: validate::digest(&r.body).1,
                context_id: CONTEXT.into(),
                clock_epoch: CLOCK.into(),
                event_id: tune.event_id,
                phase: wire::Phase::Terminal,
                outcome: wire::Outcome::ReleaseObserved,
                actual_start_unix_us: Some(123),
                release_observed_unix_us: Some(456),
                reason: None,
            }),
        );
    });
    let b = bridge(&url);
    let c = b.tx_handle(CLIENT).unwrap();
    let lease = ContextLease {
        context: context(),
        expires: Instant::now() + Duration::from_secs(1),
        live: Arc::new(AtomicBool::new(true)),
        session: b.audio_session.clone(),
        issuer: c.issuer.clone(),
    };
    let op = tune(&c, &lease);
    let proof = c.tune(&op).unwrap();
    assert_eq!(proof.key(), op.key());
    assert_eq!(proof.receipt().event_id, EVENT);
    assert_eq!(proof.receipt().actual_start_unix_us, Some(123));
    assert_eq!(proof.receipt().release_observed_unix_us, Some(456));
    assert!(matches!(c.tune(&op), Err(Error::OutcomeUnknown)));
    assert_eq!(op.cached_receipt().unwrap().receipt(), proof.receipt());
    j.join().unwrap();
}
