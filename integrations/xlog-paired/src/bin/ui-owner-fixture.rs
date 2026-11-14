// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Disposable native UI fixture. Production HTTP transport, synthetic station owner.
#![allow(dead_code)]
include!(concat!(env!("OUT_DIR"), "/owner_transport.rs"));
use companion_bridge::Bridge;
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
    let owner = Arc::new(Mutex::new(Owner {
        revision: 0,
        frequency: 14074000,
        azimuth: 45.0,
        contacts: HashMap::new(),
        receipts: HashMap::new(),
    }));
    let evidence = root.join("events.jsonl");
    let bridge=Bridge::start("xnexus-disposable-ui-fixture".into(),Arc::new(move|request|{
        use std::io::Write;
        let mut state=owner.lock().unwrap();
        if let Some(request)=request{
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
                "station.tune"=>{state.frequency=cmd["frequency_hz"].as_u64().unwrap();json!({"status":"observed","message":"Synthetic radio only"})},
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
        Ok(json!({"revision":state.revision,"generated_at_unix_ms":now(),"max_age_ms":3000,
            "station":{"connected":true,"radio_id":"synthetic-radio","frequency_hz":state.frequency,"mode":"FT8","transmitting":false},
            "rotor":{"connected":true,"rotor_id":"synthetic-rotor","azimuth_deg":state.azimuth,"moving":false},
            "spots":[{"id":"fixture-W1AW","callsign":"W1AW","frequency_hz":14074000,"mode":"FT8","observed_at_unix_ms":now(),"source":"Disposable synthetic cluster","country":"United States","adif":291,"latitude":41.7,"longitude":-72.7,"band":"20m","call_status":"New Call","dxcc_status":"ATNO","need":{"status":"needed","score":100,"reasons":["Synthetic new entity for acceptance"]}}]}))
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
        std::thread::sleep(Duration::from_millis(200));
    }
    bridge.revoke();
}
