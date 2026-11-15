use super::*;
use crate::{
    tests::{hello, json_response, server, snapshot},
    Hello, Snapshot,
};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

const WORKSPACE: &str = "00000000-0000-4000-8000-000000000001";
const EPOCH: &str = "00000000-0000-4000-8000-000000000002";

// Manually authored protocol fixtures, NOT actual owner-projector exports.
fn identity() -> Value {
    json!({"schema_version":1,"source":"saved_station_settings","state":"current",
        "mycall":"VK8RX/P","mygrid":"PH57AB","transmit_authority":false,
        "binding":{"workspace_generation":WORKSPACE,"worker_generation":7,
            "settings_revision":11,"identity_epoch":EPOCH,"identity_revision":3,
            "operating_module":"normal_logging"}})
}
fn bound_snapshot() -> Value {
    let mut value = snapshot();
    value["workspace_id"] = json!(WORKSPACE);
    value["station_identity"] = identity();
    value
}
fn bound_hello() -> Value {
    let mut value = hello();
    value["workspace_id"] = json!(WORKSPACE);
    value
}
fn validate(value: Value) -> bool {
    let h: Hello = serde_json::from_value(bound_hello()).unwrap();
    serde_json::from_value::<Snapshot>(value)
        .is_ok_and(|value| value.validate(&h, value.generated_at_unix_ms).is_ok())
}

#[test]
fn old_owner_absence_roundtrips_without_new_wire_fields() {
    let original = snapshot();
    let parsed: Snapshot = serde_json::from_value(original.clone()).unwrap();
    assert!(parsed.station_identity.is_none());
    assert!(serde_json::to_value(&parsed)
        .unwrap()
        .get("station_identity")
        .is_none());
    let h: Hello = serde_json::from_value(hello()).unwrap();
    assert!(parsed.validate(&h, parsed.generated_at_unix_ms).is_ok());
    let mut null = original;
    null["station_identity"] = Value::Null;
    assert!(serde_json::from_value::<Snapshot>(null)
        .unwrap()
        .station_identity
        .is_none());
}

#[test]
fn exact_owner_forms_and_state_shapes_are_accepted() {
    let current = bound_snapshot();
    assert!(validate(current.clone()));
    let parsed: Snapshot = serde_json::from_value(current.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&parsed).unwrap()["station_identity"],
        identity()
    );
    for missing in [vec!["mycall"], vec!["mygrid"], vec!["mycall", "mygrid"]] {
        let mut value = current.clone();
        value["station_identity"]["state"] = json!("incomplete");
        for key in missing {
            value["station_identity"][key] = Value::Null;
        }
        assert!(validate(value));
    }
    let mut unavailable = current.clone();
    unavailable["station_identity"]["state"] = json!("unavailable");
    for key in ["mycall", "mygrid", "binding"] {
        unavailable["station_identity"][key] = Value::Null;
    }
    assert!(validate(unavailable));
    for call in [
        "GB50",
        "9A2024",
        "VK8RX/P",
        &format!("A1{}", "A".repeat(62)),
    ] {
        let mut value = current.clone();
        value["station_identity"]["mycall"] = json!(call);
        assert!(validate(value), "producer call {call}");
    }
    for grid in ["AA00", "RR99XX", "JN88TQ12"] {
        let mut value = current.clone();
        value["station_identity"]["mygrid"] = json!(grid);
        assert!(validate(value), "producer grid {grid}");
    }
    for revision in [0, MAX_SAFE] {
        let mut value = current.clone();
        value["station_identity"]["binding"]["settings_revision"] = json!(revision);
        value["station_identity"]["binding"]["identity_revision"] = json!(revision);
        assert!(validate(value));
    }
}

#[test]
fn malformed_foreign_or_transmit_grant_identity_never_validates() {
    let original = bound_snapshot();
    let mutations = [
        ("/schema_version", json!(2)),
        ("/source", json!("operator_call")),
        ("/transmit_authority", json!(true)),
        ("/state", json!("guess")),
        ("/mycall", json!("vk8rx")),
        ("/mycall", json!("ß1")),
        ("/mycall", json!("A1".repeat(33))),
        ("/mycall", json!("UNKNOWN")),
        ("/mycall", json!("/VK8RX")),
        ("/mycall", json!("VK8RX/")),
        ("/mycall", json!("VK8RX//P")),
        ("/mycall", json!("VK8RX\n")),
        ("/mycall", Value::Null),
        ("/mygrid", Value::Null),
        ("/mygrid", json!("ph57ab")),
        ("/mygrid", json!("AA00YZ")),
        ("/mygrid", json!("SS00")),
        ("/mygrid", json!("JN88T")),
        ("/mygrid", json!("JN88TQ123")),
        ("/mygrid", json!(" JN88")),
        ("/state", json!("incomplete")),
        ("/state", json!("unavailable")),
        ("/binding", Value::Null),
        ("/binding/workspace_generation", json!(EPOCH)),
        ("/binding/workspace_generation", json!("invalid")),
        (
            "/binding/identity_epoch",
            json!("00000000-0000-0000-0000-000000000000"),
        ),
        ("/binding/identity_epoch", json!("not-a-uuid")),
        ("/binding/worker_generation", json!(0)),
        ("/binding/worker_generation", json!(MAX_SAFE + 1)),
        ("/binding/settings_revision", json!(MAX_SAFE + 1)),
        ("/binding/identity_revision", json!(MAX_SAFE + 1)),
        ("/binding/operating_module", json!("contesting")),
    ];
    for (pointer, replacement) in mutations {
        let mut value = original.clone();
        *value["station_identity"].pointer_mut(pointer).unwrap() = replacement;
        assert!(!validate(value), "must reject {pointer}");
    }
    for workspace in ["invalid", "00000000-0000-0000-0000-000000000000"] {
        let mut value: StationIdentity = serde_json::from_value(identity()).unwrap();
        value.binding.as_mut().unwrap().workspace_generation = workspace.into();
        assert_eq!(value.validate(workspace), Err(Error::Invalid));
    }
}

#[test]
fn identity_inherits_snapshot_expiry_and_reconnect_drops_prior_identity() {
    let first = bound_snapshot();
    let parsed: Snapshot = serde_json::from_value(first.clone()).unwrap();
    let h: Hello = serde_json::from_value(bound_hello()).unwrap();
    assert_eq!(
        parsed.validate(&h, parsed.generated_at_unix_ms + parsed.max_age_ms),
        Err(Error::Stale)
    );
    let mut later = first.clone();
    later.as_object_mut().unwrap().remove("station_identity");
    later["revision"] = json!(1);
    let (mut client, handle) = server(vec![
        json_response(bound_hello()),
        json_response(first),
        json_response(bound_hello()),
        json_response(later),
    ]);
    client.connect().unwrap();
    assert!(client.poll().unwrap().station_identity.is_some());
    // Deterministic expiry of the REAL SDK observation; no sleeps/new TTL.
    client.observation.as_mut().unwrap().received = Instant::now() - Duration::from_secs(20);
    assert_eq!(client.snapshot().unwrap_err(), Error::Stale);
    client.connect().unwrap();
    assert_eq!(client.snapshot().unwrap_err(), Error::Disconnected);
    assert!(client.poll().unwrap().station_identity.is_none());
    client.disconnect();
    assert_eq!(client.snapshot().unwrap_err(), Error::Disconnected);
    assert_eq!(handle.join().unwrap().len(), 4);
}

#[test]
#[ignore = "Requires a separately recorded actual companion_identity::project corpus; no manual replacement"]
fn actual_owner_projector_corpus_validates_without_rewriting_identity() {
    let path = std::env::var_os("XLOG_IDENTITY_FIXTURE_INPUT").expect("explicit owner-export path");
    let corpus: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(corpus["fixture_schema"], 1);
    assert_eq!(corpus["synthetic"], true); // Actual projector, synthetic saved settings.
    assert_eq!(corpus["producer"], "companion_identity::project");
    let workspace = corpus["workspace_id"].as_str().unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let original = &case["station_identity"];
        let identity: StationIdentity = serde_json::from_value(original.clone()).unwrap();
        assert_eq!(identity.validate(workspace), Ok(()), "{}", case["name"]);
        assert_eq!(
            serde_json::to_value(identity).unwrap(),
            *original,
            "unchanged owner DTO"
        );
    }
}

#[test]
fn identity_change_requires_base_revision_and_invalid_reply_retires_session() {
    for change in [json!("W1AW"), json!("bad call")] {
        let first = bound_snapshot();
        let mut second = first.clone();
        second["station_identity"]["mycall"] = change;
        let (mut client, handle) = server(vec![
            json_response(bound_hello()),
            json_response(first),
            json_response(second),
        ]);
        client.connect().unwrap();
        client.poll().unwrap();
        assert!(matches!(
            client.poll(),
            Err(Error::Revision | Error::Invalid)
        ));
        assert!(client.hello().is_none());
        assert_eq!(client.snapshot().unwrap_err(), Error::Disconnected);
        assert_eq!(handle.join().unwrap().len(), 3);
    }
    let first = bound_snapshot();
    let mut second = first.clone();
    second["revision"] = json!(6);
    second["station_identity"]["mycall"] = json!("W1AW");
    second["station_identity"]["binding"]["settings_revision"] = json!(12);
    let (mut client, handle) = server(vec![
        json_response(bound_hello()),
        json_response(first),
        json_response(second),
    ]);
    client.connect().unwrap();
    client.poll().unwrap();
    assert_eq!(
        client
            .poll()
            .unwrap()
            .station_identity
            .as_ref()
            .unwrap()
            .mycall
            .as_deref(),
        Some("W1AW")
    );
    assert_eq!(handle.join().unwrap().len(), 3);
}
