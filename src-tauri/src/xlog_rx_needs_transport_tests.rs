//! Actual local HTTP/SDK/receive worker concurrency. Zero PCM, no device or TX.
use super::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::sync::Condvar;

struct Fixture {
    endpoint: String,
    stopping: Arc<AtomicBool>,
    release: Arc<(Mutex<bool>, Condvar)>,
    reads: Arc<AtomicU64>,
    entered: mpsc::Receiver<()>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        *self.release.0.lock().unwrap() = true;
        self.release.1.notify_all();
        self.stopping.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}
fn fixture() -> Fixture {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let stopping = Arc::new(AtomicBool::new(false));
    let stop = stopping.clone();
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let gate = release.clone();
    let reads = Arc::new(AtomicU64::new(0));
    let audio_reads = reads.clone();
    let (notify, entered) = mpsc::channel();
    let origin = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    let thread = std::thread::spawn(move || {
        let mut handlers = Vec::new();
        while !stop.load(Ordering::Acquire) {
            let (mut stream, _) = match listener.accept() {
                Ok(pair) => pair,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(2));
                    continue;
                }
                Err(error) => panic!("fixture listener: {error}"),
            };
            // Each bounded request is independent, including deliberately stalled needs.
            let notify = notify.clone();
            let gate = gate.clone();
            let audio_reads = audio_reads.clone();
            handlers.push(std::thread::spawn(move || {
                stream.set_nonblocking(false).unwrap();
                stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
                let mut headers=Vec::new();
                while !headers.ends_with(b"\r\n\r\n") { let mut b=[0]; stream.read_exact(&mut b).unwrap(); headers.push(b[0]); assert!(headers.len()<16384); }
                let headers=String::from_utf8(headers).unwrap();
                let path=headers.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                let size=headers.lines().find_map(|l|l.to_ascii_lowercase().strip_prefix("content-length:").and_then(|s|s.trim().parse::<usize>().ok())).unwrap_or(0);
                assert!(size<=16384); let mut body=vec![0;size]; stream.read_exact(&mut body).unwrap();
                let now=SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64;
                let authority=json!({"logbook_revision":1,"classification_revision":1,"country_reference_revision":"xlog-country-facts-v3:fixture","policy_revision":"xlog-call-dxcc-needs-v1"});
                let value=if path=="/xlog/v1/hello" {
                    json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","capabilities":["snapshot.read","audio.rx","logbook.classify","rotor.stop"]})
                } else if path=="/xlog/v1/snapshot" {
                    json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","revision":1,"generated_at_unix_ms":now,"max_age_ms":2000,"station":{"connected":false,"radio_id":null,"frequency_hz":null,"mode":null,"transmitting":null},"rotor":{"connected":true,"rotor_id":"fixture-rotor","azimuth_deg":0,"moving":false},"spots":[],"needs":{"generated_at_unix_ms":now,"max_age_ms":2000,"readiness":"ready","authority":authority}})
                } else if path.starts_with("/xlog/v1/audio?") {
                    let sequence=audio_reads.fetch_add(1,Ordering::AcqRel)+1;
                    json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","generated_at_unix_ms":now,"max_age_ms":1000,"state":"running","reason":null,"descriptor":{"capture_id":"00000000-0000-4000-8000-000000000001","epoch":"00000000-0000-4000-8000-000000000002","radio_id":"radio1","receiver_id":"fixture/slice-a","mapping_generation":1,"device_id":"fixture-dax","slice":"A","dax_channel":2,"frequency_hz":14074000,"mode":"DIGU","source_channels":2,"selected_channel":0,"sample_rate":48000,"sample_format":"s16le","timestamp_basis":"windows_endpoint_capture","remote_rf_latency_us":null,"utc_sync_accuracy_us":null},"oldest_sequence":sequence,"next_sequence":sequence+1,"gap":false,"chunks":[{"sequence":sequence,"first_frame":(sequence-1)*1920,"frame_count":1920,"sample0_unix_us":origin+(sequence-1)*40000,"calibration_uncertainty_us":10,"pcm_base64":"A".repeat(5120)}]})
                } else if path=="/xlog/v1/needs/query" {
                    let request:Value=serde_json::from_slice(&body).unwrap();
                    notify.send(()).unwrap();
                    let released=gate.0.lock().unwrap();
                    let (_released,timeout)=gate.1.wait_timeout_while(released,Duration::from_secs(2),|value|!*value).unwrap();
                    assert!(!timeout.timed_out(),"fixture lookup was not released");
                    let results:Vec<_>=request["queries"].as_array().unwrap().iter().map(|q|json!({"query_id":q["query_id"],"normalized_callsign":q["callsign"],"dial_frequency_hz":q["dial_frequency_hz"],"decoder_mode":q["mode"],"classification_mode":"DIGI","band":"20m","country":"United States","adif":291,"country_resolution":"resolved","call_status":"New Call","dxcc_status":"ATNO","need":{"status":"needed","score":100,"reason_code":null,"reasons":["Synthetic owner"]}})).collect();
                    json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","generated_at_unix_ms":now,"max_age_ms":2000,"readiness":"ready","authority":authority,"results":results})
                } else if path=="/xlog/v1/command" {
                    let request:Value=serde_json::from_slice(&body).unwrap();
                    assert_eq!(request["command"]["kind"],"rotor.stop");
                    json!({"request_id":request["request_id"],"server_instance":"worker-fixture","workspace_id":"worker-workspace","status":"observed"})
                } else { panic!("Unexpected fixture path {path}") };
                let body=value.to_string(); let _=write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len());
            }));
        }
        for handler in handlers {
            handler.join().unwrap();
        }
    });
    Fixture {
        endpoint,
        stopping,
        release,
        reads,
        entered,
        thread: Some(thread),
    }
}

#[test]
fn actual_pcm_and_independent_stop_progress_while_needs_http_is_stalled() {
    let _serial = super::tests::ACTIVE_TEST_LOCK.lock().unwrap();
    let fixture = fixture();
    let mut client = xnexus_xlog_client::BridgeClient::new(
        &fixture.endpoint,
        "synthetic-native-worker-needs-token",
    )
    .unwrap();
    client.connect().unwrap();
    client.poll().unwrap();
    let stop = client.stop_handle().unwrap();
    let worker = Worker::start_with_needs(
        client.audio_handle().unwrap(),
        "worker-fixture".into(),
        "worker-workspace".into(),
        Mode::FT8,
        Some(client.needs_handle().unwrap()),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while worker.view().unwrap().route.is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    {
        let mut state = worker.state.lock().unwrap();
        assert!(state.view.route.is_some());
        // Decoder output insertion seam; PCM/assembly is the actual running worker.
        let row = super::tests::row(&state, "synthetic_row_a", "K1ABC");
        state.view.rows.push(row);
    }
    fixture
        .entered
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
    let before = fixture.reads.load(Ordering::Acquire);
    let deadline = Instant::now() + Duration::from_millis(400);
    while fixture.reads.load(Ordering::Acquire) < before + 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        fixture.reads.load(Ordering::Acquire) >= before + 2,
        "PCM HTTP lane blocked behind needs"
    );
    assert!(worker.view().unwrap().enabled);
    stop.stop_rotor("00000000-0000-4000-8000-000000000003", "fixture-rotor")
        .unwrap();
    worker.stop();
    assert!(!worker.view().unwrap().enabled);
    assert!(worker.view().unwrap().rows.is_empty());
    assert!(
        ACTIVE.load(Ordering::Acquire),
        "held lookup must retain old worker lease"
    );
    assert!(Worker::start(
        client.audio_handle().unwrap(),
        "worker-fixture".into(),
        "worker-workspace".into(),
        Mode::FT8
    )
    .is_err());
    *fixture.release.0.lock().unwrap() = true;
    fixture.release.1.notify_all();
    let deadline = Instant::now() + Duration::from_secs(2);
    while ACTIVE.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!ACTIVE.load(Ordering::Acquire));
    assert!(
        worker.view().unwrap().rows.is_empty(),
        "late reply restored stopped rows"
    );
    client.disconnect();
}
