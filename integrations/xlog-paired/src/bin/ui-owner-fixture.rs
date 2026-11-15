// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Disposable native UI fixture. Production HTTP transport, synthetic station owner.
#![allow(dead_code)]
include!(concat!(env!("OUT_DIR"), "/owner_transport.rs"));
use companion_bridge::Bridge;
#[cfg(feature = "owner-audio")]
#[path = "../ui_fixture_audio.rs"]
mod ui_fixture_audio;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
struct Owner {
    revision: u64,
    frequency: u64,
    azimuth: f64,
    contacts: HashMap<String, Value>,
    receipts: HashMap<String, Value>,
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(
        args.len(),
        1,
        "one explicit --fixture-root coordinate required"
    );
    let root = PathBuf::from(
        args[0]
            .strip_prefix("--fixture-root=")
            .expect("explicit fixture coordinate required"),
    );
    assert!(root.is_absolute(), "absolute fixture root required");
    let input_metadata = std::fs::symlink_metadata(&root).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            input_metadata.file_attributes() & 0x400,
            0,
            "no reparse-point fixture"
        );
    }
    assert!(input_metadata.is_dir() && !input_metadata.file_type().is_symlink());
    let root = root
        .canonicalize()
        .expect("prepare a private marked fixture first");
    assert_eq!(
        root.parent(),
        Some(std::env::temp_dir().canonicalize().unwrap().as_path()),
        "fixture must be directly under TEMP"
    );
    let metadata = std::fs::symlink_metadata(&root).unwrap();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        assert_eq!(
            metadata.file_attributes() & 0x400,
            0,
            "no reparse-point fixture"
        );
    }
    assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
    let marker_path = root.join(".xnexus-ui-fixture.json");
    let marker_metadata = std::fs::symlink_metadata(&marker_path).unwrap();
    assert!(
        marker_metadata.is_file()
            && !marker_metadata.file_type().is_symlink()
            && marker_metadata.len() <= 1024
    );
    let marker: Value = serde_json::from_slice(&std::fs::read(&marker_path).unwrap()).unwrap();
    assert_eq!(marker["schema"], 1);
    assert_eq!(marker["synthetic"], true);
    let synthetic_audio = marker
        .get("synthetic_audio")
        .map(|v| v.as_str().expect("synthetic audio kind"))
        .unwrap_or("off");
    assert!(
        matches!(synthetic_audio, "off" | "silence48k" | "generated-ft4"),
        "known synthetic audio kind required"
    );
    let has_audio = synthetic_audio != "off";
    let has_needs = synthetic_audio == "generated-ft4";
    #[cfg(not(feature = "owner-audio"))]
    assert_eq!(
        synthetic_audio, "off",
        "rebuild with owner-audio for synthetic PCM"
    );
    let id = uuid::Uuid::parse_str(marker["id"].as_str().unwrap()).unwrap();
    assert_eq!(
        root.file_name().unwrap().to_str().unwrap(),
        format!("xnexus-ui-{id}")
    );
    assert!(
        !root.join("pairing.json").exists(),
        "fresh fixture required"
    );
    assert!(
        !root.join("owner.json").exists()
            && !root.join("events.jsonl").exists()
            && !root.join("STOP").exists(),
        "fresh fixture required"
    );
    assert!(
        !root.join("STOP_AUDIO").exists(),
        "fresh audio fixture required"
    );
    #[cfg(feature = "owner-audio")]
    let mut audio = has_audio.then(|| {
        if has_needs {
            let path = root.join("generated-ft4.s16le");
            let meta = std::fs::symlink_metadata(&path).unwrap();
            assert!(meta.is_file() && !meta.file_type().is_symlink() && meta.len() == 720_000);
            #[cfg(windows)] {
                use std::os::windows::fs::MetadataExt;
                assert_eq!(meta.file_attributes() & 0x400, 0, "no waveform reparse point");
            }
            let bytes = std::fs::read(path).unwrap();
            let samples = bytes.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect();
            ui_fixture_audio::Audio::start_period(Some(samples))
        } else { ui_fixture_audio::Audio::start() }
    });
    #[cfg(feature = "owner-audio")]
    let audio_buffer = audio.as_ref().map(|audio| audio.buffer.clone());
    let owner = Arc::new(Mutex::new(Owner {
        revision: 0,
        frequency: 14074000,
        azimuth: 45.0,
        contacts: HashMap::new(),
        receipts: HashMap::new(),
    }));
    let evidence = root.join("events.jsonl");
    let needs_stop = root.join("STOP_NEEDS");
    assert!(!needs_stop.exists(), "fresh needs fixture required");
    let bridge=Bridge::start("xnexus-disposable-ui-fixture".into(),Arc::new(move|request|{
        use std::io::Write;
        let mut state=owner.lock().unwrap();
        if let Some(request)=request{
            if let Some(batch) = request.get("needs_query") {
                let mut event=std::fs::OpenOptions::new().create(true).append(true).open(&evidence).unwrap();
                writeln!(event,"{}",json!({"synthetic_needs_query":batch,"at_unix_ms":now()})).unwrap();
                event.sync_all().unwrap();
                return Ok(synthetic_needs(batch, has_needs && !needs_stop.exists()));
            }
            #[cfg(feature = "owner-audio")]
            if let (Some(after), Some(buffer)) = (request["audio_after"].as_u64(), audio_buffer.as_ref()) { return Ok(buffer.lock().unwrap().response(after, now())); }
            if let Some(after)=request["audio_after"].as_u64(){return Ok(json!({"generated_at_unix_ms":now(),"max_age_ms":1000,"state":"stopped","reason":"Synthetic control fixture has no audio capture","descriptor":null,"oldest_sequence":1,"next_sequence":after+1,"gap":false,"chunks":[]}));}
            if let Some(id)=request["receipt_lookup"].as_str(){return Ok(state.receipts.get(id).cloned().unwrap_or(Value::Null));}
            if let Some(call)=request["history_lookup"].as_str(){
                return Ok(json!({"callsign":call,"generated_at_unix_ms":now(),"contacts":state.contacts.values().filter(|q|q["callsign"]==call).cloned().collect::<Vec<_>>(),"matrix":[{"band":"20m","mode":"FT8","worked":true,"confirmed":false}]}));
            }
            let id=request["request_id"].as_str().unwrap().to_string();
            let cmd=&request["command"];
            let mut event=std::fs::OpenOptions::new().create(true).append(true).open(&evidence).unwrap();
            writeln!(event,"{}",request).unwrap();event.sync_all().unwrap();
            let result=match cmd["kind"].as_str().unwrap(){
                "station.tune"=>{
                    #[cfg(feature = "owner-audio")]
                    if let Some(buffer) = audio_buffer.as_ref() { buffer.lock().unwrap().stop("Synthetic QSY"); }
                    state.frequency=cmd["frequency_hz"].as_u64().unwrap();json!({"status":"observed","message":"Synthetic radio only"})},
                "rotor.point"=>{state.azimuth=cmd["azimuth_deg"].as_f64().unwrap();json!({"status":"observed","message":"Synthetic rotor only"})},
                "rotor.stop"=>json!({"status":"observed","message":"Synthetic rotor stopped"}),
                "logbook.create"|"logbook.update"=>{
                    let mut q=cmd["qso"].clone();let contact=q["contact_id"].as_str().unwrap().to_string();
                    let old=state.contacts.get(&contact).map(|v|v["contact_revision"].as_u64().unwrap());
                    let allowed=if cmd["kind"]=="logbook.create"{old.is_none()}else{old==cmd["expected_contact_revision"].as_u64()};
                    if !allowed{json!({"status":"rejected","message":"Synthetic contact conflict"})}else{
                        let revision=old.unwrap_or(0)+1;q["contact_revision"]=json!(revision);state.contacts.insert(contact.clone(),q);
                        json!({"status":"applied","contact_id":contact,"contact_revision":revision,"message":"Saved in disposable fixture memory only"})
                    }
                },
                _=>json!({"status":"rejected","message":"Unsupported fixture command"})
            };
            state.receipts.insert(id,result.clone());return Ok(result);
        }
        state.revision+=1;
        let mut snapshot = json!({"revision":state.revision,"generated_at_unix_ms":now(),"max_age_ms":3000,
            "station":{"connected":true,"radio_id":if has_audio {"radio1"} else {"synthetic-radio"},"frequency_hz":state.frequency,"mode":if has_audio {"DIGU"} else {"FT8"},"transmitting":false},
            "rotor":{"connected":true,"rotor_id":"synthetic-rotor","azimuth_deg":state.azimuth,"moving":false},
            "spots":[{"id":"fixture-W1AW","callsign":"W1AW","frequency_hz":14074000,"mode":"FT8","observed_at_unix_ms":now(),"source":"Disposable synthetic cluster","country":"United States","adif":291,"latitude":41.7,"longitude":-72.7,"band":"20m","call_status":"New Call","dxcc_status":"ATNO","need":{"status":"needed","score":100,"reasons":["Synthetic new entity for acceptance"]}}]});
        if has_needs {
            let ready = !needs_stop.exists();
            snapshot["needs"] = json!({"generated_at_unix_ms":now(),"max_age_ms":2000,
                "readiness":if ready {"ready"} else {"unavailable"},
                "authority":if ready { synthetic_authority() } else {Value::Null}});
        }
        Ok(snapshot)
    })).unwrap();
    let pairing = json!({"application":"OM0RX-xLog","protocol_version":1,"endpoint":bridge.endpoint,"token":bridge.token});
    use std::io::Write;
    for (name,bytes) in [("pairing.json",serde_json::to_vec_pretty(&pairing).unwrap()),("owner.json",serde_json::to_vec(&json!({"pid":std::process::id(),"root":root,"synthetic":true,"endpoint":bridge.endpoint})).unwrap())] {
        let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(root.join(name)).unwrap();
        file.write_all(&bytes).unwrap(); file.sync_all().unwrap();
    }
    println!(
        "Synthetic owner ready; pairing path: {}",
        root.join("pairing.json").display()
    );
    while !root.join("STOP").exists() {
        #[cfg(feature = "owner-audio")]
        if root.join("STOP_AUDIO").exists() {
            if let Some(audio) = audio.as_mut() {
                audio.stop();
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    bridge.revoke();
}

fn synthetic_authority() -> Value {
    json!({"logbook_revision":1,"classification_revision":1,
        "country_reference_revision":"xlog-country-facts-v3:synthetic-ui",
        "policy_revision":"xlog-call-dxcc-needs-v1"})
}
fn synthetic_needs(batch: &Value, ready: bool) -> Value {
    let results: Vec<_> = batch["queries"].as_array().unwrap().iter().map(|q| {
        let known = ready && q["callsign"] == "K1ABC" && q["mode"] == "FT4" && q["dial_frequency_hz"] == 14_074_000_u64;
        json!({"query_id":q["query_id"],"normalized_callsign":q["callsign"],
            "dial_frequency_hz":q["dial_frequency_hz"],"decoder_mode":q["mode"],
            "classification_mode":"DIGI","band":if known {Some("20m")} else {None},
            "country":if known {Some("United States")} else {None},
            "adif":if known {Some(291)} else {None},
            "country_resolution":if known {Some("resolved")} else {None},
            "call_status":if known {"New Call"} else {"Unknown"},
            "dxcc_status":if known {"ATNO"} else {"Unknown"},
            "need":{"status":if known {"needed"} else {"unknown"},
                "score":if known {Some(100)} else {None},
                "reason_code":if known {None} else {Some("unavailable")},
                "reasons":if known {vec!["Synthetic classification fixture; no real logbook"]} else {vec![]}}})
    }).collect();
    json!({"generated_at_unix_ms":now(),"max_age_ms":2000,
        "readiness":if ready {"ready"} else {"unavailable"},
        "authority":if ready {synthetic_authority()} else {Value::Null},"results":results})
}
