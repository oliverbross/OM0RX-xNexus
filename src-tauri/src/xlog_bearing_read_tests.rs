// SPDX-License-Identifier: GPL-3.0-only
//! Actual SDK and native lane, synthetic loopback envelope with actual Swift bodies.
use super::*;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::TcpListener,
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const TOKEN: &str = "synthetic-native-bearing-context";
const SERVER: &str = "00000000-0000-4000-8000-000000000001";
const WORKSPACE: &str = "00000000-0000-4000-8000-000000000002";
type Reply = Box<dyn FnOnce(&str) -> (u16, Value) + Send>;
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn body(name: &str) -> Value {
    let c: Value = serde_json::from_slice(include_bytes!(
        "../../integrations/xlog/fixtures/bearing-swift-v1.json"
    ))
    .unwrap();
    c[name].clone()
}
fn binding() -> Value {
    json!({"workspace_generation":WORKSPACE,"worker_generation":7,"settings_revision":11,"identity_epoch":"00000000-0000-4000-8000-000000000003","identity_revision":3,"operating_module":"normal_logging"})
}
fn snapshot(grid: &str, max_age: u64) -> Value {
    json!({"protocol_version":1,"server_instance":SERVER,"workspace_id":WORKSPACE,"revision":5,"generated_at_unix_ms":now(),"max_age_ms":max_age,"station":{"connected":true,"radio_id":"synthetic-radio","frequency_hz":14074000,"mode":"USB","transmitting":false},"rotor":{"connected":true,"rotor_id":"synthetic-exact-rotor","azimuth_deg":123.0,"moving":false},"spots":[],"station_identity":{"schema_version":1,"source":"saved_station_settings","state":"current","mycall":"OM0RX","mygrid":grid,"binding":binding(),"transmit_authority":false}})
}
fn hello(capability: bool) -> Value {
    json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":SERVER,"workspace_id":WORKSPACE,"capabilities":if capability{vec!["snapshot.read","bearing.read","rotor.stop"]}else{vec!["snapshot.read"]}})
}
fn envelope(name: &str) -> Value {
    json!({"protocol_version":1,"server_instance":SERVER,"workspace_id":WORKSPACE,"generation":WORKSPACE,"worker_generation":7,"station_identity_binding":binding(),"generated_at_unix_ms":now(),"max_age_ms":5000,"bearing":body(name)})
}
fn reply(name: &'static str) -> Reply {
    Box::new(move |_| {
        (
            if name == "historical_grid" || name == "cty_approximation" {
                200
            } else {
                503
            },
            envelope(name),
        )
    })
}
fn receive(stream: &mut std::net::TcpStream) -> String {
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut bytes = vec![];
    let mut byte = [0];
    while !bytes.ends_with(b"\r\n\r\n") {
        assert!(bytes.len() < 16384);
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
    }
    let header = String::from_utf8(bytes.clone()).unwrap();
    let count = header
        .lines()
        .find_map(|line| {
            line.to_lowercase()
                .strip_prefix("content-length:")
                .map(|v| v.trim().parse::<usize>().unwrap())
        })
        .unwrap_or(0);
    assert!(count <= 65536);
    let mut body = vec![0; count];
    stream.read_exact(&mut body).unwrap();
    bytes.extend_from_slice(&body);
    String::from_utf8(bytes).unwrap()
}
fn send(stream: &mut std::net::TcpStream, status: u16, value: Value) {
    let body = value.to_string();
    let _=write!(stream,"HTTP/1.1 {status} Synthetic\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
}
fn accept(listener: &TcpListener) -> std::net::TcpStream {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((s, _)) => return s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < until, "expected request missing");
                thread::sleep(Duration::from_millis(2));
            }
            Err(e) => panic!("{e}"),
        }
    }
}
fn serve(
    mut replies: Vec<Reply>,
    capability: bool,
    grid: &str,
    max_age: u64,
) -> (BridgeClient, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let first = snapshot(grid, max_age);
    replies.insert(0, Box::new(move |_| (200, first)));
    replies.insert(0, Box::new(move |_| (200, hello(capability))));
    let server = thread::spawn(move || {
        let mut requests = vec![];
        for reply in replies {
            let mut s = accept(&listener);
            let request = receive(&mut s);
            let (status, value) = reply(&request);
            requests.push(request);
            send(&mut s, status, value);
        }
        requests
    });
    let mut client =
        BridgeClient::new(&endpoint, "synthetic-native-bearing-auth-marker-000000").unwrap();
    client.connect().unwrap();
    client.poll().unwrap();
    (client, server)
}
fn grid() -> String {
    body("historical_grid")["origin"]["grid"]
        .as_str()
        .unwrap()
        .into()
}
fn capture(lane: &Lane, client: &BridgeClient, call: &str) -> Result<Prepared, String> {
    lane.capture(TOKEN, call, || {
        client.bearing_context().map_err(|e| e.to_string())
    })
}
fn read(lane: &Lane, client: &BridgeClient, call: &str) -> Result<Response, String> {
    lane.read(capture(lane, client, call)?)
}

#[test]
fn actual_four_producer_bodies_keep_exact_calls_binding_and_sdk_lease_shape() {
    for name in [
        "historical_grid",
        "cty_approximation",
        "coincident",
        "reference_unavailable",
    ] {
        let expected = body(name);
        let (client, server) = serve(
            vec![reply(name)],
            true,
            expected["origin"]["grid"].as_str().unwrap(),
            3000,
        );
        let lane = Lane::new(&client, TOKEN).unwrap();
        let call = expected["requested_callsign"].as_str().unwrap();
        let value = serde_json::to_value(read(&lane, &client, call).unwrap()).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 3);
        assert_eq!(value["connection_token"], TOKEN);
        assert_eq!(value["payload"]["bearing"]["requested_callsign"], call);
        assert_eq!(value["payload"]["bearing"]["status"], expected["status"]);
        assert_eq!(value["payload"]["station_identity_binding"], binding());
        assert_eq!(
            value["payload"]["bearing"]["target"]["contact_started_at_unix_ms"],
            expected["target"]["contact_started_at_unix_ms"]
        );
        assert!((1..=3000).contains(&value["remaining_valid_ms"].as_u64().unwrap()));
        let requests = server.join().unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[2].starts_with(&format!(
            "GET /xlog/v1/bearing?callsign={} HTTP/1.1",
            call.replace('/', "%2F")
        )));
        assert!(!requests[2].contains(TOKEN));
        assert!(!requests[2].to_lowercase().contains("origin:"));
    }
}
#[test]
fn unsupported_grant_does_not_break_setup_or_invoke_context_lookup() {
    let (client, server) = serve(vec![], false, &grid(), 3000);
    let lane = Lane::new(&client, TOKEN).unwrap();
    assert!(
        matches!(lane.capture(TOKEN,"VK8RX/P",||panic!("unsupported must not inspect context")),Err(e)if e.contains("unsupported"))
    );
    assert!(client.hello().is_some());
    server.join().unwrap();
}
#[test]
fn wrong_token_preserves_current_response_but_invalid_call_retires_it() {
    let (client, server) = serve(vec![reply("historical_grid")], true, &grid(), 3000);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let old = read(&lane, &client, "VK8RX/P").unwrap();
    assert!(lane
        .capture("old-token", "VK8RX/P", || panic!("wrong token"))
        .is_err());
    assert!(serde_json::to_value(&old).is_ok());
    assert!(capture(&lane, &client, " VK8RX/P").is_err());
    assert!(serde_json::to_value(&old).is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    assert_eq!(server.join().unwrap().len(), 3);
}
#[test]
fn main_context_trylock_busy_retires_response_before_lookup_without_waiting() {
    let (client, server) = serve(vec![reply("historical_grid")], true, &grid(), 3000);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let old = read(&lane, &client, "VK8RX/P").unwrap();
    let main = Mutex::new(client);
    let _held = main.lock().unwrap();
    let result = lane.capture(TOKEN, "W1AW", || {
        assert!(old.selection.check().is_err());
        let client = main.try_lock().map_err(|_| "context busy".to_string())?;
        client.bearing_context().map_err(|e| e.to_string())
    });
    assert!(matches!(result,Err(e)if e=="context busy"));
    assert!(serde_json::to_value(&old).is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    server.join().unwrap();
}
#[test]
fn context_failure_retires_old_response_and_next_valid_selection_recovers() {
    let (client, server) = serve(
        vec![reply("historical_grid"), reply("historical_grid")],
        true,
        &grid(),
        3000,
    );
    let lane = Lane::new(&client, TOKEN).unwrap();
    let old = read(&lane, &client, "VK8RX/P").unwrap();
    assert!(lane
        .capture(TOKEN, "W1AW", || Err("snapshot expired".into()))
        .is_err());
    assert!(serde_json::to_value(&old).is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    let fresh = read(&lane, &client, "VK8RX/P").unwrap();
    old.selection.live.store(false, Ordering::Release);
    assert!(serde_json::to_value(&fresh).is_ok());
    server.join().unwrap();
}
#[test]
fn queued_busy_selection_is_retired_without_second_http_job() {
    let (client, server) = serve(vec![reply("historical_grid")], true, &grid(), 3000);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let old = capture(&lane, &client, "VK8RX/P").unwrap();
    assert!(
        matches!(lane.capture(TOKEN,"W1AW",||panic!("busy before context")),Err(e)if e=="xLog bearing is busy")
    );
    assert!(lane.read(old).is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    assert!(read(&lane, &client, "VK8RX/P").is_ok());
    assert_eq!(server.join().unwrap().len(), 3);
}
#[test]
fn cancelled_prepared_read_releases_permit_without_dispatch() {
    let (client, server) = serve(vec![reply("historical_grid")], true, &grid(), 3000);
    let lane = Lane::new(&client, TOKEN).unwrap();
    drop(capture(&lane, &client, "VK8RX/P").unwrap());
    assert!(!lane.busy.load(Ordering::Acquire));
    assert!(read(&lane, &client, "VK8RX/P").is_ok());
    server.join().unwrap();
}
#[test]
fn original_snapshot_expiry_while_queued_prevents_http() {
    let (client, server) = serve(vec![], true, &grid(), 1000);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let prepared = capture(&lane, &client, "VK8RX/P").unwrap();
    thread::sleep(Duration::from_millis(1010));
    assert!(lane.read(prepared).is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    assert_eq!(server.join().unwrap().len(), 2);
}
#[test]
fn inflight_busy_selection_retires_late_reply_and_does_not_build_a_queue() {
    let (tx, rx) = mpsc::channel();
    let (go, wait) = mpsc::channel();
    let (client, server) = serve(
        vec![
            Box::new(move |_| {
                tx.send(()).unwrap();
                wait.recv_timeout(Duration::from_secs(2)).unwrap();
                (200, envelope("historical_grid"))
            }),
            reply("historical_grid"),
        ],
        true,
        &grid(),
        3000,
    );
    let lane = Arc::new(Lane::new(&client, TOKEN).unwrap());
    let prepared = capture(&lane, &client, "VK8RX/P").unwrap();
    let worker = lane.clone();
    let job = thread::spawn(move || worker.read(prepared));
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(lane
        .capture(TOKEN, "W1AW", || panic!("no queued context"))
        .is_err());
    go.send(()).unwrap();
    assert!(job.join().unwrap().is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    let fresh = read(&lane, &client, "VK8RX/P").unwrap();
    assert!(serde_json::to_value(fresh).is_ok());
    assert_eq!(server.join().unwrap().len(), 4);
}
#[test]
fn read_and_serialize_do_not_hold_or_acquire_ordinary_client_lock() {
    let (client, server) = serve(vec![reply("historical_grid")], true, &grid(), 3000);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let prepared = capture(&lane, &client, "VK8RX/P").unwrap();
    let main = Mutex::new(client);
    let _main_guard = main.lock().unwrap();
    let response = lane.read(prepared).unwrap();
    assert!(serde_json::to_value(response).is_ok());
    server.join().unwrap();
}
#[test]
fn own_reader_busy_and_foreign_prepared_scope_fail_without_dispatch() {
    let (client, server) = serve(vec![], true, &grid(), 3000);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let other = Lane::new(&client, TOKEN).unwrap();
    let prepared = capture(&lane, &client, "VK8RX/P").unwrap();
    assert!(other.read(prepared).is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    let prepared = capture(&lane, &client, "VK8RX/P").unwrap();
    let _held = lane.client.lock().unwrap();
    assert!(lane.read(prepared).is_err());
    assert!(!lane.busy.load(Ordering::Acquire));
    server.join().unwrap();
}
#[test]
fn epoch_overflow_and_scope_retirement_are_terminal_without_waiting_for_http() {
    let (client, server) = serve(vec![reply("historical_grid")], true, &grid(), 3000);
    let lane = Lane::new(&client, TOKEN).unwrap();
    let response = read(&lane, &client, "VK8RX/P").unwrap();
    lane.selections.lock().unwrap().sequence = u64::MAX;
    assert!(capture(&lane, &client, "W1AW").is_err());
    assert!(serde_json::to_value(response).is_err());
    assert!(lane.scope.check(TOKEN).is_err());
    server.join().unwrap();
}
#[test]
fn typed_unavailable_stays_displayable_but_plain_503_is_only_error() {
    let (client, server) = serve(
        vec![
            reply("reference_unavailable"),
            Box::new(|_| (503, json!({"error":"bearing_busy"}))),
        ],
        true,
        body("reference_unavailable")["origin"]["grid"]
            .as_str()
            .unwrap(),
        3000,
    );
    let lane = Lane::new(&client, TOKEN).unwrap();
    let old = read(&lane, &client, "VK8RX").unwrap();
    let v = serde_json::to_value(&old).unwrap();
    assert_eq!(v["payload"]["bearing"]["status"], "unavailable");
    assert!(v["payload"]["bearing"]["geometry"].is_null());
    assert!(
        matches!(read(&lane,&client,"VK8RX"),Err(e)if e=="xLog bearing unavailable (bearing_busy)")
    );
    assert!(serde_json::to_value(&old).is_err());
    assert!(client.hello().is_some());
    server.join().unwrap();
}
#[test]
fn serialization_rechecks_native_scope_and_selection_after_last_writer_callback() {
    struct Writer {
        scope: Arc<Scope>,
        selection: Arc<Selection>,
        revoke: u8,
        armed: bool,
        fired: bool,
    }
    impl Write for Writer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            if bytes == b"remaining_valid_ms" {
                self.armed = true;
            }
            if self.armed && bytes == b"}" {
                self.fired = true;
                match self.revoke {
                    1 => self.scope.live.store(false, Ordering::Release),
                    2 => self.selection.live.store(false, Ordering::Release),
                    _ => (),
                }
            }
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    for revoke in 0..3 {
        let (client, server) = serve(vec![reply("historical_grid")], true, &grid(), 3000);
        let lane = Lane::new(&client, TOKEN).unwrap();
        let response = read(&lane, &client, "VK8RX/P").unwrap();
        let mut writer = Writer {
            scope: response.scope.clone(),
            selection: response.selection.clone(),
            revoke,
            armed: false,
            fired: false,
        };
        let result = response.serialize(&mut serde_json::Serializer::new(&mut writer));
        assert!(writer.fired);
        assert_eq!(result.is_err(), revoke != 0);
        server.join().unwrap();
    }
}
#[test]
fn native_retirement_and_aba_never_retarget_old_selection() {
    let (client, server) = serve(
        vec![reply("historical_grid"), reply("historical_grid")],
        true,
        &grid(),
        3000,
    );
    let lane = Lane::new(&client, TOKEN).unwrap();
    let old = read(&lane, &client, "VK8RX/P").unwrap();
    assert!(lane
        .capture(TOKEN, "W1AW", || Err("context unavailable".into()))
        .is_err());
    let fresh = read(&lane, &client, "VK8RX/P").unwrap();
    assert!(serde_json::to_value(&old).is_err());
    old.selection.live.store(false, Ordering::Release);
    assert!(serde_json::to_value(&fresh).is_ok());
    lane.revoke();
    assert!(serde_json::to_value(&fresh).is_err());
    assert!(capture(&lane, &client, "VK8RX/P").is_err());
    server.join().unwrap();
}

#[test]
fn actual_main_poll_identity_changes_or_failure_retire_retained_native_response() {
    for case in ["grid", "worker", "missing", "failure"] {
        let next: Reply = Box::new(move |_| {
            if case == "failure" {
                return (503, json!({"error":"snapshot_unavailable"}));
            }
            let mut value = snapshot(&grid(), 3000);
            value["revision"] = json!(6);
            match case {
                "grid" => value["station_identity"]["mygrid"] = json!("JN99"),
                "worker" => value["station_identity"]["binding"]["worker_generation"] = json!(8),
                "missing" => value
                    .as_object_mut()
                    .unwrap()
                    .remove("station_identity")
                    .map(|_| ())
                    .unwrap(),
                _ => unreachable!(),
            }
            (200, value)
        });
        let (mut client, server) = serve(vec![reply("historical_grid"), next], true, &grid(), 3000);
        let lane = Lane::new(&client, TOKEN).unwrap();
        let response = read(&lane, &client, "VK8RX/P").unwrap();
        assert!(serde_json::to_value(&response).is_ok());
        assert_eq!(client.poll().is_err(), case == "failure");
        assert!(serde_json::to_value(&response).is_err(), "{case}");
        assert_eq!(server.join().unwrap().len(), 4);
    }
}

#[test]
fn actual_independent_sdk_stop_completes_while_bearing_http_and_main_lock_are_held() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (started_tx, started_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut requests = vec![];
        for value in [hello(true), snapshot(&grid(), 3000)] {
            let mut socket = accept(&listener);
            requests.push(receive(&mut socket));
            send(&mut socket, 200, value);
        }
        let mut bearing_socket = accept(&listener);
        let bearing_request = receive(&mut bearing_socket);
        assert!(bearing_request.starts_with("GET /xlog/v1/bearing?"));
        requests.push(bearing_request);
        started_tx.send(()).unwrap();
        let mut stop_socket = accept(&listener);
        let stop_request = receive(&mut stop_socket);
        assert!(stop_request.starts_with("POST /xlog/v1/command "));
        let command: Value =
            serde_json::from_str(stop_request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(command["command"]["kind"], "rotor.stop");
        assert_eq!(command["command"]["rotor_id"], "synthetic-exact-rotor");
        assert_eq!(command["expected_revision"], 0);
        send(
            &mut stop_socket,
            200,
            json!({"request_id":command["request_id"],"server_instance":SERVER,"workspace_id":WORKSPACE,"status":"accepted"}),
        );
        requests.push(stop_request);
        release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        send(&mut bearing_socket, 200, envelope("historical_grid"));
        requests
    });
    let mut client =
        BridgeClient::new(&endpoint, "synthetic-native-bearing-auth-marker-000000").unwrap();
    client.connect().unwrap();
    client.poll().unwrap();
    let stop = client.stop_handle().unwrap();
    let lane = Arc::new(Lane::new(&client, TOKEN).unwrap());
    let prepared = capture(&lane, &client, "VK8RX/P").unwrap();
    let main = Mutex::new(client);
    let _main_guard = main.lock().unwrap();
    let worker = lane.clone();
    let job = thread::spawn(move || worker.read(prepared));
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    let receipt = stop
        .stop_rotor(
            "00000000-0000-4000-8000-000000000004",
            "synthetic-exact-rotor",
        )
        .unwrap();
    assert_eq!(receipt.status, xnexus_xlog_client::CommandStatus::Accepted);
    assert!(
        !job.is_finished(),
        "bearing remains deliberately blocked until Stop completes"
    );
    lane.revoke();
    release_tx.send(()).unwrap();
    assert!(job.join().unwrap().is_err());
    assert_eq!(server.join().unwrap().len(), 4);
}

#[test]
fn construction_guard_captures_before_spawn_uses_trylock_and_keeps_retained_lease() {
    let source = include_str!("xlog_mode.rs");
    let command = source
        .split("async fn xlog_bearing_read(")
        .nth(1)
        .unwrap()
        .split("#[tauri::command]")
        .next()
        .unwrap();
    let compact: String = command.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        compact.find("connection.bearing.capture").unwrap()
            < compact.find("connection.client.try_lock()").unwrap()
    );
    assert!(
        command.find("client.bearing_context()").unwrap() < command.find("spawn_blocking").unwrap()
    );
    assert_eq!(command.matches("check_read_connection(").count(), 2);
    assert!(command.contains("connection.bearing.read(prepared)"));
    for denied in [
        "lock(&connection.client)",
        "lock(&shared)",
        ".poll(",
        ".stop",
        "serde_json::",
        "Envelope",
        ".command(",
    ] {
        assert!(!command.contains(denied), "{denied}");
    }
    assert!(source
        .split(".invoke_handler(tauri::generate_handler![")
        .nth(1)
        .unwrap()
        .contains("xlog_bearing_read,"));
    let lane = include_str!("xlog_bearing_read.rs");
    let capture = lane
        .split("pub(super) fn capture(")
        .nth(1)
        .unwrap()
        .split("pub(super) fn revoke(")
        .next()
        .unwrap();
    assert!(capture.find("self.select(expected)").unwrap() < capture.find("context()?").unwrap());
    assert!(lane.contains("self.lease.serialize(serializer)"));
    assert!(!lane.contains("serde_json::to_value"));
}
#[test]
fn construction_guard_setup_and_retirement_preserve_optional_capability_and_stop_path() {
    let source = include_str!("xlog_mode.rs");
    let setup = source
        .split("fn establish(")
        .nth(1)
        .unwrap()
        .split("#[tauri::command]")
        .next()
        .unwrap();
    assert!(
        setup.find("let snapshot = client.poll()").unwrap()
            < setup.find("passive_bearing::Lane::new").unwrap()
    );
    assert!(
        setup.find("passive_bearing::Lane::new").unwrap()
            < setup.find("session.connection = Some").unwrap()
    );
    for section in [
        source
            .split("fn begin(")
            .nth(1)
            .unwrap()
            .split("fn establish(")
            .next()
            .unwrap(),
        source
            .split("async fn xlog_poll(")
            .nth(1)
            .unwrap()
            .split("#[tauri::command]")
            .next()
            .unwrap(),
    ] {
        assert!(
            section.find("connection.bearing.revoke()").unwrap()
                < section.find("session.connection = None").unwrap()
        );
    }
    assert_eq!(source.matches("passive_bearing::Lane::new").count(), 1);
}
