// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
};
const SERVER: &str = "00000000-0000-4000-8000-000000000001";
const WORKSPACE: &str = "00000000-0000-4000-8000-000000000002";
const EPOCH: &str = "00000000-0000-4000-8000-000000000003";
const TOKEN: &str = "synthetic-local-connection";
type Reply = Box<dyn FnOnce(&str) -> String + Send>;
fn body(name: &str) -> Value {
    let corpus: Value =
        serde_json::from_slice(include_bytes!("../../../fixtures/bearing-swift-v1.json")).unwrap();
    corpus[name].clone()
}
fn binding() -> Value {
    json!({"workspace_generation":WORKSPACE,"worker_generation":7,"settings_revision":11,"identity_epoch":EPOCH,"identity_revision":3,"operating_module":"normal_logging"})
}
fn snapshot(grid: &str, revision: u64) -> Value {
    let mut v = crate::tests::snapshot();
    v["server_instance"] = json!(SERVER);
    v["workspace_id"] = json!(WORKSPACE);
    v["revision"] = json!(revision);
    v["max_age_ms"] = json!(3000);
    v["station_identity"] = json!({"schema_version":1,"source":"saved_station_settings","state":"current","mycall":"OM0RX","mygrid":grid,"binding":binding(),"transmit_authority":false});
    v
}
fn envelope(name: &str) -> Value {
    json!({"protocol_version":1,"server_instance":SERVER,"workspace_id":WORKSPACE,"worker_generation":7,"station_identity_binding":binding(),"generation":WORKSPACE,"generated_at_unix_ms":now_ms().unwrap(),"max_age_ms":5000,"bearing":body(name)})
}
fn wire(status: u16, value: Value) -> String {
    wire_body(status, value.to_string())
}
fn wire_body(status: u16, body: String) -> String {
    format!("HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len())
}
fn reply(name: &'static str) -> Reply {
    Box::new(move |_| {
        wire(
            if name == "historical_grid" || name == "cty_approximation" {
                200
            } else {
                503
            },
            envelope(name),
        )
    })
}
fn serve(
    replies: Vec<Reply>,
    capability: bool,
    grid: &str,
) -> (BridgeClient, thread::JoinHandle<Vec<String>>) {
    serve_initial(replies, capability, || snapshot(grid, 5))
}
fn serve_initial(
    mut replies: Vec<Reply>,
    capability: bool,
    first: impl FnOnce() -> Value,
) -> (BridgeClient, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let mut hello = crate::tests::hello();
    hello["server_instance"] = json!(SERVER);
    hello["workspace_id"] = json!(WORKSPACE);
    if capability {
        hello["capabilities"] = json!([
            "snapshot.read",
            "bearing.read",
            "audio.rx",
            "logbook.contact.read"
        ]);
    }
    let first = first();
    replies.insert(0, Box::new(move |_| wire(200, first)));
    replies.insert(0, Box::new(move |_| wire(200, hello)));
    let server = thread::spawn(move || {
        let mut requests = Vec::new();
        for reply in replies {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((s, _)) => break s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < deadline, "expected request absent");
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            let mut byte = [0];
            while !bytes.ends_with(b"\r\n\r\n") {
                assert!(bytes.len() < 16384);
                stream.read_exact(&mut byte).unwrap();
                bytes.push(byte[0]);
            }
            let request = String::from_utf8(bytes).unwrap();
            let response = reply(&request);
            requests.push(request);
            let _ = stream.write_all(response.as_bytes());
        }
        requests
    });
    let mut bridge = BridgeClient::new(&endpoint, &crate::tests::token()).unwrap();
    bridge.connect().unwrap();
    bridge.poll().unwrap();
    (bridge, server)
}
fn grid() -> String {
    body("historical_grid")["origin"]["grid"]
        .as_str()
        .unwrap()
        .into()
}
fn capture(bridge: &BridgeClient, client: &BearingClient) -> BearingReadRequest {
    client
        .fence()
        .capture(TOKEN, "VK8RX/P", bridge.bearing_context().unwrap())
        .unwrap()
}

#[test]
fn actual_swift_bodies_over_authenticated_get_keep_unavailable_distinct() {
    for name in [
        "historical_grid",
        "cty_approximation",
        "coincident",
        "reference_unavailable",
    ] {
        let value = body(name);
        let (bridge, server) = serve(
            vec![reply(name)],
            true,
            value["origin"]["grid"].as_str().unwrap(),
        );
        let mut client = bridge.bearing_handle(TOKEN).unwrap();
        let requested = value["requested_callsign"].as_str().unwrap();
        let request = client
            .fence()
            .capture(TOKEN, requested, bridge.bearing_context().unwrap())
            .unwrap();
        let lease = client.read(&request).unwrap();
        assert_eq!(
            lease.bearing().unwrap().status,
            if name == "historical_grid" || name == "cty_approximation" {
                Status::Available
            } else {
                Status::Unavailable
            }
        );
        assert!(lease.remaining_valid_ms() > 0 && lease.remaining_valid_ms() <= 3000);
        let serialized = serde_json::to_value(&lease).unwrap();
        assert_eq!(
            serialized["payload"]["bearing"]["target"]["contact_started_at_unix_ms"],
            value["target"]["contact_started_at_unix_ms"]
        );
        assert_eq!(serialized["payload"]["station_identity_binding"], binding());
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        let request = &requests[2];
        assert!(request.starts_with(&format!(
            "GET /xlog/v1/bearing?callsign={} HTTP/1.1\r\n",
            requested.replace('/', "%2F")
        )));
        assert!(request
            .to_lowercase()
            .contains("authorization: bearer synthetic-pairing-marker-"));
        assert!(!request.contains(TOKEN));
        assert!(!request.to_lowercase().contains("origin:"));
    }
}
#[test]
fn capability_missing_snapshot_and_missing_grid_do_not_dispatch() {
    let (bridge, server) = serve(vec![], false, &grid());
    assert!(matches!(
        bridge.bearing_handle(TOKEN),
        Err(Error::Capability)
    ));
    assert!(matches!(bridge.bearing_context(), Err(Error::Capability)));
    server.join().unwrap();
    let (mut bridge, server) = serve(vec![], true, &grid());
    bridge.observation = None;
    assert!(bridge.bearing_context().is_err());
    server.join().unwrap();
    let (mut bridge, server) = serve(vec![], true, &grid());
    bridge
        .observation
        .as_mut()
        .unwrap()
        .snapshot
        .station_identity
        .as_mut()
        .unwrap()
        .mygrid = None;
    assert!(bridge.bearing_context().is_err());
    server.join().unwrap();
}
#[test]
fn incomplete_saved_identity_with_real_grid_is_passive_admission() {
    let mut v = snapshot(&grid(), 6);
    v["station_identity"]["state"] = json!("incomplete");
    v["station_identity"]["mycall"] = Value::Null;
    let (mut bridge, server) = serve(vec![Box::new(move |_| wire(200, v))], true, &grid());
    bridge.poll().unwrap();
    assert!(bridge.bearing_context().is_ok());
    server.join().unwrap();
}
#[test]
fn full_six_field_binding_and_nine_key_envelope_are_exact() {
    let (bridge, server) = serve(vec![], true, &grid());
    let client = bridge.bearing_handle(TOKEN).unwrap();
    let request = capture(&bridge, &client);
    let original = envelope("historical_grid");
    for group in ["", "station_identity_binding"] {
        let object = if group.is_empty() {
            original.as_object().unwrap()
        } else {
            original[group].as_object().unwrap()
        };
        for key in object.keys() {
            for replace_null in [false, true] {
                let mut v = original.clone();
                let map = if group.is_empty() {
                    v.as_object_mut().unwrap()
                } else {
                    v[group].as_object_mut().unwrap()
                };
                if replace_null {
                    map.insert(key.clone(), Value::Null);
                } else {
                    map.remove(key);
                }
                assert!(
                    serde_json::from_value::<Envelope>(v).is_err(),
                    "{group}/{key}"
                );
            }
        }
        let mut v = original.clone();
        if group.is_empty() {
            v["extra"] = json!(1)
        } else {
            v[group]["extra"] = json!(1)
        };
        assert!(serde_json::from_value::<Envelope>(v).is_err());
    }
    let duplicate = original.to_string().replace(
        "\"worker_generation\":7",
        "\"worker_generation\":7,\"worker_generation\":7",
    );
    assert!(serde_json::from_str::<Envelope>(&duplicate).is_err());
    for field in [
        "settings_revision",
        "identity_revision",
        "worker_generation",
    ] {
        let mut v = original.clone();
        v["station_identity_binding"][field] = json!(SAFE + 1);
        let e: Envelope = serde_json::from_value(v).unwrap();
        assert!(e
            .validate(&client.hello, &request, now_ms().unwrap(), 200)
            .is_err());
    }
    for id in [
        "00000000-0000-0000-0000-000000000000",
        "ABCDEF12-3456-4ABC-8DEF-123456ABCDEF",
        "00000000000040008000000000000003",
    ] {
        let mut v = original.clone();
        v["station_identity_binding"]["identity_epoch"] = json!(id);
        let e: Envelope = serde_json::from_value(v).unwrap();
        assert!(e
            .validate(&client.hello, &request, now_ms().unwrap(), 200)
            .is_err());
    }
    server.join().unwrap();
}
#[test]
fn foreign_call_origin_and_identity_never_create_a_lease() {
    for (group, key, value) in [
        ("", "server_instance", json!(EPOCH)),
        ("", "workspace_id", json!(EPOCH)),
        ("", "generation", json!(EPOCH)),
        ("", "worker_generation", json!(8)),
        ("station_identity_binding", "settings_revision", json!(12)),
        ("bearing", "requested_callsign", json!("VK8RX")),
    ] {
        let (bridge, server) = serve(vec![], true, &grid());
        let client = bridge.bearing_handle(TOKEN).unwrap();
        let request = capture(&bridge, &client);
        let mut v = envelope("historical_grid");
        if group.is_empty() {
            v[key] = value
        } else {
            v[group][key] = value
        };
        let e: Envelope = serde_json::from_value(v).unwrap();
        assert!(e
            .validate(&client.hello, &request, now_ms().unwrap(), 200)
            .is_err());
        server.join().unwrap();
    }
    let (bridge, server) = serve(vec![], true, &grid());
    let client = bridge.bearing_handle(TOKEN).unwrap();
    let request = capture(&bridge, &client);
    let mut e: Envelope = serde_json::from_value(envelope("historical_grid")).unwrap();
    e.bearing.origin.as_mut().unwrap().grid = "AA00".into();
    assert!(e
        .validate(&client.hello, &request, now_ms().unwrap(), 200)
        .is_err());
    server.join().unwrap();
}
#[test]
fn plain_503_is_stable_error_not_typed_unavailable_or_empty_geometry() {
    let (bridge, server) = serve(
        vec![
            reply("historical_grid"),
            Box::new(|_| wire(503, json!({"error":"bearing_busy"}))),
            Box::new(|_| wire(503, json!({"error":"Bad code"}))),
        ],
        true,
        &grid(),
    );
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let lease = client.read(&capture(&bridge, &client)).unwrap();
    assert!(
        matches!(client.read(&capture(&bridge,&client)),Err(BearingReadError::Unavailable(s))if s=="bearing_busy")
    );
    assert!(lease.bearing().is_err());
    assert!(matches!(
        client.read(&capture(&bridge, &client)),
        Err(BearingReadError::Bridge(Error::Invalid))
    ));
    server.join().unwrap();
}
#[test]
fn typed_status_and_body_must_agree() {
    for (name, status) in [("historical_grid", 503), ("reference_unavailable", 200)] {
        let (bridge, server) = serve(vec![], true, body(name)["origin"]["grid"].as_str().unwrap());
        let client = bridge.bearing_handle(TOKEN).unwrap();
        let request = capture(&bridge, &client);
        let e: Envelope = serde_json::from_value(envelope(name)).unwrap();
        assert!(e
            .validate(&client.hello, &request, now_ms().unwrap(), status)
            .is_err());
        server.join().unwrap();
    }
}
#[test]
fn malformed_content_oversize_duplicate_and_redirect_are_not_followed() {
    let replies: Vec<Reply> = vec![
        Box::new(|_| wire_body(200, " ".repeat(MAX_ENVELOPE_BYTES + 1))),
        Box::new(|_| {
            wire(200, envelope("historical_grid")).replace("application/json", "text/plain")
        }),
        Box::new(|_| {
            wire_body(
                200,
                envelope("historical_grid").to_string().replace(
                    "\"max_age_ms\":5000",
                    "\"max_age_ms\":5000,\"max_age_ms\":5000",
                ),
            )
        }),
        Box::new(|_| {
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
        }),
    ];
    let (bridge, server) = serve(replies, true, &grid());
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    for _ in 0..4 {
        assert!(client.read(&capture(&bridge, &client)).is_err());
    }
    assert_eq!(server.join().unwrap().len(), 6);
}
#[test]
fn original_snapshot_deadline_bounds_context_and_delayed_reply() {
    let (started_tx, started_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();
    let (bridge, server) = serve(
        vec![Box::new(move |_| {
            started_tx.send(()).unwrap();
            go_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            wire(200, envelope("historical_grid"))
        })],
        true,
        &grid(),
    );
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let mut context = bridge.bearing_context().unwrap();
    context.deadline = Instant::now() + Duration::from_millis(100);
    let request = client.fence().capture(TOKEN, "VK8RX/P", context).unwrap();
    let deadline = request.context.deadline;
    let job = thread::spawn(move || client.read(&request));
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    thread::sleep(deadline.saturating_duration_since(Instant::now()) + Duration::from_millis(2));
    go_tx.send(()).unwrap();
    assert!(job.join().unwrap().is_err());
    server.join().unwrap();
}
#[test]
fn fresh_same_identity_poll_never_renews_captured_deadline() {
    let mut initial = snapshot(&grid(), 5);
    initial["generated_at_unix_ms"] =
        json!(initial["generated_at_unix_ms"].as_u64().unwrap() - 1000);
    let (mut bridge, server) = serve_initial(
        vec![
            Box::new(|_| wire(200, snapshot(&grid(), 6))),
            reply("historical_grid"),
        ],
        true,
        move || initial,
    );
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let request = capture(&bridge, &client);
    let deadline = request.context.deadline;
    bridge.poll().unwrap();
    assert!(bridge.bearing_context().unwrap().deadline >= deadline);
    let lease = client.read(&request).unwrap();
    assert!(lease.expires <= deadline);
    assert!(lease.check_at(Ok(lease.started_wall), deadline).is_err());
    assert!(lease.bearing().is_err());
    server.join().unwrap();
}
#[test]
fn binding_grid_worker_and_missing_identity_retire_existing_contexts() {
    for variant in 0..5 {
        let mut next = snapshot(&grid(), 6);
        match variant {
            0 => next["station_identity"]["mygrid"] = json!("AA00"),
            1 => next["station_identity"]["binding"]["worker_generation"] = json!(8),
            2 => next["station_identity"]["binding"]["settings_revision"] = json!(12),
            3 => next["station_identity"] = Value::Null,
            _ => {
                next["station_identity"]["binding"]["identity_epoch"] =
                    json!("ABCDEF12-3456-4ABC-8DEF-123456ABCDEF")
            }
        };
        let (mut bridge, server) = serve(vec![Box::new(move |_| wire(200, next))], true, &grid());
        let context = bridge.bearing_context().unwrap();
        bridge.poll().unwrap();
        assert!(context.check().is_err());
        server.join().unwrap();
    }
}
#[test]
fn poll_failure_disconnect_and_drop_retire_without_bearing_lock() {
    let (mut bridge, server) = serve(
        vec![Box::new(|_| wire(503, json!({"error":"busy"})))],
        true,
        &grid(),
    );
    let context = bridge.bearing_context().unwrap();
    let client = bridge.bearing_handle(TOKEN).unwrap();
    assert!(bridge.poll().is_err());
    assert!(context.check().is_err());
    assert!(client.active().is_err());
    server.join().unwrap();
    let (bridge, server) = serve(vec![], true, &grid());
    let context = bridge.bearing_context().unwrap();
    drop(bridge);
    assert!(context.check().is_err());
    server.join().unwrap();
}
#[test]
fn capture_aba_invalid_attempt_and_foreign_token_retire_without_dispatch() {
    let (bridge, server) = serve(vec![reply("historical_grid")], true, &grid());
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let first = capture(&bridge, &client);
    let middle = client
        .fence()
        .capture(TOKEN, "W1AW", bridge.bearing_context().unwrap())
        .unwrap();
    let last = capture(&bridge, &client);
    assert!(first.check().is_err());
    assert!(middle.check().is_err());
    let lease = client.read(&last).unwrap();
    assert!(client.read(&first).is_err());
    assert!(lease.bearing().is_ok());
    assert!(client
        .fence()
        .capture(TOKEN, " VK8RX/P", bridge.bearing_context().unwrap())
        .is_err());
    assert!(lease.bearing().is_err());
    assert!(client
        .fence()
        .capture("wrong", "VK8RX/P", bridge.bearing_context().unwrap())
        .is_err());
    server.join().unwrap();
}
#[test]
fn expired_queued_request_and_sequence_overflow_make_no_http() {
    let (bridge, server) = serve(vec![], true, &grid());
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let mut request = capture(&bridge, &client);
    request.context.deadline = Instant::now();
    assert!(client.read(&request).is_err());
    client.fence.0.requests.lock().unwrap().sequence = u64::MAX;
    assert!(client
        .fence()
        .capture(TOKEN, "VK8RX/P", bridge.bearing_context().unwrap())
        .is_err());
    assert!(client.active().is_err());
    server.join().unwrap();
}
#[test]
fn stalled_http_new_capture_retires_late_result_without_blocking_fence() {
    let (start_tx, start_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel();
    let (bridge, server) = serve(
        vec![Box::new(move |_| {
            start_tx.send(()).unwrap();
            go_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            wire(200, envelope("historical_grid"))
        })],
        true,
        &grid(),
    );
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let fence = client.fence();
    let request = capture(&bridge, &client);
    let job = thread::spawn(move || client.read(&request));
    start_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let newer = fence
        .capture(TOKEN, "W1AW", bridge.bearing_context().unwrap())
        .unwrap();
    assert!(newer.check().is_ok());
    go_tx.send(()).unwrap();
    assert!(job.join().unwrap().is_err());
    server.join().unwrap();
}
#[test]
fn repeated_stamp_cannot_extend_and_revision_rollback_retires_facts() {
    let (bridge, server) = serve(vec![], true, &grid());
    let client = bridge.bearing_handle(TOKEN).unwrap();
    let request = capture(&bridge, &client);
    let now = request.started;
    let old = now + Duration::from_millis(300);
    assert_eq!(
        deadline(&request, 1000, 1000, now, Some((1000, old))).unwrap(),
        old
    );
    assert!(deadline(&request, 999, 1000, now, Some((1000, old))).is_err());
    assert!(deadline(&request, 1001, 1001, now, Some((1000, old))).unwrap() > old);
    server.join().unwrap();
    let (bridge, server) = serve(
        vec![reply("historical_grid"), reply("historical_grid")],
        true,
        &grid(),
    );
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let request = capture(&bridge, &client);
    let lease = client.read(&request).unwrap();
    let stamp = lease.payload.generated_at_unix_ms;
    // Pin a later synthetic observation stamp to exercise the actual highwater branch.
    client.last = Some((SAFE, lease.expires));
    assert!(client.read(&capture(&bridge, &client)).is_err());
    assert!(lease.bearing().is_err());
    assert!(stamp < SAFE);
    server.join().unwrap();
    let low: Reply = Box::new(|_| {
        let mut v = envelope("historical_grid");
        v["bearing"]["logbook_revision"] = json!(6);
        wire(200, v)
    });
    let (bridge, server) = serve(vec![reply("historical_grid"), low], true, &grid());
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let lease = client.read(&capture(&bridge, &client)).unwrap();
    assert!(matches!(
        client.read(&capture(&bridge, &client)),
        Err(BearingReadError::Bridge(Error::Revision))
    ));
    assert!(lease.bearing().is_err());
    server.join().unwrap();
}
#[test]
fn clock_discontinuity_latches_facts_and_never_rebounds() {
    let (bridge, server) = serve(vec![reply("historical_grid")], true, &grid());
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let lease = client.read(&capture(&bridge, &client)).unwrap();
    assert!(lease
        .check_at(Ok(lease.started_wall + 251), lease.started)
        .is_err());
    assert!(lease
        .check_at(Ok(lease.started_wall), lease.started)
        .is_err());
    assert!(serde_json::to_value(&lease).is_err());
    server.join().unwrap();
}
#[test]
fn unauthorized_retires_retained_handle_and_leases() {
    let (mut bridge, server) = serve(
        vec![
            reply("historical_grid"),
            Box::new(|_| wire(401, json!({"error":"unauthorized"}))),
        ],
        true,
        &grid(),
    );
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let lease = client.read(&capture(&bridge, &client)).unwrap();
    assert!(matches!(
        client.read(&capture(&bridge, &client)),
        Err(BearingReadError::Bridge(Error::Http(401)))
    ));
    assert!(lease.bearing().is_err());
    assert!(bridge.hello().is_none());
    bridge.disconnect();
    server.join().unwrap();
}

#[test]
fn reconnect_same_owner_ids_cannot_resurrect_old_context_or_handle() {
    let mut hello = crate::tests::hello();
    hello["server_instance"] = json!(SERVER);
    hello["workspace_id"] = json!(WORKSPACE);
    hello["capabilities"] = json!(["snapshot.read", "bearing.read"]);
    let next = snapshot(&grid(), 6);
    let (mut bridge, server) = serve(
        vec![
            reply("historical_grid"),
            Box::new(move |_| wire(200, hello)),
            Box::new(move |_| wire(200, next)),
        ],
        true,
        &grid(),
    );
    let mut old = bridge.bearing_handle(TOKEN).unwrap();
    let context = bridge.bearing_context().unwrap();
    let lease = old.read(&capture(&bridge, &old)).unwrap();
    bridge.connect().unwrap();
    bridge.poll().unwrap();
    assert!(context.check().is_err());
    assert!(old.active().is_err());
    assert!(lease.bearing().is_err());
    let fresh = bridge.bearing_handle(TOKEN).unwrap();
    assert!(capture(&bridge, &fresh).check().is_ok());
    server.join().unwrap();
}

#[test]
fn failed_capture_clock_retires_only_previous_request_and_wrong_token_does_not() {
    let (bridge, server) = serve(
        vec![reply("historical_grid"), reply("historical_grid")],
        true,
        &grid(),
    );
    let mut client = bridge.bearing_handle(TOKEN).unwrap();
    let old = client.read(&capture(&bridge, &client)).unwrap();
    assert!(client
        .fence()
        .capture_clock(TOKEN, "VK8RX/P", bridge.bearing_context().unwrap(), || {
            assert!(!old.request_live.load(Ordering::Acquire));
            Err(Error::Invalid)
        })
        .is_err());
    assert!(old.bearing().is_err());
    let fresh = client.read(&capture(&bridge, &client)).unwrap();
    assert!(client
        .fence()
        .capture_clock(
            "wrong",
            "VK8RX/P",
            bridge.bearing_context().unwrap(),
            || panic!("wrong token must not read clock")
        )
        .is_err());
    assert!(fresh.bearing().is_ok());
    assert!(old.check_at(Err(Error::Invalid), Instant::now()).is_err());
    assert!(fresh.bearing().is_ok());
    server.join().unwrap();
}
#[test]
fn clock_budget_checks_original_start_and_exact_250ms_boundary() {
    let (bridge, server) = serve(vec![], true, &grid());
    let client = bridge.bearing_handle(TOKEN).unwrap();
    let request = capture(&bridge, &client);
    assert!(request
        .clock(request.started_wall + 250, request.started)
        .is_ok());
    assert!(request
        .clock(request.started_wall + 251, request.started)
        .is_err());
    assert!(request
        .clock(request.started_wall - 1, request.started)
        .is_err());
    assert!(request
        .clock(
            request.started_wall + 5000,
            request.started + Duration::from_millis(5000)
        )
        .is_err());
    let mut e: Envelope = serde_json::from_value(envelope("historical_grid")).unwrap();
    e.generated_at_unix_ms = request.started_wall - 1;
    assert!(e
        .validate(&client.hello, &request, request.started_wall, 200)
        .is_err());
    e.generated_at_unix_ms = request.started_wall + 1;
    assert!(e
        .validate(&client.hello, &request, request.started_wall, 200)
        .is_err());
    server.join().unwrap();
}
#[test]
fn final_serializer_callback_rechecks_revocation_before_success() {
    struct Writer {
        bytes: Vec<u8>,
        fence: BearingReadFence,
        revoke: bool,
        armed: bool,
        triggered: bool,
    }
    impl Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes == b"remaining_valid_ms" {
                self.armed = true;
            }
            if self.armed && bytes == b"}" {
                self.triggered = true;
                if self.revoke {
                    self.fence.revoke();
                }
            }
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    for revoke in [false, true] {
        let (bridge, server) = serve(vec![reply("historical_grid")], true, &grid());
        let mut client = bridge.bearing_handle(TOKEN).unwrap();
        let lease = client.read(&capture(&bridge, &client)).unwrap();
        let mut writer = Writer {
            bytes: vec![],
            fence: client.fence(),
            revoke,
            armed: false,
            triggered: false,
        };
        let result = lease.serialize(&mut serde_json::Serializer::new(&mut writer));
        assert!(writer.triggered);
        assert_eq!(result.is_err(), revoke);
        assert_eq!(lease.bearing().is_err(), revoke);
        server.join().unwrap();
    }
}

#[test]
fn passive_binding_or_grid_change_preserves_main_audio_and_other_read_handles() {
    for variant in 0..3 {
        let changed: Reply = Box::new(move |_| {
            let mut v = envelope("historical_grid");
            match variant {
                0 => v["station_identity_binding"]["settings_revision"] = json!(12),
                1 => v["bearing"]["origin"]["grid"] = json!("AA00"),
                _ => {
                    v["worker_generation"] = json!(8);
                    v["station_identity_binding"]["worker_generation"] = json!(8);
                }
            }
            wire(200, v)
        });
        let audio_reply: Reply = Box::new(|request| {
            assert!(request.starts_with("GET /xlog/v1/audio?after=0 "));
            let mut v = crate::audio::tests::fixture(now_ms().unwrap());
            v["server_instance"] = json!(SERVER);
            v["workspace_id"] = json!(WORKSPACE);
            wire(200, v)
        });
        let refresh = snapshot(&grid(), 6);
        let (mut bridge, server) = serve(
            vec![
                reply("historical_grid"),
                changed,
                audio_reply,
                Box::new(move |_| wire(200, refresh)),
            ],
            true,
            &grid(),
        );
        let mut previous = bridge.bearing_handle(TOKEN).unwrap();
        let old = previous.read(&capture(&bridge, &previous)).unwrap();
        let old_context = bridge.bearing_context().unwrap();
        let mut audio = bridge.audio_handle().unwrap();
        let other_read = bridge.contact_detail_handle(TOKEN).unwrap();
        let mut client = bridge.bearing_handle(TOKEN).unwrap();
        assert!(matches!(
            client.read(&capture(&bridge, &client)),
            Err(BearingReadError::ContextChanged)
        ));
        assert!(old.bearing().is_err());
        assert!(old_context.check().is_err());
        assert!(bridge.hello().is_some());
        assert!(bridge.snapshot().is_ok());
        assert!(other_read
            .capture(TOKEN, "abcdef12-3456-4abc-8def-123456abcdef")
            .is_ok());
        assert!(audio.poll().unwrap().is_fresh());
        assert!(bridge.bearing_context().is_err());
        bridge.poll().unwrap();
        assert!(capture(&bridge, &client).check().is_ok());
        server.join().unwrap();
    }
}

#[test]
fn old_context_error_cannot_clear_replacement_or_aba_identity_authority() {
    let (bridge, server) = serve(vec![], true, &grid());
    let original = bridge.bearing_context().unwrap();
    let changed: Snapshot = serde_json::from_value(snapshot("AA00", 6)).unwrap();
    let restored: Snapshot = serde_json::from_value(snapshot(&grid(), 7)).unwrap();
    let mut authority = bridge.bearing_authority.lock().unwrap();
    authority.observe(&changed);
    let replacement = authority.current.as_ref().unwrap().1.clone();
    authority.retire_if_current(&original.live);
    assert!(replacement.load(Ordering::Acquire));
    assert!(Arc::ptr_eq(
        &authority.current.as_ref().unwrap().1,
        &replacement
    ));
    authority.observe(&restored);
    let back_to_a = authority.current.as_ref().unwrap().1.clone();
    assert!(!Arc::ptr_eq(&back_to_a, &original.live));
    authority.retire_if_current(&original.live);
    assert!(back_to_a.load(Ordering::Acquire));
    assert!(Arc::ptr_eq(
        &authority.current.as_ref().unwrap().1,
        &back_to_a
    ));
    drop(authority);
    assert!(bridge.hello().is_some());
    assert!(bridge.snapshot().is_ok());
    server.join().unwrap();
}
