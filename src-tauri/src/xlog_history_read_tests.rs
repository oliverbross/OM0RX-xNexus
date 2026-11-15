// SPDX-License-Identifier: GPL-3.0-only
//! Synthetic loopback tests of the actual SDK and native lease/lane boundary.
use super::*;
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn hello(capability: bool) -> Value {
    json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":"native-read-fixture",
        "workspace_id":"synthetic-workspace","capabilities":if capability {vec!["snapshot.read","logbook.read"]}else{vec!["snapshot.read"]}})
}
fn history() -> Value {
    json!({"protocol_version":1,"server_instance":"native-read-fixture","workspace_id":"synthetic-workspace",
        "callsign":"K1ABC","generated_at_unix_ms":now(),"contacts":[],"matrix":[]})
}
fn recent() -> Value {
    json!({"protocol_version":1,"server_instance":"native-read-fixture","workspace_id":"synthetic-workspace",
        "schema_version":1,"generation":"synthetic-workspace","logbook_revision":4,"max_age_ms":5000,
        "generated_at_unix_ms":now(),"contacts":[],"limit":5,"total":0,"complete":true,
        "ordering":"qso_start_desc_contact_id_asc"})
}
fn snapshot() -> Value {
    json!({"protocol_version":1,"server_instance":"native-read-fixture","workspace_id":"synthetic-workspace",
        "revision":1,"generated_at_unix_ms":now(),"max_age_ms":10000,
        "station":{"connected":false,"radio_id":null,"frequency_hz":null,"mode":null,"transmitting":null},
        "rotor":{"connected":false,"rotor_id":null,"azimuth_deg":null,"moving":null},"spots":[]})
}
fn accept(listener: &TcpListener) -> TcpStream {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(3)))
                    .unwrap();
                return stream;
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => panic!("fixture accept: {error}"),
        }
    }
}
fn request(stream: &mut TcpStream) -> String {
    let mut bytes = [0; 4096];
    let count = stream.read(&mut bytes).unwrap();
    String::from_utf8_lossy(&bytes[..count]).into_owned()
}
fn reply(stream: &mut TcpStream, status: u16, value: Value) {
    let body = value.to_string();
    write!(stream,"HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
}
fn fixture(replies: Vec<(u16, Value)>) -> (BridgeClient, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        replies
            .into_iter()
            .map(|(status, body)| {
                let mut stream = accept(&listener);
                let request = request(&mut stream);
                reply(&mut stream, status, body);
                request
            })
            .collect()
    });
    let mut client =
        BridgeClient::new(&endpoint, "synthetic-history-test-token-for-fixtures-only").unwrap();
    client.connect().unwrap();
    (client, server)
}
fn query() -> HistoryQuery {
    HistoryQuery::new("K1ABC", None, None).unwrap()
}

#[test]
fn valid_response_preserves_payload_and_bounds_lease_without_inventing_summary() {
    let (client, server) = fixture(vec![(200, hello(true)), (200, history())]);
    let lane = Lane::new(&client).unwrap();
    assert_eq!(lane.token().len(), 32);
    assert!(lane.history("wrong-token", &query()).is_err());
    let response = lane.history(lane.token(), &query()).unwrap();
    let wire = serde_json::to_value(&response).unwrap();
    assert_eq!(wire["connection_token"], lane.token());
    assert_eq!(wire["payload"]["callsign"], "K1ABC");
    assert!(wire["payload"].get("history_summary").is_none());
    assert!((1..=60000).contains(&wire["remaining_valid_ms"].as_u64().unwrap()));
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].starts_with("GET /xlog/v1/history?callsign=K1ABC "));
}

#[test]
fn retained_response_cannot_serialize_after_retirement_or_new_connection() {
    let (client, server) = fixture(vec![(200, hello(true)), (200, history())]);
    let lane = Lane::new(&client).unwrap();
    let response = lane.history(lane.token(), &query()).unwrap();
    let replacement = Lane::new(&client).unwrap();
    assert_ne!(lane.token(), replacement.token());
    lane.revoke(); // Does not depend on dropping the still-retained BridgeClient.
    assert!(serde_json::to_value(&response).is_err());
    assert!(replacement.history(lane.token(), &query()).is_err());
    server.join().unwrap();
}

#[test]
fn sdk_session_revocation_is_visible_to_serialization() {
    let (mut client, server) = fixture(vec![(200, hello(true)), (200, history())]);
    let lane = Lane::new(&client).unwrap();
    let response = lane.history(lane.token(), &query()).unwrap();
    client.disconnect();
    assert!(serde_json::to_value(&response).is_err());
    server.join().unwrap();
}

#[test]
fn missing_capability_and_404_are_unavailable_not_empty() {
    let (client, server) = fixture(vec![(200, hello(false))]);
    let lane = Lane::new(&client).unwrap();
    assert!(lane.recent(lane.token()).is_err());
    server.join().unwrap();
    let (client, server) = fixture(vec![
        (200, hello(true)),
        (404, json!({"error":"unsupported"})),
    ]);
    let lane = Lane::new(&client).unwrap();
    assert!(lane.recent(lane.token()).is_err());
    assert!(client.hello().is_some());
    server.join().unwrap();
}

#[test]
fn auth_failure_retires_previous_response() {
    let (client, server) = fixture(vec![
        (200, hello(true)),
        (200, history()),
        (401, json!({"error":"revoked"})),
    ]);
    let lane = Lane::new(&client).unwrap();
    let response = lane.history(lane.token(), &query()).unwrap();
    assert!(lane.recent(lane.token()).is_err());
    assert!(serde_json::to_value(&response).is_err());
    server.join().unwrap();
}

#[test]
fn retained_handle_advancing_revision_retires_earlier_result() {
    let first = recent();
    let mut second = first.clone();
    second["logbook_revision"] = json!(5);
    let (client, server) = fixture(vec![(200, hello(true)), (200, first), (200, second)]);
    let lane = Lane::new(&client).unwrap();
    let earlier = lane.recent(lane.token()).unwrap();
    assert!(serde_json::to_value(&earlier).is_ok());
    let later = lane.recent(lane.token()).unwrap();
    assert!(serde_json::to_value(&earlier).is_err());
    assert_eq!(
        serde_json::to_value(&later).unwrap()["payload"]["logbook_revision"],
        5
    );
    server.join().unwrap();
}

#[test]
fn actual_lease_expiry_is_checked_at_serialization() {
    let mut body = recent();
    body["generated_at_unix_ms"] = json!(now() - 4000);
    let (client, server) = fixture(vec![(200, hello(true)), (200, body)]);
    let lane = Lane::new(&client).unwrap();
    let response = lane.recent(lane.token()).unwrap();
    assert!(serde_json::to_value(&response).is_ok());
    thread::sleep(Duration::from_millis(1100));
    assert!(serde_json::to_value(&response).is_err());
    server.join().unwrap();
}

#[test]
fn stalled_read_is_busy_but_does_not_block_station_poll_or_retirement() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let mut stream = accept(&listener);
        request(&mut stream);
        reply(&mut stream, 200, hello(true));
        let mut read_stream = accept(&listener);
        assert!(request(&mut read_stream).starts_with("GET /xlog/v1/history?"));
        entered_tx.send(()).unwrap();
        let mut poll_stream = accept(&listener);
        assert!(request(&mut poll_stream).starts_with("GET /xlog/v1/snapshot "));
        reply(&mut poll_stream, 200, snapshot());
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        reply(&mut read_stream, 200, history());
    });
    let mut client =
        BridgeClient::new(&endpoint, "synthetic-history-test-token-for-fixtures-only").unwrap();
    client.connect().unwrap();
    let lane = Arc::new(Lane::new(&client).unwrap());
    let reader = Arc::clone(&lane);
    let pending = thread::spawn(move || reader.history(reader.token(), &query()));
    entered_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    assert!(matches!(lane.recent(lane.token()), Err(error) if error.contains("busy")));
    assert!(client.poll().is_ok()); // Real HTTP completes while history is held.
    lane.revoke();
    release_tx.send(()).unwrap();
    assert!(pending.join().unwrap().is_err());
    server.join().unwrap();
}
