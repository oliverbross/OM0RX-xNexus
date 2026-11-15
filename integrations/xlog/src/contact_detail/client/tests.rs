// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};
const ID: &str = "abcdef12-3456-4abc-8def-123456abcdef";
const TOKEN: &str = "synthetic-local-connection";
type Reply = Box<dyn FnOnce(&str) -> String + Send>;
fn corpus(name: &str) -> Value {
    let all: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/contact-detail-swift.json"
    ))
    .unwrap();
    all[name].clone()
}
fn envelope(name: &str) -> Value {
    json!({"protocol_version":1,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace",
        "generation":"synthetic-workspace","worker_generation":7,"generated_at_unix_ms":now_ms().unwrap(),"max_age_ms":5000,"detail":corpus(name)})
}
fn wire(status: u16, value: Value) -> String {
    wire_body(status, value.to_string())
}
fn wire_body(status: u16, body: String) -> String {
    format!("HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len())
}

fn reply(name: &'static str) -> Reply {
    Box::new(move |_| wire(200, envelope(name)))
}
fn serve(replies: Vec<Reply>, capability: bool) -> (BridgeClient, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let mut hello = crate::tests::hello();
    if capability {
        hello["capabilities"] = json!(["snapshot.read", "logbook.contact.read"]);
    }
    let mut replies = replies;
    replies.insert(0, Box::new(move |_| wire(200, hello)));
    let handle = thread::spawn(move || {
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
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream.set_nonblocking(false).unwrap();
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
    (
        BridgeClient::new(&endpoint, &crate::tests::token()).unwrap(),
        handle,
    )
}
fn read(client: &mut ContactDetailClient, id: &str) -> Result<ContactLease, ContactReadError> {
    let request = client.capture(TOKEN, id).unwrap();
    client.read(&request)
}
#[test]
fn actual_producer_cases_roundtrip_over_authenticated_exact_get_without_fallback() {
    let (mut bridge, server) = serve(
        vec![reply("found"), reply("deleted"), reply("not_found")],
        true,
    );
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    for name in ["found", "deleted", "not_found"] {
        let expected = corpus(name);
        let id = expected["contact_id"].as_str().unwrap();
        let lease = read(&mut client, id).unwrap();
        assert_eq!(
            serde_json::to_value(&lease).unwrap()["payload"]["detail"],
            expected
        );
        assert!(lease.remaining_valid_ms() > 0 && lease.remaining_valid_ms() <= 5000);
    }
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 4);
    for request in &requests[1..] {
        assert!(request.starts_with("GET /xlog/v1/logbook/contacts/"));
        assert!(request
            .to_lowercase()
            .contains("authorization: bearer synthetic-pairing-marker-"));
        assert!(!request.to_lowercase().contains("origin:"));
        assert!(!request.contains('?'));
        assert!(!request.contains(TOKEN));
        assert!(request.ends_with("\r\n\r\n"));
    }
}
#[test]
fn capability_and_canonical_selection_are_rejected_before_http() {
    let (mut bridge, server) = serve(vec![], false);
    bridge.connect().unwrap();
    assert!(matches!(
        bridge.contact_detail_handle(TOKEN),
        Err(Error::Capability)
    ));
    server.join().unwrap();
    let (mut bridge, server) = serve(vec![], true);
    bridge.connect().unwrap();
    let client = bridge.contact_detail_handle(TOKEN).unwrap();
    for id in [
        ID.to_uppercase(),
        format!("{ID}?x=1"),
        format!("{ID}/"),
        ID.replace('-', ""),
        "not-a-contact".into(),
    ] {
        assert!(client.capture(TOKEN, &id).is_err());
    }
    assert!(client.capture("old-token", ID).is_err());
    server.join().unwrap();
}
#[test]
fn unavailable_status_and_stable_errors_never_become_missing_contact() {
    let typed: Reply = Box::new(|_| {
        let mut v = envelope("deleted");
        v["detail"]["status"] = json!("unavailable");
        v["detail"]["reason"] = json!("detail_too_large");
        v["detail"]["editability"]["reason"] = json!("detail_too_large");
        wire(503, v)
    });
    let (mut bridge, server) = serve(
        vec![
            typed,
            Box::new(|_| wire(503, json!({"error":"contact_detail_busy"}))),
        ],
        true,
    );
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let first = read(&mut client, ID).unwrap();
    assert_eq!(
        first.value().unwrap().detail.data().status,
        Status::Unavailable
    );
    assert!(
        matches!(read(&mut client,ID),Err(ContactReadError::Unavailable(code)) if code=="contact_detail_busy")
    );
    assert!(serde_json::to_value(first).is_err());
    assert!(bridge.hello().is_some());
    server.join().unwrap();
}
#[test]
fn status_body_mismatch_and_malformed_error_are_rejected() {
    let variants: Vec<Reply> = vec![
        Box::new(|_| wire(503, envelope("found"))),
        Box::new(|_| wire(200, json!({"error":"busy"}))),
        Box::new(|_| wire(503, json!({"error":"private\nerror"}))),
        Box::new(|_| wire(503, json!({"error":"busy","extra":1}))),
        Box::new(|_| {
            let mut v = envelope("deleted");
            v["detail"]["status"] = json!("unavailable");
            v["detail"]["reason"] = json!("busy");
            wire(200, v)
        }),
    ];
    let (mut bridge, server) = serve(variants, true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    for _ in 0..5 {
        assert!(matches!(
            read(&mut client, ID),
            Err(ContactReadError::Bridge(Error::Invalid))
        ));
    }
    server.join().unwrap();
}
#[test]
fn fixed_envelope_and_original_detail_reject_null_missing_extra_base_and_numeric_errors() {
    let mutations: Vec<(&str, Value)> = vec![
        ("/worker_generation", json!(0)),
        ("/worker_generation", json!(SAFE + 1)),
        ("/worker_generation", json!(1.5)),
        ("/max_age_ms", json!(5001)),
        ("/detail", Value::Null),
        ("/detail/editability/base/frequency_hz", json!(0)),
        (
            "/detail/contact_id",
            json!("00000000-0000-4000-8000-000000000999"),
        ),
    ];
    let mut replies: Vec<Reply> = mutations
        .into_iter()
        .map(|(p, x)| {
            Box::new(move |_: &str| {
                let mut v = envelope("found");
                *v.pointer_mut(p).unwrap() = x;
                wire(200, v)
            }) as Reply
        })
        .collect();
    replies.push(Box::new(|_| {
        let mut v = envelope("found");
        v.as_object_mut().unwrap().remove("generation");
        wire(200, v)
    }));
    replies.push(Box::new(|_| {
        let mut v = envelope("found");
        v["extra"] = json!(true);
        wire(200, v)
    }));
    replies.push(Box::new(|_| {
        let mut v = envelope("found");
        v["detail"]["editability"]["base"]
            .as_object_mut()
            .unwrap()
            .remove("frequency_hz");
        wire(200, v)
    }));
    let count = replies.len();
    let (mut bridge, server) = serve(replies, true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    for _ in 0..count {
        assert!(read(&mut client, ID).is_err());
    }
    server.join().unwrap();
}
#[test]
fn old_future_or_regressing_wall_clock_and_elapsed_queue_are_rejected() {
    let (mut bridge, server) = serve(vec![], true);
    bridge.connect().unwrap();
    let client = bridge.contact_detail_handle(TOKEN).unwrap();
    let request = client.capture(TOKEN, ID).unwrap();
    let hello = bridge.hello().unwrap();
    let mut env: Envelope = serde_json::from_value(envelope("found")).unwrap();
    env.generated_at_unix_ms = request.started_wall;
    assert_eq!(
        env.validate(hello, &request, request.started_wall, 200),
        Ok(())
    );
    assert_eq!(
        env.validate(hello, &request, request.started_wall - 1, 200),
        Err(Error::Stale)
    );
    env.generated_at_unix_ms = request.started_wall - 1;
    assert_eq!(
        env.validate(hello, &request, request.started_wall, 200),
        Err(Error::Stale)
    );
    env.generated_at_unix_ms = request.started_wall + 1;
    assert_eq!(
        env.validate(hello, &request, request.started_wall, 200),
        Err(Error::Stale)
    );
    env.generated_at_unix_ms = request.started_wall;
    let mut old = request;
    old.started -= Duration::from_millis(5000);
    assert_eq!(
        env.validate(hello, &old, old.started_wall, 200),
        Err(Error::Stale)
    );
    server.join().unwrap();
}
#[test]
fn delayed_http_consumes_original_monotonic_lease() {
    let (mut bridge, server) = serve(
        vec![Box::new(|_| {
            let v = envelope("found");
            thread::sleep(Duration::from_millis(140));
            wire(200, v)
        })],
        true,
    );
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let request = client.capture(TOKEN, ID).unwrap();
    let start = request.started;
    let lease = client.read(&request).unwrap();
    assert!(start.elapsed() >= Duration::from_millis(140));
    assert!(lease.expires <= start + Duration::from_millis(5000));
    assert!(lease.remaining_valid_ms() <= 4860);
    server.join().unwrap();
}
#[test]
fn paired_disconnect_during_http_and_local_retirement_reject_delayed_serialization() {
    let (tx, rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel();
    let (mut bridge, server) = serve(
        vec![Box::new(move |_| {
            tx.send(()).unwrap();
            go_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            wire(200, envelope("found"))
        })],
        true,
    );
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let request = client.capture(TOKEN, ID).unwrap();
    let job = thread::spawn(move || client.read(&request));
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    bridge.disconnect();
    go_tx.send(()).unwrap();
    assert!(matches!(
        job.join().unwrap(),
        Err(ContactReadError::Bridge(Error::Disconnected))
    ));
    server.join().unwrap();
    let (mut bridge, server) = serve(vec![reply("found")], true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let fence = client.fence();
    let lease = read(&mut client, ID).unwrap();
    fence.revoke();
    assert!(serde_json::to_value(lease).is_err());
    server.join().unwrap();
}
#[test]
fn auth_revocation_retires_session_and_old_facts() {
    let (mut bridge, server) = serve(
        vec![
            reply("found"),
            Box::new(|_| wire(401, json!({"error":"unauthorized"}))),
        ],
        true,
    );
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let old = read(&mut client, ID).unwrap();
    assert!(matches!(
        read(&mut client, ID),
        Err(ContactReadError::Bridge(Error::Http(401)))
    ));
    assert!(bridge.hello().is_none());
    assert!(serde_json::to_value(old).is_err());
    server.join().unwrap();
}
#[test]
fn owner_workspace_protocol_and_worker_changes_retire_authority() {
    for (key, value) in [
        ("server_instance", json!("other")),
        ("workspace_id", json!("other")),
        ("generation", json!("other")),
        ("protocol_version", json!(2)),
        ("worker_generation", json!(8)),
    ] {
        let (mut bridge, server) = serve(
            vec![
                reply("found"),
                Box::new(move |_| {
                    let mut v = envelope("found");
                    v[key] = value;
                    wire(200, v)
                }),
            ],
            true,
        );
        bridge.connect().unwrap();
        let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
        let old = read(&mut client, ID).unwrap();
        assert!(read(&mut client, ID).is_err());
        assert!(bridge.hello().is_none());
        assert!(serde_json::to_value(old).is_err());
        server.join().unwrap();
    }
}
#[test]
fn byte_caps_content_type_and_redirect_are_not_bypassed() {
    let replies: Vec<Reply> = vec![
        Box::new(|_| {
            format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",MAX_ENVELOPE_BYTES+1)
        }),
        Box::new(|_| {
            format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                " ".repeat(MAX_ENVELOPE_BYTES + 1)
            )
        }),
        Box::new(|_| wire(200, envelope("found")).replace("application/json", "text/plain")),
        Box::new(|_| {
            "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/forbidden\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into()
        }),
    ];
    let (mut bridge, server) = serve(replies, true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    for _ in 0..4 {
        assert!(read(&mut client, ID).is_err());
    }
    assert_eq!(server.join().unwrap().len(), 5);
}
#[test]
fn lease_expiry_serialization_and_repeated_stamp_cannot_renew_current_facts() {
    let (mut bridge, server) = serve(vec![reply("found")], true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let mut lease = read(&mut client, ID).unwrap();
    lease.expires = Instant::now() - Duration::from_millis(1);
    assert_eq!(lease.remaining_valid_ms(), 0);
    assert!(serde_json::to_value(&lease).is_err());
    server.join().unwrap();
    let (mut bridge, server) = serve(vec![], true);
    bridge.connect().unwrap();
    let client = bridge.contact_detail_handle(TOKEN).unwrap();
    let request = client.capture(TOKEN, ID).unwrap();
    let old_expiry = request.started + Duration::from_millis(4000);
    let expiry = deadline(
        &request,
        request.started_wall,
        request.started_wall,
        Some((request.started_wall, old_expiry)),
    )
    .unwrap();
    assert_eq!(expiry, old_expiry);
    assert!(matches!(
        deadline(
            &request,
            request.started_wall,
            request.started_wall,
            Some((request.started_wall + 1, old_expiry))
        ),
        Err(Error::Stale)
    ));
    server.join().unwrap();
}

#[test]
fn endpoint_policy_is_stricter_than_retained_codec_without_rewriting_facts() {
    let base = corpus("found");
    let mutations = vec![
        ("/contact_revision", json!(2)),
        (
            "/fields/radio/tx_power_w",
            json!({"state":"value","value":-1.0,"reason":null}),
        ),
        ("/fields/confirmations/paper/source/value", json!("lotw")),
        ("/fields/uploads/qrz/source/value", json!("eqsl")),
        ("/locate/reason", json!("some_other_route")),
        (
            "/fields/contact/notes",
            json!({"state":"unavailable","value":null,"reason":"not recorded"}),
        ),
    ];
    for (path, value) in mutations {
        let mut v = base.clone();
        *v.pointer_mut(path).unwrap() = value;
        let detail: ContactDetail = serde_json::from_value(v).unwrap();
        assert_eq!(endpoint_policy(&detail), Err(Error::Invalid), "{path}");
    }
    let mut deleted = corpus("deleted");
    deleted["contact_revision"] = Value::Null;
    assert!(endpoint_policy(&serde_json::from_value(deleted).unwrap()).is_err());
    let mut missing = corpus("not_found");
    missing["contact_revision"] = json!(0);
    assert!(endpoint_policy(&serde_json::from_value(missing).unwrap()).is_err());
}
#[test]
fn selection_capture_retires_inflight_and_displayed_facts_without_http_lock() {
    let (tx, rx) = std::sync::mpsc::channel();
    let (go_tx, go_rx) = std::sync::mpsc::channel();
    let (mut bridge, server) = serve(
        vec![Box::new(move |_| {
            tx.send(()).unwrap();
            go_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            wire(200, envelope("found"))
        })],
        true,
    );
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let fence = client.fence();
    let request = client.capture(TOKEN, ID).unwrap();
    let job = thread::spawn(move || client.read(&request));
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let newer = fence
        .capture(TOKEN, "00000000-0000-4000-8000-000000000999")
        .unwrap();
    assert!(newer.sequence > 1);
    go_tx.send(()).unwrap();
    assert!(matches!(
        job.join().unwrap(),
        Err(ContactReadError::Bridge(Error::Disconnected))
    ));
    server.join().unwrap();
    let (mut bridge, server) = serve(vec![reply("found")], true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let lease = read(&mut client, ID).unwrap();
    assert!(client.capture(TOKEN, "invalid-contact").is_err());
    assert!(serde_json::to_value(lease).is_err());
    server.join().unwrap();
}
#[test]
fn request_epoch_cannot_wrap_and_foreign_same_token_handle_is_not_equivalent() {
    let (mut bridge, server) = serve(vec![], true);
    bridge.connect().unwrap();
    let client = bridge.contact_detail_handle(TOKEN).unwrap();
    let other = bridge.contact_detail_handle(TOKEN).unwrap();
    let request = other.capture(TOKEN, ID).unwrap();
    let mut client = client;
    assert!(matches!(
        client.read(&request),
        Err(ContactReadError::Bridge(Error::IdentityChanged))
    ));
    server.join().unwrap();
    let (mut bridge, server) = serve(vec![], true);
    bridge.connect().unwrap();
    let client = bridge.contact_detail_handle(TOKEN).unwrap();
    client.fence.0.requests.lock().unwrap().sequence = u64::MAX;
    assert!(matches!(
        client.capture(TOKEN, ID),
        Err(Error::Disconnected)
    ));
    assert!(!client.fence.0.live.load(Ordering::Acquire));
    server.join().unwrap();
}
#[test]
fn duplicate_json_keys_and_oversized_detail_do_not_bypass_typed_parser() {
    let replies: Vec<Reply> = vec![
        Box::new(|_| {
            wire_body(
                200,
                envelope("found").to_string().replace(
                    "\"worker_generation\":7",
                    "\"worker_generation\":7,\"worker_generation\":7",
                ),
            )
        }),
        Box::new(|_| {
            let mut v = envelope("found");
            v["detail"]["fields"]["contact"]["notes"] =
                json!({"state":"value","value":"N".repeat(16385),"reason":null});
            wire(200, v)
        }),
    ];
    let (mut bridge, server) = serve(replies, true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    assert!(read(&mut client, ID).is_err());
    assert!(read(&mut client, ID).is_err());
    server.join().unwrap();
}
#[test]
fn retained_handle_rejects_logbook_and_contact_revision_regressions() {
    let high: Reply = Box::new(|_| {
        let mut v = envelope("found");
        v["detail"]["logbook_revision"] = json!(10);
        v["detail"]["contact_revision"] = json!(5);
        wire(200, v)
    });
    let lower_contact: Reply = Box::new(|_| {
        let mut v = envelope("found");
        v["detail"]["logbook_revision"] = json!(11);
        v["detail"]["contact_revision"] = json!(4);
        wire(200, v)
    });
    let (mut bridge, server) = serve(vec![high, lower_contact, reply("found")], true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let lease = read(&mut client, ID).unwrap();
    assert!(matches!(
        read(&mut client, ID),
        Err(ContactReadError::Bridge(Error::Revision))
    ));
    assert!(serde_json::to_value(lease).is_err());
    assert!(matches!(
        read(&mut client, ID),
        Err(ContactReadError::Bridge(Error::Revision))
    ));
    server.join().unwrap();
}
#[test]
fn http_timeout_uses_remaining_capture_budget_and_keeps_other_session_lanes_alive() {
    let (mut bridge, server) = serve(
        vec![Box::new(|_| {
            thread::sleep(Duration::from_millis(700));
            wire(200, envelope("found"))
        })],
        true,
    );
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let mut request = client.capture(TOKEN, ID).unwrap();
    request.started -= Duration::from_millis(4700);
    assert!(matches!(
        client.read(&request),
        Err(ContactReadError::Bridge(Error::Transport))
    ));
    assert!(bridge.hello().is_some());
    server.join().unwrap();
}
#[test]
fn transport_does_not_accept_history_style_clock_skew_or_old_owner_read_start() {
    let replies: Vec<Reply> = vec![
        Box::new(|_| {
            let mut v = envelope("found");
            v["generated_at_unix_ms"] = json!(now_ms().unwrap() + 1000);
            wire(200, v)
        }),
        Box::new(|_| {
            let mut v = envelope("found");
            v["generated_at_unix_ms"] = json!(0);
            wire(200, v)
        }),
    ];
    let (mut bridge, server) = serve(replies, true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    for _ in 0..2 {
        assert!(matches!(
            read(&mut client, ID),
            Err(ContactReadError::Bridge(Error::Stale))
        ));
    }
    assert!(bridge.hello().is_some());
    server.join().unwrap();
}

#[test]
fn failed_clock_or_expiry_is_terminal_only_for_that_retained_operation() {
    for failure in 0..4 {
        let (mut bridge, server) = serve(vec![reply("found"), reply("found")], true);
        bridge.connect().unwrap();
        let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
        let old = read(&mut client, ID).unwrap();
        let wall = match failure {
            0 => Err(Error::Invalid),
            1 => Ok(old.receipt_wall - 1),
            2 => Ok(old.payload.generated_at_unix_ms + 5000),
            _ => Ok(old.receipt_wall),
        };
        let instant = if failure == 3 {
            old.expires
        } else {
            Instant::now()
        };
        assert!(old.check_at(wall, instant).is_err());
        assert!(!old.facts.load(Ordering::Acquire));
        // Restoring the clock cannot resurrect the old operation.
        assert!(old.check_at(Ok(old.receipt_wall), Instant::now()).is_err());
        assert!(serde_json::to_value(&old).is_err());
        let fresh = read(&mut client, ID).unwrap();
        assert!(fresh.check().is_ok());
        // An old callback cannot retire the new read's distinct facts Arc.
        assert!(old.check_at(Err(Error::Invalid), Instant::now()).is_err());
        assert!(serde_json::to_value(&fresh).is_ok());
        server.join().unwrap();
    }
}
#[test]
fn failed_new_selection_clock_retires_previous_request_before_clock_is_read() {
    let (mut bridge, server) = serve(vec![reply("found"), reply("found")], true);
    bridge.connect().unwrap();
    let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
    let old = read(&mut client, ID).unwrap();
    let fence = client.fence();
    assert!(matches!(
        fence.capture_clock(TOKEN, "00000000-0000-4000-8000-000000000999", || {
            assert!(!old.request_live.load(Ordering::Acquire));
            Err(Error::Invalid)
        }),
        Err(Error::Invalid)
    ));
    assert!(serde_json::to_value(&old).is_err());
    let fresh = read(&mut client, ID).unwrap();
    assert!(fresh.value().is_ok());
    // Wrong-token calls are refused before they can retire current authority.
    assert!(fence
        .capture_clock("foreign-token", ID, || panic!("must not read clock"))
        .is_err());
    assert!(fresh.value().is_ok());
    server.join().unwrap();
}
#[test]
fn final_serializer_callback_cannot_return_a_retired_lease() {
    struct EndWriter {
        bytes: Vec<u8>,
        fence: ContactReadFence,
        revoke: bool,
        armed: bool,
        triggered: bool,
    }
    impl Write for EndWriter {
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
        let (mut bridge, server) = serve(vec![reply("found")], true);
        bridge.connect().unwrap();
        let mut client = bridge.contact_detail_handle(TOKEN).unwrap();
        let lease = read(&mut client, ID).unwrap();
        let mut writer = EndWriter {
            bytes: vec![],
            fence: client.fence(),
            revoke,
            armed: false,
            triggered: false,
        };
        let result = lease.serialize(&mut serde_json::Serializer::new(&mut writer));
        assert!(
            writer.triggered,
            "must reach actual final object writer callback"
        );
        assert_eq!(result.is_err(), revoke);
        let completed: Value = serde_json::from_slice(&writer.bytes).unwrap();
        assert_eq!(completed["payload"]["detail"]["contact_id"], ID);
        assert_eq!(lease.value().is_err(), revoke);
        server.join().unwrap();
    }
}
