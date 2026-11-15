// SPDX-License-Identifier: GPL-3.0-only
//! Actual SDK + native lane against synthetic loopback; no Tauri app or owner DB.
use super::*;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const TOKEN: &str = "synthetic-native-connection-token";
const ID: &str = "abcdef12-3456-4abc-8def-123456abcdef";
type Reply = Box<dyn FnOnce(&str) -> (u16, Value) + Send>;
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn corpus(name: &str) -> Value {
    let corpus: Value = serde_json::from_slice(include_bytes!(
        "../../integrations/xlog/fixtures/contact-detail-swift.json"
    ))
    .unwrap();
    corpus[name].clone()
}
fn envelope(name: &str) -> Value {
    json!({"protocol_version":1,"server_instance":"native-detail-test","workspace_id":"native-workspace","generation":"native-workspace","worker_generation":1,"generated_at_unix_ms":now(),"max_age_ms":5000,"detail":corpus(name)})
}
fn reply(name: &'static str) -> Reply {
    Box::new(move |_| (200, envelope(name)))
}
fn serve(
    mut replies: Vec<Reply>,
    capability: bool,
) -> (BridgeClient, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    replies.insert(0,Box::new(move |_|(200,json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":"native-detail-test","workspace_id":"native-workspace","capabilities":if capability {vec!["snapshot.read","logbook.contact.read"]} else {vec!["snapshot.read"]}}))));
    let server = thread::spawn(move || {
        let mut requests = vec![];
        for reply in replies {
            let until = Instant::now() + Duration::from_secs(5);
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(Instant::now() < until);
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(e) => panic!("{e}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = vec![];
            let mut byte = [0];
            while !bytes.ends_with(b"\r\n\r\n") {
                assert!(bytes.len() < 16384);
                stream.read_exact(&mut byte).unwrap();
                bytes.push(byte[0]);
            }
            let request = String::from_utf8(bytes).unwrap();
            let (status, value) = reply(&request);
            let body = value.to_string();
            requests.push(request);
            let _=write!(stream,"HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
        }
        requests
    });
    let mut client =
        BridgeClient::new(&endpoint, "synthetic-native-detail-auth-marker-000000").unwrap();
    client.connect().unwrap();
    (client, server)
}
fn read(lane: &Lane, id: &str) -> Result<Response, String> {
    let prepared = lane.capture(TOKEN, id)?;
    lane.read(prepared)
}
#[test]
fn actual_producer_payloads_keep_exact_identity_nulls_status_and_original_lease_shape() {
    let (client, server) = serve(
        vec![reply("found"), reply("deleted"), reply("not_found")],
        true,
    );
    let lane = Lane::new(&client, TOKEN).unwrap();
    for name in ["found", "deleted", "not_found"] {
        let expected = corpus(name);
        let response = read(&lane, expected["contact_id"].as_str().unwrap()).unwrap();
        let value = serde_json::to_value(response).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 3);
        assert_eq!(value["connection_token"], TOKEN);
        assert_eq!(value["payload"]["detail"], expected);
        assert!((1..=5000).contains(&value["remaining_valid_ms"].as_u64().unwrap()));
    }
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 4);
    for request in &requests[1..] {
        assert!(request.starts_with("GET /xlog/v1/logbook/contacts/"));
        assert!(!request.contains(TOKEN));
        assert!(!request.to_lowercase().contains("origin:"));
    }
}
#[test]
fn unsupported_invalid_selection_and_wrong_token_do_not_start_http() {
    let (client, server) = serve(vec![], false);
    let lane = Lane::new(&client, TOKEN).unwrap();
    assert!(matches!(lane.capture(TOKEN,ID),Err(error) if error.contains("unsupported")));
    server.join().unwrap();
    let (client, server) = serve(vec![], true);
    let lane = Lane::new(&client, TOKEN).unwrap();
    assert!(matches!(lane.capture("old-token",ID),Err(error) if error=="Connection changed"));
    assert!(lane.capture(TOKEN, &ID.to_uppercase()).is_err());
    server.join().unwrap();
}
#[test]
fn busy_capture_retires_old_selection_without_scheduling_another_http_job() {
    let (client, server) = serve(vec![reply("found")], true);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let first = lane.capture(TOKEN, ID).unwrap();
    assert!(lane.busy.load(Ordering::Acquire));
    assert!(
        matches!(lane.capture(TOKEN,"00000000-0000-4000-8000-000000000999"),Err(error) if error=="xLog contact detail is busy")
    );
    assert!(lane.read(first).is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    assert!(read(&lane, ID).is_ok());
    assert_eq!(server.join().unwrap().len(), 2);
}
#[test]
fn cancelled_prepared_request_releases_admission_without_any_http() {
    let (client, server) = serve(vec![reply("found")], true);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let prepared = lane.capture(TOKEN, ID).unwrap();
    drop(prepared);
    assert!(!lane.busy.load(Ordering::Acquire));
    assert!(read(&lane, ID).is_ok());
    assert_eq!(server.join().unwrap().len(), 2);
}
#[test]
fn delayed_inflight_reply_is_invalidated_by_busy_new_selection() {
    let (tx, rx) = std::sync::mpsc::channel();
    let (go, wait) = std::sync::mpsc::channel();
    let (client, server) = serve(
        vec![
            Box::new(move |_| {
                tx.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(2)).unwrap();
                (200, envelope("found"))
            }),
            reply("found"),
        ],
        true,
    );
    let lane = Arc::new(Lane::new(&client, TOKEN).unwrap());
    let prepared = lane.capture(TOKEN, ID).unwrap();
    let worker = lane.clone();
    let job = thread::spawn(move || worker.read(prepared));
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(
        matches!(lane.capture(TOKEN,"00000000-0000-4000-8000-000000000999"),Err(error) if error.contains("busy"))
    );
    go.send(()).unwrap();
    assert!(job.join().unwrap().is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    assert!(read(&lane, ID).is_ok());
    server.join().unwrap();
}
#[test]
fn retirement_during_http_is_immediate_with_retained_lane_and_client() {
    let (tx, rx) = std::sync::mpsc::channel();
    let (go, wait) = std::sync::mpsc::channel();
    let (client, server) = serve(
        vec![Box::new(move |_| {
            tx.send(()).unwrap();
            wait.recv_timeout(Duration::from_secs(2)).unwrap();
            (200, envelope("found"))
        })],
        true,
    );
    let lane = Arc::new(Lane::new(&client, TOKEN).unwrap());
    let prepared = lane.capture(TOKEN, ID).unwrap();
    let worker = lane.clone();
    let job = thread::spawn(move || worker.read(prepared));
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    lane.revoke();
    assert!(!lane.scope.live.load(Ordering::Acquire));
    assert!(lane.busy.load(Ordering::Acquire));
    go.send(()).unwrap();
    assert!(job.join().unwrap().is_err());
    assert!(lane.capture(TOKEN, ID).is_err());
    server.join().unwrap();
}
#[test]
fn retained_response_cannot_serialize_after_native_or_sdk_retirement() {
    for native in [true, false] {
        let (mut client, server) = serve(vec![reply("found")], true);
        let lane = Lane::new(&client, TOKEN).unwrap();
        let response = read(&lane, ID).unwrap();
        if native {
            lane.revoke();
        } else {
            client.disconnect();
        }
        assert!(serde_json::to_value(response).is_err());
        server.join().unwrap();
    }
}
#[test]
fn typed_unavailable_and_stable_busy_error_never_turn_into_empty_success() {
    let (client, server) = serve(
        vec![
            Box::new(|_| {
                let mut value = envelope("deleted");
                value["detail"]["status"] = json!("unavailable");
                value["detail"]["reason"] = json!("detail_too_large");
                value["detail"]["editability"]["reason"] = json!("detail_too_large");
                (503, value)
            }),
            Box::new(|_| (503, json!({"error":"contact_detail_busy"}))),
        ],
        true,
    );
    let lane = Lane::new(&client, TOKEN).unwrap();
    let response = serde_json::to_value(read(&lane, ID).unwrap()).unwrap();
    assert_eq!(response["payload"]["detail"]["status"], "unavailable");
    assert!(response["payload"]["detail"]["fields"].is_null());
    assert!(
        matches!(read(&lane,ID),Err(error) if error=="xLog contact detail unavailable (contact_detail_busy)")
    );
    assert!(client.hello().is_some());
    server.join().unwrap();
}
#[test]
fn detail_lane_uses_no_ordinary_client_lock_and_fails_its_own_lock_busy() {
    let (client, server) = serve(vec![reply("found")], true);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let ordinary = Mutex::new(client);
    let _ordinary_guard = ordinary.lock().unwrap();
    let guard = lane.client.lock().unwrap();
    let prepared = lane.capture(TOKEN, ID).unwrap();
    assert!(matches!(lane.read(prepared),Err(error) if error.contains("busy")));
    assert!(!lane.busy.load(Ordering::Acquire));
    drop(guard);
    assert!(read(&lane, ID).is_ok());
    server.join().unwrap();
}
#[test]
fn prepared_request_from_another_native_lane_cannot_use_matching_token() {
    let (client, server) = serve(vec![], true);
    let first = Lane::new(&client, TOKEN).unwrap();
    let second = Lane::new(&client, TOKEN).unwrap();
    let prepared = first.capture(TOKEN, ID).unwrap();
    assert!(matches!(second.read(prepared),Err(error) if error=="Connection changed"));
    assert!(!first.busy.load(Ordering::Acquire));
    server.join().unwrap();
}
#[test]
fn native_scope_is_rechecked_after_the_final_serializer_callback() {
    struct Writer {
        scope: Arc<Scope>,
        armed: bool,
        fired: bool,
        revoke: bool,
    }
    impl Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes == b"remaining_valid_ms" {
                self.armed = true;
            }
            if self.armed && bytes == b"}" {
                self.fired = true;
                if self.revoke {
                    self.scope.live.store(false, Ordering::Release);
                }
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    for revoke in [false, true] {
        let (client, server) = serve(vec![reply("found")], true);
        let lane = Lane::new(&client, TOKEN).unwrap();
        let response = read(&lane, ID).unwrap();
        let mut writer = Writer {
            scope: response.scope.clone(),
            armed: false,
            fired: false,
            revoke,
        };
        let result = response.serialize(&mut serde_json::Serializer::new(&mut writer));
        assert!(writer.fired);
        assert_eq!(result.is_err(), revoke);
        server.join().unwrap();
    }
}
#[test]
fn construction_guard_binds_validated_setup_and_immediate_lifecycle_retirement() {
    let source = include_str!("xlog_mode.rs");
    let setup = source
        .split("fn establish(")
        .nth(1)
        .unwrap()
        .split("#[tauri::command]")
        .next()
        .unwrap();
    let validated = setup.find("let snapshot = client.poll()").unwrap();
    let factory = setup
        .find("passive_contact_detail::Lane::new(&client, history.token())")
        .unwrap();
    let stored = setup.find("session.connection = Some").unwrap();
    assert!(validated < factory && factory < stored);
    let begin = source
        .split("fn begin(")
        .nth(1)
        .unwrap()
        .split("fn establish(")
        .next()
        .unwrap();
    assert!(
        begin.find("connection.detail.revoke()").unwrap()
            < begin.find("session.connection = None").unwrap()
    );
    let poll = source
        .split("async fn xlog_poll(")
        .nth(1)
        .unwrap()
        .split("#[tauri::command]")
        .next()
        .unwrap();
    assert!(
        poll.find("connection.detail.revoke()").unwrap()
            < poll.find("session.connection = None").unwrap()
    );
    assert_eq!(
        source.matches("passive_contact_detail::Lane::new").count(),
        1
    );
}
#[test]
fn construction_guard_captures_before_spawn_and_keeps_http_and_serialization_outside_global_locks()
{
    let source = include_str!("xlog_mode.rs");
    let command = source
        .split("async fn xlog_contact_detail_read(")
        .nth(1)
        .unwrap()
        .split("type Queue =")
        .next()
        .unwrap();
    assert!(
        command.find("connection.detail.capture").unwrap()
            < command.find("spawn_blocking").unwrap()
    );
    assert_eq!(command.matches("check_read_connection(").count(), 2);
    assert!(command.contains("connection.detail.read(prepared)"));
    for prohibited in [
        "lock(&connection.client)",
        "lock(&shared)",
        ".stop",
        "serde_json::",
        "Envelope",
        "lazy",
    ] {
        assert!(!command.contains(prohibited), "{prohibited}");
    }
    let registration = source
        .split(".invoke_handler(tauri::generate_handler![")
        .nth(1)
        .unwrap();
    assert!(registration.contains("xlog_contact_detail_read,"));
    let lane = include_str!("xlog_contact_detail_read.rs");
    let compact: String = lane.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(compact.contains("self.client.try_lock()"));
    assert!(lane.contains("self.lease.serialize(serializer)"));
    assert!(!compact.contains("Envelope{"));
    assert!(!compact.contains("serde_json::to_value("));
}
