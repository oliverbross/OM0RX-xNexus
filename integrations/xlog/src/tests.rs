// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
use super::*;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn hello() -> Value {
    json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace","capabilities":["snapshot.read"]})
}
fn snapshot() -> Value {
    json!({"protocol_version":1,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace","revision":5,"generated_at_unix_ms":now_ms().unwrap(),"max_age_ms":10000,
        "station":{"connected":true,"radio_id":"fake-radio","frequency_hz":14074000,"mode":"USB","transmitting":false},
        "rotor":{"connected":true,"rotor_id":"fake-rotor","azimuth_deg":123.0,"moving":false},
        "spots":[{"id":"synthetic-1","callsign":"W1AW","frequency_hz":14074000,"mode":"FT8","observed_at_unix_ms":now_ms().unwrap(),"source":"synthetic-cluster","need":{"status":"needed","score":100,"reasons":["New entity on 20m"]}}]})
}
fn json_response(body: Value) -> String {
    let body = body.to_string();
    format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len())
}
// The generated marker is synthetic authentication test data, never an operator token.
fn token() -> String {
    "synthetic-pairing-marker-".repeat(3)
}
fn server(replies: Vec<String>) -> (BridgeClient, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let thread = thread::spawn(move || {
        let mut requests = Vec::new();
        for reply in replies {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "client never made expected request"
                        );
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(e) => panic!("synthetic server: {e}"),
                }
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream.set_nonblocking(false).unwrap();
            let mut request = Vec::new();
            let mut byte = [0];
            while !request.ends_with(b"\r\n\r\n") {
                assert!(request.len() < 16384);
                stream.read_exact(&mut byte).unwrap();
                request.push(byte[0]);
            }
            let header_text = String::from_utf8(request.clone()).unwrap();
            let content_len = header_text
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            assert!(content_len <= 65536);
            let mut body = vec![0; content_len];
            stream.read_exact(&mut body).unwrap();
            request.extend_from_slice(&body);
            requests.push(String::from_utf8(request).unwrap());
            // A client rejecting the length may close before the response finishes.
            let _ = stream.write_all(reply.as_bytes());
        }
        requests
    });
    (
        BridgeClient::new(&format!("http://{address}"), &token()).unwrap(),
        thread,
    )
}

const REQUEST_ID: &str = "00000000-0000-4000-8000-000000000001";
fn history_fixture() -> Value {
    json!({"protocol_version":1,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace","callsign":"OM0RX/P","generated_at_unix_ms":now_ms().unwrap(),
        "contacts":[{"contact_id":REQUEST_ID,"callsign":"OM0RX/P","band":"20m","mode":"FT8","started_at_unix_ms":1_790_000_000_000u64,"frequency_hz":14074000,"rst_sent":"-10","rst_received":"-11","grid":"JN88","comment":"Synthetic\nmultiline history","contact_revision":null}],
        "matrix":[{"band":"20m","mode":"FT8","worked":true,"confirmed":false}]})
}
fn history_hello() -> Value {
    let mut h = hello();
    h["capabilities"] = json!(["snapshot.read", "logbook.read"]);
    h
}
#[test]
fn explicit_history_preserves_owner_facts_and_encodes_portable_calls() {
    let (mut c, s) = server(vec![
        json_response(history_hello()),
        json_response(history_fixture()),
    ]);
    c.connect().unwrap();
    let history = c.history("om0rx/p").unwrap();
    assert_eq!(history.contacts[0].contact_revision, None);
    assert!(history.matrix[0].worked);
    assert!(!history.matrix[0].confirmed);
    let requests = s.join().unwrap();
    assert!(requests[1].starts_with("GET /xlog/v1/history?callsign=OM0RX%2FP HTTP/1.1"));
}
#[test]
fn history_does_not_send_without_capability_or_valid_call() {
    let (mut c, s) = server(vec![json_response(hello())]);
    c.connect().unwrap();
    assert_eq!(c.history("W1AW").unwrap_err(), Error::Capability);
    assert_eq!(c.history("W1AW&secret=true").unwrap_err(), Error::Invalid);
    assert_eq!(s.join().unwrap().len(), 1);
}
#[test]
fn history_refuses_foreign_workspace_and_wrong_call() {
    for (field, value, error) in [
        ("workspace_id", "foreign", Error::IdentityChanged),
        ("callsign", "W1AW", Error::Invalid),
    ] {
        let mut h = history_fixture();
        h[field] = json!(value);
        let (mut c, s) = server(vec![json_response(history_hello()), json_response(h)]);
        c.connect().unwrap();
        assert_eq!(c.history("OM0RX/P").unwrap_err(), error);
        if error == Error::IdentityChanged {
            assert!(c.hello().is_none());
        }
        s.join().unwrap();
    }
}
#[test]
fn history_bounds_rows_and_never_invents_edit_revision() {
    let mut cases = Vec::new();
    let mut h = history_fixture();
    h["contacts"] = json!(vec![h["contacts"][0].clone(); 26]);
    cases.push(h);
    let mut h = history_fixture();
    h["contacts"][0]["contact_revision"] = json!(0);
    cases.push(h);
    let mut h = history_fixture();
    h["matrix"] = json!(vec![h["matrix"][0].clone(); 2]);
    cases.push(h);
    for h in cases {
        let (mut c, s) = server(vec![json_response(history_hello()), json_response(h)]);
        c.connect().unwrap();
        assert_eq!(c.history("OM0RX/P").unwrap_err(), Error::Invalid);
        s.join().unwrap();
    }
}
fn control_hello() -> Value {
    let mut h = hello();
    h["capabilities"] = json!(["snapshot.read", "station.tune", "rotor.point", "rotor.stop"]);
    h
}
fn receipt(status: &str) -> Value {
    json!({"request_id":REQUEST_ID,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace","status":status,"message":null})
}
fn tune() -> Command {
    Command::StationTune {
        radio_id: "fake-radio".into(),
        frequency_hz: 14075000,
        mode: "USB".into(),
    }
}

#[test]
fn action_is_bound_to_identity_revision_and_observed_target() {
    let (mut c, s) = server(vec![
        json_response(control_hello()),
        json_response(snapshot()),
        json_response(receipt("applied")),
    ]);
    c.connect().unwrap();
    c.poll().unwrap();
    assert_eq!(
        c.command(REQUEST_ID, tune()).unwrap().status,
        CommandStatus::Applied
    );
    assert_eq!(c.snapshot().unwrap_err(), Error::Disconnected);
    let requests = s.join().unwrap();
    let action = &requests[2];
    assert!(action.starts_with("POST /xlog/v1/command HTTP/1.1"));
    let value: Value = serde_json::from_str(action.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(value["workspace_id"], "synthetic-workspace");
    assert_eq!(value["expected_revision"], 5);
    assert_eq!(value["command"]["kind"], "station.tune");
    assert_eq!(value["command"]["radio_id"], "fake-radio");
}
#[test]
fn actions_require_capability_and_fresh_non_transmitting_target() {
    let (mut c, s) = server(vec![json_response(hello()), json_response(snapshot())]);
    c.connect().unwrap();
    c.poll().unwrap();
    assert_eq!(
        c.command(REQUEST_ID, tune()).unwrap_err(),
        Error::Capability
    );
    s.join().unwrap();
    for (key, value) in [
        ("radio_id", json!("other")),
        ("transmitting", json!(true)),
        ("transmitting", Value::Null),
        ("connected", json!(false)),
    ] {
        let mut snap = snapshot();
        snap["station"][key] = value;
        let (mut c, s) = server(vec![json_response(control_hello()), json_response(snap)]);
        c.connect().unwrap();
        c.poll().unwrap();
        assert_eq!(c.command(REQUEST_ID, tune()).unwrap_err(), Error::Invalid);
        s.join().unwrap();
    }
}
#[test]
fn stale_action_is_refused_but_independent_rotor_stop_is_sent() {
    let (mut c, s) = server(vec![
        json_response(control_hello()),
        json_response(snapshot()),
        json_response(receipt("observed")),
    ]);
    c.connect().unwrap();
    let stop = c.stop_handle().unwrap();
    c.poll().unwrap();
    c.observation.as_mut().unwrap().received -= Duration::from_secs(11);
    assert_eq!(c.command(REQUEST_ID, tune()).unwrap_err(), Error::Stale);
    assert_eq!(
        stop.stop_rotor(REQUEST_ID, "fake-rotor").unwrap().status,
        CommandStatus::Observed
    );
    let requests = s.join().unwrap();
    let value: Value = serde_json::from_str(requests[2].split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(value["expected_revision"], 0);
    assert_eq!(value["command"]["kind"], "rotor.stop");
}
#[test]
fn malformed_or_wrong_identity_action_reply_is_uncertain_never_retried() {
    for mut r in [receipt("observed"), receipt("invalid-status")] {
        r["workspace_id"] = json!("other");
        let (mut c, s) = server(vec![
            json_response(control_hello()),
            json_response(snapshot()),
            json_response(r),
        ]);
        c.connect().unwrap();
        c.poll().unwrap();
        assert_eq!(
            c.command(REQUEST_ID, tune()).unwrap_err(),
            Error::OutcomeUnknown
        );
        assert_eq!(c.snapshot().unwrap_err(), Error::Disconnected);
        assert_eq!(s.join().unwrap().len(), 3);
    }
}
#[test]
fn receipt_lookup_distinguishes_missing_from_rejected_and_validates_identity() {
    let (mut c, s) = server(vec![
        json_response(control_hello()),
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
        json_response(receipt("rejected")),
    ]);
    c.connect().unwrap();
    assert!(c.receipt(REQUEST_ID).unwrap().is_none());
    assert_eq!(
        c.receipt(REQUEST_ID).unwrap().unwrap().status,
        CommandStatus::Rejected
    );
    let requests = s.join().unwrap();
    assert!(requests[1].starts_with(&format!("GET /xlog/v1/receipt/{REQUEST_ID} ")));
}
#[test]
fn invalid_action_request_id_and_out_of_range_rotor_never_leave_client() {
    let (mut c, s) = server(vec![
        json_response(control_hello()),
        json_response(snapshot()),
    ]);
    c.connect().unwrap();
    let stop = c.stop_handle().unwrap();
    c.poll().unwrap();
    assert_eq!(
        c.command(
            REQUEST_ID,
            Command::RotorPoint {
                rotor_id: "fake-rotor".into(),
                azimuth_deg: 360.0
            }
        )
        .unwrap_err(),
        Error::Invalid
    );
    assert_eq!(
        stop.stop_rotor("../../secrets", "fake-rotor").unwrap_err(),
        Error::Invalid
    );
    assert_eq!(c.command("bad", tune()).unwrap_err(), Error::Invalid);
    s.join().unwrap();
}

struct QueueFixture(std::path::PathBuf);
impl QueueFixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "xnexus-delivery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for QueueFixture {
    fn drop(&mut self) {
        for item in std::fs::read_dir(&self.0).unwrap() {
            let path = item.unwrap().path();
            assert_eq!(path.parent(), Some(self.0.as_path()));
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_dir(&self.0).unwrap();
    }
}
fn log_hello() -> Value {
    let mut h = hello();
    h["capabilities"] = json!(["snapshot.read", "logbook.write"]);
    h
}
fn queued_draft() -> outbox::LogbookMutation {
    outbox::LogbookMutation::Create {
        qso: outbox::QsoDraft {
            contact_id: REQUEST_ID.into(),
            callsign: "W1AW".into(),
            band: "20m".into(),
            mode: "FT8".into(),
            started_at_unix_ms: 1700000000000,
            frequency_hz: Some(14074000),
            rst_sent: "-10".into(),
            rst_received: "-12".into(),
            grid: "FN31".into(),
            comment: "synthetic".into(),
        },
    }
}

#[test]
fn durable_queue_sends_owner_create_and_retains_terminal_receipt() {
    let f = QueueFixture::new();
    let mut queue = outbox::Outbox::open(&f.0).unwrap();
    queue
        .enqueue("synthetic-workspace", REQUEST_ID, queued_draft())
        .unwrap();
    let (mut c, s) = server(vec![
        json_response(log_hello()),
        json_response(snapshot()),
        json_response(receipt("applied")),
    ]);
    c.connect().unwrap();
    c.poll().unwrap();
    queue.submit(&mut c, REQUEST_ID).unwrap();
    assert!(matches!(
        queue.entries()[0].state,
        outbox::OutboxState::Settled(_)
    ));
    let requests = s.join().unwrap();
    let body: Value = serde_json::from_str(requests[2].split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["command"]["kind"], "logbook.create");
    assert_eq!(body["command"]["qso"]["callsign"], "W1AW");
}
#[test]
fn crash_after_dispatch_recovers_by_receipt_without_second_post() {
    let f = QueueFixture::new();
    {
        let mut queue = outbox::Outbox::open(&f.0).unwrap();
        queue
            .enqueue("synthetic-workspace", REQUEST_ID, queued_draft())
            .unwrap();
        let (mut c, s) = server(vec![
            json_response(log_hello()),
            json_response(snapshot()),
            json_response(json!({"broken":true})),
        ]);
        c.connect().unwrap();
        c.poll().unwrap();
        assert_eq!(
            queue.submit(&mut c, REQUEST_ID).unwrap_err(),
            outbox::DeliveryError::Bridge(Error::OutcomeUnknown)
        );
        s.join().unwrap();
    }
    let mut queue = outbox::Outbox::open(&f.0).unwrap();
    let (mut c, s) = server(vec![
        json_response(log_hello()),
        json_response(receipt("applied")),
    ]);
    c.connect().unwrap();
    queue.submit(&mut c, REQUEST_ID).unwrap();
    let requests = s.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].starts_with("GET /xlog/v1/receipt/"));
}
#[test]
fn missing_receipt_needs_separate_explicit_retry_using_same_operation() {
    let f = QueueFixture::new();
    let mut queue = outbox::Outbox::open(&f.0).unwrap();
    queue
        .enqueue("synthetic-workspace", REQUEST_ID, queued_draft())
        .unwrap();
    queue
        .mark_attempted(REQUEST_ID, "synthetic-workspace")
        .unwrap();
    let missing = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    let (mut c, s) = server(vec![
        json_response(log_hello()),
        json_response(snapshot()),
        missing.into(),
        missing.into(),
        json_response(receipt("applied")),
    ]);
    c.connect().unwrap();
    c.poll().unwrap();
    assert_eq!(
        queue.submit(&mut c, REQUEST_ID).unwrap_err(),
        outbox::DeliveryError::Queue(outbox::OutboxError::ReceiptRequired)
    );
    queue.retry_missing(&mut c, REQUEST_ID).unwrap();
    let requests = s.join().unwrap();
    assert_eq!(
        requests.iter().filter(|r| r.starts_with("POST ")).count(),
        1
    );
    let body: Value = serde_json::from_str(requests[4].split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["request_id"], REQUEST_ID);
}
#[test]
fn wrong_workspace_or_missing_grant_does_not_record_attempt_or_send() {
    let f = QueueFixture::new();
    let mut queue = outbox::Outbox::open(&f.0).unwrap();
    queue
        .enqueue("synthetic-workspace", REQUEST_ID, queued_draft())
        .unwrap();
    let (mut c, s) = server(vec![json_response(hello())]);
    c.connect().unwrap();
    assert_eq!(
        queue.submit(&mut c, REQUEST_ID).unwrap_err(),
        outbox::DeliveryError::Bridge(Error::Capability)
    );
    s.join().unwrap();
    let mut other = log_hello();
    other["workspace_id"] = json!("other");
    let (mut c, s) = server(vec![json_response(other)]);
    c.connect().unwrap();
    assert_eq!(
        queue.submit(&mut c, REQUEST_ID).unwrap_err(),
        outbox::DeliveryError::Queue(outbox::OutboxError::WorkspaceChanged)
    );
    s.join().unwrap();
    assert_eq!(queue.entries()[0].state, outbox::OutboxState::Pending);
}
fn assert_bad_snapshot(value: Value, expected: Error) {
    let (mut client, server) = server(vec![json_response(hello()), json_response(value)]);
    client.connect().unwrap();
    assert_eq!(client.poll().unwrap_err(), expected);
    assert_eq!(client.snapshot().unwrap_err(), Error::Disconnected);
    assert_eq!(client.poll().unwrap_err(), Error::Disconnected);
    server.join().unwrap();
}

#[test]
fn paired_read_uses_exact_routes_and_authentication() {
    let (mut client, server) = server(vec![json_response(hello()), json_response(snapshot())]);
    client.connect().unwrap();
    let s = client.poll().unwrap();
    assert_eq!(s.needed()[0].callsign, "W1AW");
    assert_eq!(s.station.frequency_hz, Some(14074000));
    assert_eq!(s.rotor.azimuth_deg, Some(123.0));
    let requests = server.join().unwrap();
    assert!(requests[0].starts_with("GET /xlog/v1/hello HTTP/1.1"));
    assert!(requests[1].starts_with("GET /xlog/v1/snapshot HTTP/1.1"));
    for request in requests {
        assert!(request
            .to_ascii_lowercase()
            .contains(&format!("authorization: bearer {}", token())));
        assert!(!request.to_ascii_lowercase().contains("origin:"));
    }
}
#[test]
fn only_canonical_explicit_loopback_endpoint_is_accepted() {
    for endpoint in [
        "http://localhost:1234",
        "http://127.1:1234",
        "http://2130706433:1234",
        "http://127.0.0.1:0",
        "http://127.0.0.1:65536",
        "http://127.0.0.1:1234/",
        "http://127.0.0.1:1234@evil.test",
        "https://127.0.0.1:1234",
        "http://192.168.0.1:1234",
        "http://[::1]:1234",
        "http://127.0.0.1:1234?token=x",
    ] {
        assert!(
            matches!(BridgeClient::new(endpoint, &token()), Err(Error::Endpoint)),
            "{endpoint}"
        );
    }
    assert!(BridgeClient::new("http://127.0.0.1:1234", &token()).is_ok());
}
#[test]
fn rejects_empty_short_oversized_and_header_injection_tokens() {
    for token in [
        String::new(),
        "short".into(),
        "x".repeat(4097),
        format!("{}\r\nInjected: x", token()),
    ] {
        assert!(matches!(
            BridgeClient::new("http://127.0.0.1:1234", &token),
            Err(Error::Token)
        ));
    }
}
#[test]
fn no_snapshot_before_handshake() {
    let mut client = BridgeClient::new("http://127.0.0.1:1234", &token()).unwrap();
    assert_eq!(client.poll().unwrap_err(), Error::Disconnected);
}
#[test]
fn protocol_application_and_capability_are_required() {
    for (key, value) in [
        ("protocol_version", json!(2)),
        ("application", json!("Other app")),
        ("capabilities", json!([])),
    ] {
        let mut h = hello();
        h[key] = value;
        let (mut c, s) = server(vec![json_response(h)]);
        assert_eq!(c.connect().unwrap_err(), Error::Incompatible);
        assert_eq!(c.poll().unwrap_err(), Error::Disconnected);
        s.join().unwrap();
    }
}
#[test]
fn workspace_or_server_change_invalidates_session() {
    for key in ["workspace_id", "server_instance"] {
        let mut s = snapshot();
        s[key] = json!("different");
        assert_bad_snapshot(s, Error::IdentityChanged);
    }
}
#[test]
fn refuses_stale_future_and_unbounded_freshness() {
    for (key, value) in [
        ("generated_at_unix_ms", json!(1)),
        ("generated_at_unix_ms", json!(now_ms().unwrap() + 20000)),
        ("max_age_ms", json!(0)),
        ("max_age_ms", json!(10001)),
    ] {
        let mut s = snapshot();
        s[key] = value;
        assert_bad_snapshot(s, Error::Stale);
    }
}
#[test]
fn freshness_expires_without_network_activity() {
    let (mut c, s) = server(vec![json_response(hello()), json_response(snapshot())]);
    c.connect().unwrap();
    c.poll().unwrap();
    c.observation.as_mut().unwrap().received -= Duration::from_secs(11);
    assert_eq!(c.snapshot().unwrap_err(), Error::Stale);
    s.join().unwrap();
}
#[test]
fn repeated_identical_snapshot_does_not_renew_monotonic_expiry() {
    let value = snapshot();
    let (mut c, s) = server(vec![
        json_response(hello()),
        json_response(value.clone()),
        json_response(value),
    ]);
    c.connect().unwrap();
    c.poll().unwrap();
    c.observation.as_mut().unwrap().received -= Duration::from_secs(8);
    c.poll().unwrap();
    assert!(c.observation.as_ref().unwrap().remaining <= Duration::from_secs(2));
    s.join().unwrap();
}
#[test]
fn rollback_or_changed_same_revision_is_refused() {
    for change in [true, false] {
        let first = snapshot();
        let mut second = first.clone();
        if change {
            second["station"]["mode"] = json!("CW");
        } else {
            second["revision"] = json!(4);
        }
        let (mut c, s) = server(vec![
            json_response(hello()),
            json_response(first),
            json_response(second),
        ]);
        c.connect().unwrap();
        c.poll().unwrap();
        assert_eq!(c.poll().unwrap_err(), Error::Revision);
        assert_eq!(c.snapshot().unwrap_err(), Error::Disconnected);
        s.join().unwrap();
    }
}
#[test]
fn newer_revision_and_explicit_reconnect_replace_prior_state() {
    let first = snapshot();
    let mut second = first.clone();
    second["revision"] = json!(6);
    second["spots"] = json!([]);
    let mut h = hello();
    h["workspace_id"] = json!("new-workspace");
    let mut third = second.clone();
    third["workspace_id"] = json!("new-workspace");
    third["revision"] = json!(1);
    let (mut c, s) = server(vec![
        json_response(hello()),
        json_response(first),
        json_response(second),
        json_response(h),
        json_response(third),
    ]);
    c.connect().unwrap();
    c.poll().unwrap();
    assert!(c.poll().unwrap().spots.is_empty());
    c.connect().unwrap();
    assert_eq!(c.snapshot().unwrap_err(), Error::Disconnected);
    assert_eq!(c.poll().unwrap().revision, 1);
    s.join().unwrap();
}
#[test]
fn unknown_is_not_needed_and_owner_score_is_preserved() {
    let mut value = snapshot();
    let mut second = value["spots"][0].clone();
    second["id"] = json!("other");
    second["need"]["status"] = json!("unknown");
    second["need"]["score"] = json!(9999);
    value["spots"].as_array_mut().unwrap().push(second);
    let (mut c, s) = server(vec![json_response(hello()), json_response(value)]);
    c.connect().unwrap();
    let needed = c.poll().unwrap().needed();
    assert_eq!(needed.len(), 1);
    assert_eq!(needed[0].need.score, 100);
    s.join().unwrap();
}
#[test]
fn rejects_unknown_need_enum_and_duplicate_spots() {
    let mut value = snapshot();
    value["spots"][0]["need"]["status"] = json!("probably_new");
    assert_bad_snapshot(value, Error::Invalid);
    let mut value = snapshot();
    let duplicate = value["spots"][0].clone();
    value["spots"].as_array_mut().unwrap().push(duplicate);
    assert_bad_snapshot(value, Error::Invalid);
}
#[test]
fn needs_evidence_and_valid_observations() {
    let mut value = snapshot();
    value["spots"][0]["need"]["reasons"] = json!([]);
    assert_bad_snapshot(value, Error::Invalid);
    let mut value = snapshot();
    value["station"]["frequency_hz"] = json!(0);
    assert_bad_snapshot(value, Error::Invalid);
    let mut value = snapshot();
    value["rotor"]["azimuth_deg"] = json!(360);
    assert_bad_snapshot(value, Error::Invalid);
    let mut value = snapshot();
    value["station"]["radio_id"] = Value::Null;
    assert_bad_snapshot(value, Error::Invalid);
    let mut value = snapshot();
    value["spots"][0]["callsign"] = json!("<script>");
    assert_bad_snapshot(value, Error::Invalid);
}
#[test]
fn owner_geography_is_preserved_and_invalid_coordinates_are_refused() {
    let mut value = snapshot();
    value["spots"][0]["country"] = json!("United States");
    value["spots"][0]["adif"] = json!(291);
    value["spots"][0]["latitude"] = json!(41.7);
    value["spots"][0]["longitude"] = json!(-72.7);
    value["spots"][0]["call_status"] = json!("worked");
    let (mut c, s) = server(vec![json_response(hello()), json_response(value.clone())]);
    c.connect().unwrap();
    let spot = &c.poll().unwrap().spots[0];
    assert_eq!(spot.adif, Some(291));
    assert_eq!(spot.latitude, Some(41.7));
    assert_eq!(spot.call_status.as_deref(), Some("worked"));
    assert_eq!(spot.need.status, NeedStatus::Needed);
    s.join().unwrap();
    value["spots"][0]["latitude"] = json!(91);
    assert_bad_snapshot(value, Error::Invalid);
}
#[test]
fn bounded_identifiers_and_spot_count() {
    let mut h = hello();
    h["workspace_id"] = json!("x".repeat(129));
    let (mut c, s) = server(vec![json_response(h)]);
    assert_eq!(c.connect().unwrap_err(), Error::Invalid);
    s.join().unwrap();
    let mut value = snapshot();
    value["spots"] = json!(vec![value["spots"][0].clone(); 4097]);
    assert_bad_snapshot(value, Error::Invalid);
}
#[test]
fn http_error_revokes_prior_observation_and_does_not_leak_body() {
    let (mut c,s)=server(vec![json_response(hello()),json_response(snapshot()),"HTTP/1.1 401 Unauthorized\r\nContent-Length: 15\r\nConnection: close\r\n\r\nprivate-details".into()]);
    c.connect().unwrap();
    c.poll().unwrap();
    let error = c.poll().unwrap_err();
    assert_eq!(error, Error::Http(401));
    assert!(!error.to_string().contains("private-details"));
    assert_eq!(c.snapshot().unwrap_err(), Error::Disconnected);
    s.join().unwrap();
}
#[test]
fn redirect_is_not_followed_or_sent_credentials() {
    let trap = TcpListener::bind("127.0.0.1:0").unwrap();
    trap.set_nonblocking(true).unwrap();
    let reply=format!("HTTP/1.1 302 Found\r\nLocation: http://{}/stolen\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",trap.local_addr().unwrap());
    let (mut c, s) = server(vec![reply]);
    assert_eq!(c.connect().unwrap_err(), Error::Http(302));
    assert!(matches!(trap.accept(),Err(e) if e.kind()==std::io::ErrorKind::WouldBlock));
    s.join().unwrap();
}
#[test]
fn malformed_json_wrong_type_and_size_bombs_fail_closed() {
    for reply in [
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 1\r\nConnection: close\r\n\r\n{".into(),
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
    ] {
        let (mut c,s)=server(vec![reply]);assert_eq!(c.connect().unwrap_err(),Error::Invalid);s.join().unwrap();
    }
    let (mut c,s)=server(vec!["HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 999999999\r\nConnection: close\r\n\r\n".into()]);
    assert_eq!(c.connect().unwrap_err(), Error::TooLarge);
    s.join().unwrap();
    let (mut c, s) = server(vec![format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
        " ".repeat(65537)
    )]);
    assert_eq!(c.connect().unwrap_err(), Error::TooLarge);
    s.join().unwrap();
}
#[test]
fn additive_fields_are_compatible_but_missing_required_fields_are_not() {
    let mut h = hello();
    h["future_extension"] = json!(true);
    let (mut c, s) = server(vec![json_response(h)]);
    c.connect().unwrap();
    s.join().unwrap();
    let mut value = snapshot();
    value.as_object_mut().unwrap().remove("station");
    assert_bad_snapshot(value, Error::Invalid);
}
#[test]
fn refused_connection_has_bounded_redacted_error() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    let mut c = BridgeClient::new(&format!("http://{addr}"), &token()).unwrap();
    let start = Instant::now();
    assert_eq!(c.connect().unwrap_err(), Error::Transport);
    assert!(start.elapsed() < Duration::from_secs(5));
}
