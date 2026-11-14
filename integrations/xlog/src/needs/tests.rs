use super::*;
use serde_json::{json, Value};
const NOW: u64 = 1_790_000_000_000;
fn metadata(readiness: Readiness, now: u64) -> NeedsMetadata {
    NeedsMetadata {
        generated_at_unix_ms: now,
        max_age_ms: 2000,
        readiness,
        authority: (readiness == Readiness::Ready).then(|| NeedsAuthority {
            logbook_revision: 0,
            classification_revision: 7,
            country_reference_revision: "cty:opaque/1".into(),
            policy_revision: "xlog-call-dxcc-needs-v1".into(),
        }),
    }
}
fn queries() -> Vec<NeedsQuery> {
    vec![
        NeedsQuery {
            query_id: "decode_1-0".into(),
            callsign: "K1ABC".into(),
            dial_frequency_hz: 14_074_000,
            mode: DecoderMode::FT8,
        },
        NeedsQuery {
            query_id: "decode_1-1".into(),
            callsign: "DL1ABC/P".into(),
            dial_frequency_hz: 14_074_000,
            mode: DecoderMode::FT8,
        },
    ]
}
fn wire(readiness: Readiness, now: u64) -> Value {
    let m = metadata(readiness, now);
    let results:Vec<_>=queries().iter().map(|q| {
        let ready=readiness==Readiness::Ready;
        json!({"query_id":q.query_id,"normalized_callsign":q.callsign,"dial_frequency_hz":q.dial_frequency_hz,"decoder_mode":q.mode,"classification_mode":"DIGI",
            "band":if ready {json!("20m")}else{Value::Null},"country":if ready{json!("United States")}else{Value::Null},"adif":if ready{json!(291)}else{Value::Null},"country_resolution":if ready{json!("resolved")}else{Value::Null},
            "call_status":if ready{"New Call"}else{"Unknown"},"dxcc_status":if ready{"ATNO"}else{"Unknown"},
            "need":{"status":if ready{"needed"}else{"unknown"},"score":if ready{json!(100)}else{Value::Null},"reason_code":if ready{Value::Null}else{json!(readiness.code())},"reasons":if ready{json!(["Owner reason"])}else{json!([])}}})
    }).collect();
    json!({"protocol_version":1,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace","generated_at_unix_ms":now,"max_age_ms":2000,"readiness":m.readiness,"authority":m.authority,"results":results})
}
fn paired(readiness: Readiness) -> (BridgeClient, NeedsClient) {
    let mut bridge = BridgeClient::new("http://127.0.0.1:1", &crate::tests::token()).unwrap();
    bridge.hello = Some(Hello {
        application: "OM0RX-xLog".into(),
        protocol_version: 1,
        server_instance: "synthetic-server".into(),
        workspace_id: "synthetic-workspace".into(),
        capabilities: vec!["snapshot.read".into(), "logbook.classify".into()],
    });
    bridge.audio_session = Arc::new(AtomicBool::new(true));
    bridge
        .needs_authority
        .lock()
        .unwrap()
        .observe(Some(&metadata(readiness, NOW)), NOW)
        .unwrap();
    let client = bridge.needs_handle().unwrap();
    (bridge, client)
}
fn accept(client: &mut NeedsClient, value: Value) -> Result<NeedsBatch, Error> {
    let lease = client.authority.lock().unwrap().lease()?;
    client.accept(
        serde_json::from_value(value).map_err(|_| Error::Invalid)?,
        &queries(),
        lease,
        NOW,
    )
}
#[test]
fn owner_scores_and_portable_calls_attach_by_id_not_position() {
    let (_bridge, mut client) = paired(Readiness::Ready);
    let mut value = wire(Readiness::Ready, NOW);
    value["results"].as_array_mut().unwrap().reverse();
    value["results"][0]["need"]["score"] = json!(-17);
    let batch = accept(&mut client, value).unwrap();
    assert_eq!(batch.results().unwrap()[0].query_id, "decode_1-1");
    assert_eq!(batch.results().unwrap()[0].normalized_callsign, "DL1ABC/P");
    assert_eq!(batch.results().unwrap()[0].need.score, Some(-17));
    assert_eq!(
        batch
            .metadata()
            .unwrap()
            .authority
            .as_ref()
            .unwrap()
            .logbook_revision,
        0
    );
}
#[test]
fn exact_result_id_set_and_echoes_are_required() {
    for mutation in 0..6 {
        let (_bridge, mut client) = paired(Readiness::Ready);
        let mut v = wire(Readiness::Ready, NOW);
        match mutation {
            0 => {
                v["results"].as_array_mut().unwrap().pop();
            }
            1 => v["results"][1]["query_id"] = json!("decode_1-0"),
            2 => v["results"][1]["query_id"] = json!("another-row"),
            3 => v["results"][0]["normalized_callsign"] = json!("K2OTHER"),
            4 => v["results"][0]["dial_frequency_hz"] = json!(7_074_000),
            _ => v["results"][0]["decoder_mode"] = json!("FT4"),
        }
        assert!(
            matches!(accept(&mut client, v), Err(Error::Invalid)),
            "mutation{mutation}"
        );
    }
}
#[test]
fn requests_are_strictly_bounded_before_network() {
    let (_bridge, mut client) = paired(Readiness::Ready);
    let good = queries()[0].clone();
    for mut q in [
        good.clone(),
        good.clone(),
        good.clone(),
        good.clone(),
        good.clone(),
    ]
    .into_iter()
    .enumerate()
    {
        match q.0 {
            0 => q.1.query_id = "a".repeat(129),
            1 => q.1.query_id = "bad/id".into(),
            2 => q.1.callsign = "<K1ABC>".into(),
            3 => q.1.callsign = "k1abc".into(),
            _ => q.1.dial_frequency_hz = 1_000_000_000_001,
        }
        assert!(matches!(client.query(&[q.1]), Err(Error::Invalid)));
    }
    assert!(matches!(client.query(&[]), Err(Error::Invalid)));
    assert!(matches!(
        client.query(&[good.clone(), good.clone()]),
        Err(Error::Invalid)
    ));
    let many: Vec<_> = (0..33)
        .map(|i| NeedsQuery {
            query_id: format!("q{i}"),
            ..good.clone()
        })
        .collect();
    assert!(matches!(client.query(&many), Err(Error::Invalid)));
    let maximum: Vec<_> = (0..32)
        .map(|i| NeedsQuery {
            query_id: format!("{}-{i}", "x".repeat(124)),
            callsign: format!("K1{}", "A".repeat(30)),
            dial_frequency_hz: 1_000_000_000_000,
            mode: DecoderMode::FT4,
        })
        .collect();
    assert!(maximum.iter().all(NeedsQuery::validate));
    assert!(
        serde_json::to_vec(&json!({"queries":maximum}))
            .unwrap()
            .len()
            < REQUEST_BYTES
    );
}
#[test]
fn required_nullable_fields_cannot_be_omitted() {
    for field in [
        "authority",
        "generated_at_unix_ms",
        "max_age_ms",
        "readiness",
        "results",
    ] {
        let mut v = wire(Readiness::Unavailable, NOW);
        v.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<WireResponse>(v).is_err(),
            "{field}"
        );
    }
    for field in [
        "band",
        "country",
        "adif",
        "country_resolution",
        "query_id",
        "call_status",
        "dxcc_status",
    ] {
        let mut v = wire(Readiness::Unavailable, NOW);
        v["results"][0].as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<WireResponse>(v).is_err(),
            "{field}"
        );
    }
    for field in ["score", "reason_code", "status", "reasons"] {
        let mut v = wire(Readiness::Unavailable, NOW);
        v["results"][0]["need"]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            serde_json::from_value::<WireResponse>(v).is_err(),
            "{field}"
        );
    }
    assert!(serde_json::from_value::<WireResponse>(wire(Readiness::Unavailable, NOW)).is_ok());
}
#[test]
fn nonready_and_known_no_entity_stay_unknown_without_atno_or_zero_score() {
    for readiness in [
        Readiness::Unavailable,
        Readiness::Refreshing,
        Readiness::Uncertain,
        Readiness::Stale,
    ] {
        let (_bridge, mut client) = paired(readiness);
        let b = accept(&mut client, wire(readiness, NOW)).unwrap();
        assert_eq!(b.results().unwrap()[0].need.score, None);
        let mut bad = wire(readiness, NOW);
        bad["results"][0]["need"]["score"] = json!(0);
        assert!(accept(&mut client, bad).is_err());
    }
    let (_bridge, mut client) = paired(Readiness::Ready);
    let mut v = wire(Readiness::Ready, NOW);
    v["results"][0]["country"] = Value::Null;
    v["results"][0]["adif"] = Value::Null;
    v["results"][0]["country_resolution"] = json!("knownNoEntity");
    v["results"][0]["dxcc_status"] = json!("Unknown");
    v["results"][0]["need"] =
        json!({"status":"unknown","score":null,"reason_code":"known_no_entity","reasons":[]});
    let b = accept(&mut client, v.clone()).unwrap();
    assert_eq!(b.results().unwrap()[0].call_status, CallStatus::NewCall);
    v["results"][0]["dxcc_status"] = json!("ATNO");
    assert!(accept(&mut client, v).is_err());
}
#[test]
fn authority_fence_and_expiry_revoke_already_returned_facts() {
    for action in 0..5 {
        let (mut bridge, mut client) = paired(Readiness::Ready);
        let b = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
        assert!(b.is_fresh());
        match action {
            0 => bridge.disconnect(),
            1 => bridge.invalidate_needs(),
            2 => {
                let mut m = metadata(Readiness::Ready, NOW + 1);
                m.authority.as_mut().unwrap().country_reference_revision = "changed".into();
                bridge
                    .needs_authority
                    .lock()
                    .unwrap()
                    .observe(Some(&m), NOW)
                    .unwrap();
            }
            3 => {
                bridge
                    .needs_authority
                    .lock()
                    .unwrap()
                    .observe(None, NOW)
                    .unwrap();
            }
            _ => client.disconnect(),
        }
        assert!(!b.is_fresh());
        assert!(b.results().is_err());
    }
    let (_bridge, mut client) = paired(Readiness::Ready);
    let mut b = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
    b.expires = Instant::now();
    assert!(b.results().is_err());
}
#[test]
fn repeated_timestamps_and_wall_clock_rollback_cannot_renew_leases() {
    let (bridge, mut client) = paired(Readiness::Ready);
    let original = bridge
        .needs_authority
        .lock()
        .unwrap()
        .lease()
        .unwrap()
        .expires;
    bridge
        .needs_authority
        .lock()
        .unwrap()
        .observe(Some(&metadata(Readiness::Ready, NOW)), NOW - 20)
        .unwrap();
    assert_eq!(
        bridge
            .needs_authority
            .lock()
            .unwrap()
            .lease()
            .unwrap()
            .expires,
        original
    );
    let first = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
    let second = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
    assert_eq!(first.expires, second.expires);
    let mut state = bridge.needs_authority.lock().unwrap();
    state.last_timestamp = Some((NOW, Instant::now()));
    state.invalidate();
    assert!(matches!(
        state.observe(Some(&metadata(Readiness::Ready, NOW)), NOW - 20),
        Err(Error::Stale)
    ));
}
#[test]
fn changed_inflight_authority_and_late_disconnect_cannot_attach() {
    for disconnect in [false, true] {
        let (mut bridge, mut client) = paired(Readiness::Ready);
        let lease = client.authority.lock().unwrap().lease().unwrap();
        if disconnect {
            bridge.disconnect();
        } else {
            bridge.invalidate_needs();
        }
        assert!(client
            .accept(
                serde_json::from_value(wire(Readiness::Ready, NOW)).unwrap(),
                &queries(),
                lease,
                NOW
            )
            .is_err());
    }
    let (_bridge, mut client) = paired(Readiness::Ready);
    let mut v = wire(Readiness::Ready, NOW);
    v["authority"]["logbook_revision"] = json!(1);
    assert!(matches!(accept(&mut client, v), Err(Error::Stale)));
}
#[test]
fn response_bounds_identity_statuses_and_clock_are_checked() {
    for (path, value) in [
        ("/server_instance", json!("other")),
        ("/workspace_id", json!("other")),
        ("/protocol_version", json!(2)),
        ("/max_age_ms", json!(2001)),
        ("/generated_at_unix_ms", json!(NOW + 51)),
        ("/authority/logbook_revision", json!(SAFE + 1)),
        (
            "/authority/country_reference_revision",
            json!("bad\u{007f}"),
        ),
        ("/authority/policy_revision", json!("x".repeat(65))),
        ("/results/0/country", json!("bad\ncountry")),
        ("/results/0/adif", json!(0)),
        ("/results/0/classification_mode", json!("FT8")),
        ("/results/0/call_status", json!("new call")),
        ("/results/0/need/reasons", json!(vec!["reason"; 31])),
        ("/results/0/need/score", Value::Null),
    ] {
        let (_bridge, mut client) = paired(Readiness::Ready);
        let mut v = wire(Readiness::Ready, NOW);
        *v.pointer_mut(path).unwrap() = value;
        assert!(accept(&mut client, v).is_err(), "{path}");
    }
    let (_bridge, mut client) = paired(Readiness::Ready);
    let mut v = wire(Readiness::Ready, NOW);
    v["future_additive"] = json!({"bounded":"ignored"});
    assert!(accept(&mut client, v).is_ok());
}
fn network_client(
    responses: Vec<String>,
) -> (
    BridgeClient,
    NeedsClient,
    std::thread::JoinHandle<Vec<String>>,
) {
    let now = now_ms().unwrap();
    let mut h = crate::tests::hello();
    h["capabilities"] = json!(["snapshot.read", "logbook.classify", "audio.rx"]);
    let mut s = crate::tests::snapshot();
    s["needs"] = serde_json::to_value(metadata(Readiness::Ready, now)).unwrap();
    let mut replies = vec![
        crate::tests::json_response(h),
        crate::tests::json_response(s),
    ];
    replies.extend(responses);
    let (mut b, server) = crate::tests::server(replies);
    b.connect().unwrap();
    b.poll().unwrap();
    let n = b.needs_handle().unwrap();
    (b, n, server)
}
#[test]
fn post_is_readonly_bounded_and_auth_revocation_is_terminal() {
    let reply = crate::tests::json_response(wire(Readiness::Ready, now_ms().unwrap()));
    let (mut bridge, mut client, server) = network_client(vec![
        reply,
        "HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".into(),
    ]);
    let mut audio = bridge.audio_handle().unwrap();
    let batch = client.query(&queries()).unwrap();
    assert!(batch.is_fresh());
    assert!(matches!(client.query(&queries()), Err(Error::Http(401))));
    assert!(!batch.is_fresh());
    assert!(bridge.snapshot().is_err());
    assert!(bridge.hello().is_none());
    assert!(matches!(audio.poll(), Err(Error::Disconnected)));
    assert!(matches!(bridge.history("K1ABC"), Err(Error::Disconnected)));
    assert!(matches!(
        bridge.receipt("00000000-0000-4000-8000-000000000001"),
        Err(Error::Disconnected)
    ));
    assert!(matches!(client.query(&queries()), Err(Error::Disconnected)));
    let requests = server.join().unwrap();
    assert_eq!(requests.len(), 4);
    let request = &requests[2];
    assert!(request.starts_with("POST /xlog/v1/needs/query HTTP/1.1"));
    let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body.as_object().unwrap().len(), 1);
    assert_eq!(body["queries"][0].as_object().unwrap().len(), 4);
    assert!(!body.to_string().contains("adif"));
}
#[test]
fn absent_capability_and_missing_snapshot_metadata_preserve_base_client() {
    let (mut b, server) = crate::tests::server(vec![
        crate::tests::json_response(crate::tests::hello()),
        crate::tests::json_response(crate::tests::snapshot()),
    ]);
    b.connect().unwrap();
    b.poll().unwrap();
    assert!(matches!(b.needs_handle(), Err(Error::Capability)));
    assert!(b.snapshot().is_ok());
    assert_eq!(server.join().unwrap().len(), 2);
    let (bridge, mut needs, server) = network_client(vec![]);
    bridge.invalidate_needs();
    assert!(matches!(needs.query(&queries()), Err(Error::Stale)));
    assert!(bridge.snapshot().is_ok());
    assert_eq!(server.join().unwrap().len(), 2);
}
#[test]
fn saturation_optional_absence_and_oversized_reply_do_not_revoke_audio_session() {
    for status in [503, 404] {
        let (bridge, mut client, server) = network_client(vec![format!(
            "HTTP/1.1 {status} Refused\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        )]);
        assert!(matches!(client.query(&queries()),Err(Error::Http(code)) if code==status));
        assert!(bridge.snapshot().is_ok());
        assert!(bridge.audio_session.load(Ordering::Acquire));
        server.join().unwrap();
    }
    let response=format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",RESPONSE_BYTES+1);
    let (bridge, mut client, server) = network_client(vec![response]);
    assert!(matches!(client.query(&queries()), Err(Error::TooLarge)));
    assert!(bridge.snapshot().is_ok());
    server.join().unwrap();
}

#[test]
fn stalled_needs_transport_does_not_serialize_pcm_or_emergency_stop() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::Condvar,
        thread,
    };
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let started = Arc::new((Mutex::new(false), Condvar::new()));
    let signal = started.clone();
    let server = thread::spawn(move || {
        let mut workers = Vec::new();
        for _ in 0..5 {
            let (mut stream, _) = listener.accept().unwrap();
            let signal = signal.clone();
            workers.push(thread::spawn(move || {
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut bytes=Vec::new();let mut byte=[0];
                while !bytes.ends_with(b"\r\n\r\n") { assert!(bytes.len()<16384);stream.read_exact(&mut byte).unwrap();bytes.push(byte[0]); }
                let header=String::from_utf8(bytes).unwrap();
                let count=header.lines().find_map(|l|l.to_ascii_lowercase().strip_prefix("content-length:").map(|s|s.trim().parse::<usize>().unwrap())).unwrap_or(0);
                assert!(count<=REQUEST_BYTES);let mut body=vec![0;count];stream.read_exact(&mut body).unwrap();
                let now=now_ms().unwrap();
                let value=if header.starts_with("GET /xlog/v1/hello ") {
                    let mut h=crate::tests::hello();h["capabilities"]=json!(["snapshot.read","logbook.classify","audio.rx","rotor.stop"]);h
                } else if header.starts_with("GET /xlog/v1/snapshot ") {
                    let mut s=crate::tests::snapshot();s["needs"]=serde_json::to_value(metadata(Readiness::Ready,now)).unwrap();s
                } else if header.starts_with("POST /xlog/v1/needs/query ") {
                    *signal.0.lock().unwrap()=true;signal.1.notify_one();
                    thread::sleep(Duration::from_millis(1200));wire(Readiness::Ready,now_ms().unwrap())
                } else if header.starts_with("GET /xlog/v1/audio?after=0 ") { crate::audio::tests::fixture(now) }
                else {
                    assert!(header.starts_with("POST /xlog/v1/command "));
                    let body:Value=serde_json::from_slice(&body).unwrap();assert_eq!(body["command"]["kind"],"rotor.stop");
                    json!({"request_id":body["request_id"],"server_instance":"synthetic-server","workspace_id":"synthetic-workspace","status":"observed","message":null})
                };
                let _=stream.write_all(crate::tests::json_response(value).as_bytes());
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
    let mut bridge =
        BridgeClient::new(&format!("http://{address}"), &crate::tests::token()).unwrap();
    bridge.connect().unwrap();
    bridge.poll().unwrap();
    let mut needs = bridge.needs_handle().unwrap();
    let mut audio = bridge.audio_handle().unwrap();
    let stop = bridge.stop_handle().unwrap();
    let pending = thread::spawn(move || needs.query(&queries()));
    let ready = started.0.lock().unwrap();
    let (ready, timeout) = started
        .1
        .wait_timeout_while(ready, Duration::from_secs(2), |v| !*v)
        .unwrap();
    assert!(*ready && !timeout.timed_out());
    drop(ready);
    assert!(audio.poll().unwrap().is_fresh());
    assert_eq!(
        stop.stop_rotor("00000000-0000-4000-8000-000000000001", "fake-rotor")
            .unwrap()
            .status,
        CommandStatus::Observed
    );
    assert!(
        !pending.is_finished(),
        "PCM and Stop should complete while the bounded needs query is still waiting"
    );
    assert!(matches!(pending.join().unwrap(), Err(Error::Transport)));
    assert!(bridge.snapshot().is_ok());
    assert!(bridge.audio_session.load(Ordering::Acquire));
    server.join().unwrap();
}

#[test]
fn shared_literal_grammar_keeps_digit_ending_special_calls_and_rejects_reports() {
    for call in [
        "GB50",
        "9A2024",
        "K1ABC",
        "DL1ABC/P",
        "PJ4/K1ABC",
        "G8ABC/MM",
    ] {
        assert!(normalized_call(call), "{call}");
    }
    for call in [
        "", "A1", "RR73", "RRR", "R73", "73", "599", "59", "5NN", "59N", "5N9", "<K1ABC>", "<...>",
        "/K1ABC", "K1ABC/", "K1ABC//P", "k1abc", "K1ABC P",
    ] {
        assert!(!normalized_call(call), "{call}");
    }
}

#[test]
fn unresolved_country_facts_and_unsupported_band_cannot_contradict_unknown() {
    for (code, resolutions) in [
        ("country_ambiguous", vec!["ambiguous"]),
        ("country_unknown", vec!["unknown", "invalid", "missingDate"]),
    ] {
        for resolution in resolutions {
            let (_bridge, mut client) = paired(Readiness::Ready);
            let mut v = wire(Readiness::Ready, NOW);
            v["results"][0]["country"] = Value::Null;
            v["results"][0]["adif"] = Value::Null;
            v["results"][0]["country_resolution"] = json!(resolution);
            v["results"][0]["dxcc_status"] = json!("Unknown");
            v["results"][0]["need"] =
                json!({"status":"unknown","score":null,"reason_code":code,"reasons":[]});
            assert!(accept(&mut client, v.clone()).is_ok());
            for (field, value) in [
                ("country", json!("borrowed")),
                ("adif", json!(291)),
                ("dxcc_status", json!("ATNO")),
                ("country_resolution", json!("resolved")),
            ] {
                let mut bad = v.clone();
                bad["results"][0][field] = value;
                assert!(accept(&mut client, bad).is_err());
            }
        }
    }
    let (_bridge, mut client) = paired(Readiness::Ready);
    let mut v = wire(Readiness::Ready, NOW);
    v["results"][0]["need"] =
        json!({"status":"unknown","score":null,"reason_code":"unsupported_band","reasons":[]});
    assert!(accept(&mut client, v.clone()).is_err());
    v["results"][0]["band"] = Value::Null;
    assert!(accept(&mut client, v).is_ok());
}

#[test]
fn opaque_printable_ascii_identities_preserve_spaces_and_unicode_text_is_byte_bounded() {
    let (bridge, mut client) = paired(Readiness::Ready);
    let mut m = metadata(Readiness::Ready, NOW);
    m.authority.as_mut().unwrap().country_reference_revision =
        " opaque identity with spaces ".into();
    bridge
        .needs_authority
        .lock()
        .unwrap()
        .observe(Some(&m), NOW)
        .unwrap();
    let mut v = wire(Readiness::Ready, NOW);
    v["authority"] = serde_json::to_value(m.authority).unwrap();
    v["results"][0]["country"] = json!("Č".repeat(64));
    v["results"][0]["need"]["reasons"] = json!([" ", "Č".repeat(128)]);
    assert!(accept(&mut client, v.clone()).is_ok());
    let mut empty = v.clone();
    empty["results"][0]["need"]["reasons"] = json!([""]);
    assert!(accept(&mut client, empty).is_err());
    v["results"][0]["country"] = json!("Č".repeat(65));
    assert!(accept(&mut client, v).is_err());
}

#[test]
fn expired_backwards_or_missing_optional_metadata_only_revokes_needs() {
    for change in ["expired", "backwards", "missing", "null"] {
        let now = now_ms().unwrap();
        let mut next = crate::tests::snapshot();
        next["revision"] = json!(6);
        next["needs"] = serde_json::to_value(metadata(
            Readiness::Ready,
            if change == "expired" {
                now - 2001
            } else {
                now - 1
            },
        ))
        .unwrap();
        if change == "missing" {
            next.as_object_mut().unwrap().remove("needs");
        }
        if change == "null" {
            next["needs"] = Value::Null;
        }
        let (mut bridge, mut client, server) = network_client(vec![
            crate::tests::json_response(wire(Readiness::Ready, now)),
            crate::tests::json_response(next),
            crate::tests::json_response(crate::audio::tests::fixture(now)),
        ]);
        let mut audio = bridge.audio_handle().unwrap();
        let batch = client.query(&queries()).unwrap();
        assert!(batch.is_fresh());
        bridge.poll().unwrap();
        assert!(!batch.is_fresh(), "{change}");
        assert!(bridge.hello().is_some());
        assert!(bridge.snapshot().is_ok());
        assert!(matches!(client.query(&queries()), Err(Error::Stale)));
        assert!(audio.poll().unwrap().is_fresh());
        assert_eq!(server.join().unwrap().len(), 5);
    }
}

#[test]
fn preflight_failures_and_pending_writes_revoke_existing_needs_facts() {
    let (_bridge, mut client) = paired(Readiness::Ready);
    let batch = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
    assert!(matches!(client.query(&[]), Err(Error::Invalid)));
    assert!(!batch.is_fresh());
    let now = now_ms().unwrap();
    let reply = crate::tests::json_response(wire(Readiness::Ready, now));
    let (mut bridge, mut client, server) = network_client(vec![reply]);
    let batch = client.query(&queries()).unwrap();
    let command = Command::LogbookCreate {
        qso: crate::tests::queued_draft().qso().clone(),
    };
    // Even the conservative local write fence precedes the denied capability;
    // no mutation endpoint is reached by this fixture.
    assert!(matches!(
        bridge.command("00000000-0000-4000-8000-000000000001", command),
        Err(Error::Capability)
    ));
    assert!(!batch.is_fresh());
    assert_eq!(server.join().unwrap().len(), 3);
}

#[test]
fn remaining_lifetime_is_floored_bounded_and_never_renews_from_observation() {
    let (bridge, mut client) = paired(Readiness::Ready);
    let mut batch = accept(&mut client, wire(Readiness::Ready, NOW)).unwrap();
    assert!((1..=2000).contains(&batch.remaining_valid_ms()));
    batch.expires = Instant::now() + Duration::from_millis(1400);
    batch.authority.expires = Instant::now() + Duration::from_millis(700);
    let first = batch.remaining_valid_ms();
    assert!((1..=700).contains(&first));
    let mut newer = metadata(Readiness::Ready, NOW + 1);
    newer.max_age_ms = 2000;
    bridge
        .needs_authority
        .lock()
        .unwrap()
        .observe(Some(&newer), NOW + 1)
        .unwrap();
    assert!(batch.remaining_valid_ms() <= first);
    batch.expires = Instant::now() + Duration::from_micros(999);
    assert_eq!(batch.remaining_valid_ms(), 0);
    batch.expires = Instant::now() + Duration::from_secs(1);
    bridge.invalidate_needs();
    assert_eq!(batch.remaining_valid_ms(), 0);
}
