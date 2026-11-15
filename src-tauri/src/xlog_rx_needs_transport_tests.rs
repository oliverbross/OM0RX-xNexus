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
    query_ids: Arc<Mutex<Vec<Vec<String>>>>,
    audio_phase: Arc<AtomicU64>,
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
    fixture_with_miss(false)
}
fn fixture_with_miss(first_miss: bool) -> Fixture {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let stopping = Arc::new(AtomicBool::new(false));
    let stop = stopping.clone();
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let gate = release.clone();
    let reads = Arc::new(AtomicU64::new(0));
    let audio_reads = reads.clone();
    let audio_sequence = Arc::new(AtomicU64::new(0));
    let audio_phase = Arc::new(AtomicU64::new(0));
    let phase_control = audio_phase.clone();
    let (notify, entered) = mpsc::channel();
    let query_ids = Arc::new(Mutex::new(Vec::new()));
    let recorded_queries = query_ids.clone();
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
            let audio_sequence = audio_sequence.clone();
            let recorded_queries = recorded_queries.clone();
            let phase_control = phase_control.clone();
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
                    json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","revision":if first_miss {now} else {1},"generated_at_unix_ms":now,"max_age_ms":2000,"station":{"connected":false,"radio_id":null,"frequency_hz":null,"mode":null,"transmitting":null},"rotor":{"connected":true,"rotor_id":"fixture-rotor","azimuth_deg":0,"moving":false},"spots":[],"needs":{"generated_at_unix_ms":now,"max_age_ms":2000,"readiness":"ready","authority":authority}})
                } else if path.starts_with("/xlog/v1/audio?") {
                    audio_reads.fetch_add(1,Ordering::AcqRel);
                    let phase=phase_control.load(Ordering::Acquire);
                    let after:u64=path.split("after=").nth(1).unwrap().parse().unwrap();
                    if phase==1 {
                        json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","generated_at_unix_ms":now,"max_age_ms":1000,"state":"stopped","reason":"Synthetic capture gap; waiting for new qualified PCM","descriptor":null,"oldest_sequence":after+1,"next_sequence":after+1,"gap":false,"chunks":[]})
                    } else {
                    let sequence=audio_sequence.fetch_add(1,Ordering::AcqRel)+1;
                    json!({"protocol_version":1,"server_instance":"worker-fixture","workspace_id":"worker-workspace","generated_at_unix_ms":now,"max_age_ms":1000,"state":"running","reason":null,"descriptor":{"capture_id":"00000000-0000-4000-8000-000000000001","epoch":if phase==0 {"00000000-0000-4000-8000-000000000002"} else {"00000000-0000-4000-8000-000000000004"},"radio_id":"radio1","receiver_id":"fixture/slice-a","mapping_generation":1,"device_id":"fixture-dax","slice":"A","dax_channel":2,"frequency_hz":14074000,"mode":"DIGU","source_channels":2,"selected_channel":0,"sample_rate":48000,"sample_format":"s16le","timestamp_basis":"windows_endpoint_capture","remote_rf_latency_us":null,"utc_sync_accuracy_us":null},"oldest_sequence":sequence,"next_sequence":sequence+1,"gap":phase==3,"chunks":[{"sequence":sequence,"first_frame":(sequence-1)*1920,"frame_count":1920,"sample0_unix_us":origin+(sequence-1)*40000,"calibration_uncertainty_us":10,"pcm_base64":"A".repeat(5120)}]})
                    }
                } else if path=="/xlog/v1/needs/query" {
                    let request:Value=serde_json::from_slice(&body).unwrap();
                    let attempt={let mut ids=recorded_queries.lock().unwrap();
                        ids.push(request["queries"].as_array().unwrap().iter().map(|q|q["query_id"].as_str().unwrap().to_owned()).collect());ids.len()};
                    notify.send(()).unwrap();
                    if first_miss && attempt==1 {
                        let body=r#"{"error":"Synthetic owner cache miss"}"#;
                        write!(stream,"HTTP/1.1 503 Service Unavailable\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
                        return;
                    }
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
        query_ids,
        audio_phase,
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

#[test]
fn actual_pending503_retry_preserves_query_identity_while_pcm_and_stop_progress() {
    let _serial = super::tests::ACTIVE_TEST_LOCK.lock().unwrap();
    let fixture = fixture_with_miss(true);
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
        let row = super::tests::row(&state, "pending_original", "K1ABC");
        state.view.rows.push(row);
    }
    fixture
        .entered
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
    let first = Instant::now();
    let mut polled = false;
    loop {
        match fixture.entered.recv_timeout(Duration::from_millis(50)) {
            Ok(()) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                assert!(
                    first.elapsed() < Duration::from_secs(3),
                    "second needs request missing: phase={} reason={} rows={} PCMreads={}",
                    worker.view().unwrap().phase,
                    worker.view().unwrap().reason,
                    worker.view().unwrap().rows.len(),
                    fixture.reads.load(Ordering::Acquire)
                );
                // Match main client observation, never extend an old fact lease.
                if !polled && first.elapsed() >= Duration::from_secs(1) {
                    let stamp = client
                        .poll()
                        .unwrap()
                        .needs
                        .as_ref()
                        .unwrap()
                        .generated_at_unix_ms;
                    worker.needs_snapshot_observed(stamp);
                    polled = true;
                }
            }
            Err(error) => panic!("fixture stopped before retry: {error}"),
        }
    }
    let ids = fixture.query_ids.lock().unwrap().clone();
    assert_eq!(ids.len(), 2);
    assert_eq!(ids[0], vec!["pending_original"]);
    assert_eq!(ids[0], ids[1]);
    assert!(
        first.elapsed() >= Duration::from_millis(900),
        "retry bypassed one-second cadence"
    );
    let before = fixture.reads.load(Ordering::Acquire);
    let deadline = Instant::now() + Duration::from_millis(400);
    while fixture.reads.load(Ordering::Acquire) < before + 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        fixture.reads.load(Ordering::Acquire) >= before + 2,
        "PCM stalled behind pending retry"
    );
    stop.stop_rotor("00000000-0000-4000-8000-000000000003", "fixture-rotor")
        .unwrap();
    worker.stop();
    assert!(worker.view().unwrap().rows.is_empty());
    assert!(!worker.view().unwrap().enabled);
    *fixture.release.0.lock().unwrap() = true;
    fixture.release.1.notify_all();
    let deadline = Instant::now() + Duration::from_secs(2);
    while ACTIVE.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!ACTIVE.load(Ordering::Acquire));
    assert!(worker.view().unwrap().rows.is_empty());
    client.disconnect();
}

#[test]
fn actual_worker_pcm_waterfall_flushes_gap_and_accepts_new_epoch_despite_old_cursor() {
    let _serial = super::tests::ACTIVE_TEST_LOCK.lock().unwrap();
    let fixture = fixture();
    let mut client = xnexus_xlog_client::BridgeClient::new(
        &fixture.endpoint,
        "synthetic-native-spectrum-gap-test-marker",
    )
    .unwrap();
    client.connect().unwrap();
    let worker = Worker::start(
        client.audio_handle().unwrap(),
        "worker-fixture".into(),
        "worker-workspace".into(),
        Mode::FT8,
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let first = loop {
        if let Some(frame) = worker.spectrum(None).unwrap() {
            break frame;
        }
        assert!(
            Instant::now() < deadline,
            "actual Worker PCM never produced complete spectrum"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(first.spectrum.row.len(), 640);
    let observed = worker.view().unwrap();
    assert!(observed.rows.is_empty());
    assert_eq!(observed.current_context.as_ref(), Some(&first.context));
    assert!(
        first.spectrum.row.iter().all(|v| *v == 0.0),
        "actual synthetic silence, not fabricated warming-up bins"
    );
    let cursor = SpectrumCursor {
        worker_instance: first.context.worker_instance.clone(),
        worker_epoch: first.context.worker_epoch,
        sequence: u64::MAX, // New epoch must beat context, never just sequence.
    };
    fixture.audio_phase.store(1, Ordering::Release);
    let deadline = Instant::now() + Duration::from_secs(1);
    while worker.view().unwrap().route.is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(worker.view().unwrap().route.is_none());
    assert!(worker.view().unwrap().current_context.is_none());
    assert!(worker.spectrum(None).unwrap().is_none());
    fixture.audio_phase.store(2, Ordering::Release);
    let deadline = Instant::now() + Duration::from_secs(2);
    let resumed = loop {
        if let Some(frame) = worker.spectrum(Some(&cursor)).unwrap() {
            break frame;
        }
        assert!(
            Instant::now() < deadline,
            "new epoch never produced fresh spectrum"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(resumed.context.worker_epoch > first.context.worker_epoch);
    assert_ne!(resumed.context.route.epoch, first.context.route.epoch);
    assert!(resumed.sequence > first.sequence);
    assert!(resumed.source_origin_frame > first.source_origin_frame);
    // A genuine running gap=true response reaches AudioClient.reset and the
    // production Worker gap path while the descriptor remains unchanged.
    let gap_cursor = SpectrumCursor {
        worker_instance: resumed.context.worker_instance.clone(),
        worker_epoch: resumed.context.worker_epoch,
        sequence: u64::MAX,
    };
    fixture.audio_phase.store(3, Ordering::Release);
    let deadline = Instant::now() + Duration::from_secs(1);
    while worker.state.lock().unwrap().epoch <= resumed.context.worker_epoch
        && Instant::now() < deadline
    {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(worker.state.lock().unwrap().epoch > resumed.context.worker_epoch);
    assert!(worker.view().unwrap().route.is_some());
    assert!(worker.spectrum(None).unwrap().is_none());
    fixture.audio_phase.store(4, Ordering::Release);
    let deadline = Instant::now() + Duration::from_secs(2);
    let after_gap = loop {
        if let Some(frame) = worker.spectrum(Some(&gap_cursor)).unwrap() {
            break frame;
        }
        assert!(Instant::now() < deadline, "running gap did not re-prime");
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(after_gap.context.route, resumed.context.route);
    assert!(after_gap.context.worker_epoch > resumed.context.worker_epoch);
    assert!(after_gap.source_origin_frame > resumed.source_origin_frame);
    assert_eq!(
        worker.view().unwrap().current_context.as_ref(),
        Some(&after_gap.context)
    );
    worker.stop();
    assert!(worker.spectrum(None).unwrap().is_none());
    let deadline = Instant::now() + Duration::from_secs(2);
    while ACTIVE.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        !ACTIVE.load(Ordering::Acquire),
        "all actual Worker lanes must retire"
    );
    let restarted = Worker::start(
        client.audio_handle().unwrap(),
        "worker-fixture".into(),
        "worker-workspace".into(),
        Mode::FT8,
    )
    .unwrap();
    let old_worker_cursor = SpectrumCursor {
        worker_instance: after_gap.context.worker_instance.clone(),
        worker_epoch: after_gap.context.worker_epoch,
        sequence: u64::MAX,
    };
    let deadline = Instant::now() + Duration::from_secs(2);
    let replacement = loop {
        if let Some(frame) = restarted.spectrum(Some(&old_worker_cursor)).unwrap() {
            break frame;
        }
        assert!(
            Instant::now() < deadline,
            "new worker cursor failed to retire previous worker"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_ne!(
        replacement.context.worker_instance,
        after_gap.context.worker_instance
    );
    assert_eq!(
        restarted.view().unwrap().current_context.as_ref(),
        Some(&replacement.context)
    );
    restarted.stop();
    let deadline = Instant::now() + Duration::from_secs(2);
    while ACTIVE.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(!ACTIVE.load(Ordering::Acquire));
    client.disconnect();
}
