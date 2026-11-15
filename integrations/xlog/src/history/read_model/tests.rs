// SPDX-License-Identifier: GPL-3.0-only
//! Synthetic contract/HTTP tests. Actual Swift producer replay is a separate gate.
use super::*;
use crate::tests::{hello, json_response, server};
use serde_json::{json, Value};

fn history_hello() -> Value {
    let mut value = hello();
    value["capabilities"] = json!(["snapshot.read", "logbook.read"]);
    value
}
fn contact() -> Value {
    json!({"contact_id":"00000000-0000-4000-8000-000000000001","callsign":"K1ABC/P","band":"20m","mode":"DATA",
        "submode":"FT4","exact_mode":"FT4","details_complete":true,"started_at_unix_ms":1_700_000_000_000u64,
        "frequency_hz":14080000,"rst_sent":"-10","rst_received":"-12","grid":"FN42","comment":"synthetic","contact_revision":2})
}
fn history() -> Value {
    json!({"protocol_version":1,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace",
        "callsign":"K1ABC","generation":"synthetic-workspace","logbook_revision":4,"max_age_ms":5000,
        "generated_at_unix_ms":now_ms().unwrap(),"contacts_limit":25,"contacts_complete":true,"matrix_complete":true,
        "contacts":[contact()],"matrix":[{"band":"20m","mode":"FT4","worked":true,"confirmed":false}],
        "history_summary":{"schema_version":1,"scope":"normal_log_base_callsign","complete":true,"total":1,
            "worked_before":true,"confirmed_count":1,"confirmation_policy":"xlog_cluster_paper_or_lotw_v1",
            "confirmation_sources":["paper_qsl","lotw"],"last_started_at_unix_ms":1_700_000_000_000u64,
            "bands":["20m"],"bands_complete":true,"modes":["FT4"],"modes_complete":true,
            "requested_band":"20m","requested_exact_mode":"FT4","requested_band_worked":true,"requested_band_exact_mode_worked":true}})
}
fn recent() -> Value {
    json!({"protocol_version":1,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace",
        "schema_version":1,"generation":"synthetic-workspace","logbook_revision":4,"max_age_ms":5000,
        "generated_at_unix_ms":now_ms().unwrap(),"contacts":[contact()],"limit":5,"total":1,"complete":true,
        "ordering":"qso_start_desc_contact_id_asc"})
}
fn validate(value: Value) -> Result<(), Error> {
    let wire: History = serde_json::from_value(value).map_err(|_| Error::Invalid)?;
    let hello = serde_json::from_value(history_hello()).unwrap();
    wire.validate(&hello, "K1ABC")?;
    wire.validate_read_model(
        &hello,
        &HistoryQuery::new("k1abc", Some("20M"), Some("ft4"))?,
    )?;
    Ok(())
}

#[test]
fn full_owner_summary_is_not_reconstructed_from_page_or_legacy_confirmation() {
    let mut value = history();
    let mut contacts = Vec::new();
    for i in 0..25 {
        let mut c = contact();
        c["contact_id"] = json!(format!("00000000-0000-4000-8000-{:012}", i + 1));
        contacts.push(c);
    }
    value["contacts"] = json!(contacts);
    value["contacts_complete"] = json!(false);
    value["history_summary"]["total"] = json!(30);
    value["history_summary"]["confirmed_count"] = json!(29);
    assert_eq!(validate(value), Ok(())); // Matrix confirmed=false is a DIFFERENT owner policy.
}

#[test]
fn semantic_contradictions_are_rejected() {
    let cases = [
        ("/history_summary/total", json!(0)),
        ("/history_summary/confirmed_count", json!(2)),
        ("/history_summary/worked_before", json!(false)),
        ("/contacts_complete", json!(false)),
        ("/contacts_limit", json!(26)),
        ("/matrix_complete", json!(false)),
        ("/history_summary/bands_complete", json!(false)),
        ("/history_summary/modes", json!(["DIGI"])),
        ("/history_summary/requested_exact_mode", json!("FT8")),
        ("/history_summary/requested_band_worked", json!(false)),
        (
            "/history_summary/requested_band_exact_mode_worked",
            json!(false),
        ),
        (
            "/history_summary/confirmation_policy",
            json!("client_guess"),
        ),
        (
            "/history_summary/confirmation_sources",
            json!(["lotw", "paper_qsl"]),
        ),
        ("/history_summary/last_started_at_unix_ms", Value::Null),
        ("/contacts/0/exact_mode", json!("DATA")),
        ("/matrix/0/worked", json!(false)),
        ("/logbook_revision", json!(SAFE + 1)),
        ("/max_age_ms", json!(5001)),
    ];
    for (path, replacement) in cases {
        let mut value = history();
        *value.pointer_mut(path).unwrap() = replacement;
        assert_eq!(validate(value), Err(Error::Invalid), "{path}");
    }
    let mut unavailable = history();
    unavailable["history_summary"]["complete"] = json!(false);
    assert_eq!(validate(unavailable), Err(Error::Stale));
    let mut foreign = history();
    foreign["generation"] = json!("another-workspace");
    assert_eq!(validate(foreign), Err(Error::IdentityChanged));
}

#[test]
fn nullable_fields_must_be_present_in_extended_payload() {
    for key in [
        "last_started_at_unix_ms",
        "requested_band",
        "requested_exact_mode",
        "requested_band_worked",
        "requested_band_exact_mode_worked",
    ] {
        let mut value = history();
        value["history_summary"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert_eq!(validate(value), Err(Error::Invalid), "{key}");
    }
    for key in [
        "contact_revision",
        "frequency_hz",
        "submode",
        "exact_mode",
        "details_complete",
    ] {
        let mut value = history();
        value["contacts"][0].as_object_mut().unwrap().remove(key);
        assert_eq!(validate(value), Err(Error::Invalid), "{key}");
    }
    let mut view_only = history();
    view_only["contacts"][0]["details_complete"] = json!(false);
    assert_eq!(validate(view_only.clone()), Err(Error::Invalid));
    view_only["contacts"][0]["contact_revision"] = Value::Null;
    assert_eq!(validate(view_only), Ok(()));
}

#[test]
fn contact_round_trip_preserves_view_only_nulls_and_cannot_launder_missing_fields() {
    let mut view_only = history();
    view_only["contacts"][0]["details_complete"] = json!(false);
    view_only["contacts"][0]["contact_revision"] = Value::Null;
    view_only["contacts"][0]["frequency_hz"] = Value::Null;
    assert_eq!(validate(view_only.clone()), Ok(()));
    let typed: History = serde_json::from_value(view_only.clone()).unwrap();
    let encoded = serde_json::to_value(&typed).unwrap();
    assert_eq!(encoded, view_only);
    assert_eq!(validate(encoded.clone()), Ok(()));
    let decoded: History = serde_json::from_value(encoded).unwrap();
    assert_eq!(typed, decoded);
    for key in ["contact_revision", "frequency_hz"] {
        let mut malformed = view_only.clone();
        malformed["contacts"][0]
            .as_object_mut()
            .unwrap()
            .remove(key);
        let typed: History = serde_json::from_value(malformed).unwrap();
        let encoded = serde_json::to_value(typed).unwrap();
        assert!(!encoded["contacts"][0]
            .as_object()
            .unwrap()
            .contains_key(key));
        assert_eq!(
            validate(encoded),
            Err(Error::Invalid),
            "serialization cannot manufacture missing {key}"
        );
    }
    let mut legacy = contact();
    for key in [
        "contact_revision",
        "frequency_hz",
        "submode",
        "exact_mode",
        "details_complete",
    ] {
        legacy.as_object_mut().unwrap().remove(key);
    }
    let typed: HistoryContact = serde_json::from_value(legacy.clone()).unwrap();
    assert_eq!(typed.validate_details(false), Ok(()));
    assert_eq!(serde_json::to_value(&typed).unwrap(), legacy);
}

#[test]
fn partial_extension_is_not_legacy_and_empty_summary_is_explicit() {
    let mut value = history();
    value.as_object_mut().unwrap().remove("matrix_complete");
    assert_eq!(validate(value), Err(Error::Invalid));
    let mut value = history();
    value["contacts"] = json!([]);
    value["matrix"] = json!([]);
    let summary = &mut value["history_summary"];
    summary["total"] = json!(0);
    summary["confirmed_count"] = json!(0);
    summary["worked_before"] = json!(false);
    summary["last_started_at_unix_ms"] = Value::Null;
    summary["bands"] = json!([]);
    summary["modes"] = json!([]);
    summary["requested_band_worked"] = json!(false);
    summary["requested_band_exact_mode_worked"] = json!(false);
    assert_eq!(validate(value), Ok(()));
}

#[test]
fn exact_aliases_and_request_encoding_do_not_collapse_digital_modes() {
    for (input, expected) in [
        ("USB", "SSB"),
        ("LSB", "SSB"),
        ("PHONE", "SSB"),
        ("PH", "SSB"),
        ("CWR", "CW"),
        ("RTTYR", "RTTY"),
        ("FT8", "FT8"),
        ("FT4", "FT4"),
    ] {
        let query = HistoryQuery::new("k1abc/p", Some("20M"), Some(input)).unwrap();
        assert_eq!(
            query.path(),
            format!("/xlog/v1/history?callsign=K1ABC%2FP&band=20m&mode={expected}")
        );
    }
    assert!(HistoryQuery::new("K1ABC", None, Some("FT8")).is_err());
    assert!(HistoryQuery::new("K1ABC", Some("20m&mode=CW"), None).is_err());
    assert!(HistoryQuery::new("K1ABC", Some("20m"), Some("FT8&x=1")).is_err());
}

#[test]
fn recent_count_completeness_order_and_details_are_consistent() {
    let h: Hello = serde_json::from_value(history_hello()).unwrap();
    let r: Recent = serde_json::from_value(recent()).unwrap();
    assert_eq!(r.validate(&h), Ok(()));
    for (field, value) in [
        ("total", json!(2)),
        ("complete", json!(false)),
        ("limit", json!(6)),
        ("ordering", json!("arrival")),
    ] {
        let mut r = recent();
        r[field] = value;
        let r: Recent = serde_json::from_value(r).unwrap();
        assert_eq!(r.validate(&h), Err(Error::Invalid));
    }
    let mut r = recent();
    let first = contact();
    let mut second = contact();
    second["contact_id"] = json!("00000000-0000-4000-8000-000000000002");
    r["contacts"] = json!([second, first]);
    r["total"] = json!(2);
    let r: Recent = serde_json::from_value(r).unwrap();
    assert_eq!(r.validate(&h), Err(Error::Invalid));
}

#[test]
fn independent_read_handle_is_revoked_by_disconnect_and_never_reconnects_itself() {
    let (mut bridge, owner) = server(vec![
        json_response(history_hello()),
        json_response(history()),
        json_response(history_hello()),
    ]);
    bridge.connect().unwrap();
    let mut client = bridge.history_handle().unwrap();
    let query = HistoryQuery::new("K1ABC", Some("20m"), Some("FT4")).unwrap();
    let result = client.history(&query).unwrap();
    assert!(result.value().is_ok());
    bridge.disconnect();
    assert_eq!(result.remaining_valid_ms(), 0);
    bridge.connect().unwrap();
    assert_eq!(client.history(&query).err(), Some(Error::Disconnected));
    assert_eq!(owner.join().unwrap().len(), 3);
}

#[test]
fn optional_recent_absence_is_not_empty_and_does_not_disconnect_station() {
    let not_found =
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned();
    let (mut bridge, owner) = server(vec![
        json_response(history_hello()),
        not_found,
        json_response(history()),
    ]);
    bridge.connect().unwrap();
    let mut client = bridge.history_handle().unwrap();
    assert_eq!(client.recent().err(), Some(Error::Http(404)));
    assert!(bridge.hello().is_some());
    assert!(client
        .history(&HistoryQuery::new("K1ABC", Some("20m"), Some("FT4")).unwrap())
        .is_ok());
    assert_eq!(owner.join().unwrap().len(), 3);
}

#[test]
fn legacy_empty_page_remains_without_summary_and_extended_recent_has_real_total() {
    let legacy = json!({"protocol_version":1,"server_instance":"synthetic-server","workspace_id":"synthetic-workspace",
        "callsign":"K1ABC","generated_at_unix_ms":now_ms().unwrap(),"contacts":[],"matrix":[]});
    let (mut bridge, owner) = server(vec![
        json_response(history_hello()),
        json_response(legacy),
        json_response(recent()),
    ]);
    bridge.connect().unwrap();
    let mut client = bridge.history_handle().unwrap();
    let old = client
        .history(&HistoryQuery::new("K1ABC", None, None).unwrap())
        .unwrap();
    assert!(old.value().unwrap().history_summary.is_none());
    assert!(old.value().unwrap().contacts.is_empty());
    let current = client.recent().unwrap();
    assert_eq!(current.value().unwrap().total, 1);
    assert_eq!(current.value().unwrap().generation, "synthetic-workspace");
    assert!(current.remaining_valid_ms() <= 5000);
    assert_eq!(
        old.remaining_valid_ms(),
        0,
        "revision-scoped response retires unversioned legacy evidence"
    );
    let requests = owner.join().unwrap();
    assert!(requests[2].starts_with("GET /xlog/v1/recent HTTP/1.1"));
}

#[test]
fn foreign_read_identity_and_revocation_retire_existing_session() {
    for status in [401, 403] {
        let refusal =
            format!("HTTP/1.1 {status} Refused\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let (mut bridge, owner) = server(vec![json_response(history_hello()), refusal]);
        bridge.connect().unwrap();
        let mut client = bridge.history_handle().unwrap();
        assert_eq!(client.recent().err(), Some(Error::Http(status)));
        assert!(bridge.hello().is_none());
        owner.join().unwrap();
    }
    for key in ["server_instance", "workspace_id", "generation"] {
        let mut foreign = recent();
        foreign[key] = json!("foreign");
        let (mut bridge, owner) =
            server(vec![json_response(history_hello()), json_response(foreign)]);
        bridge.connect().unwrap();
        let mut client = bridge.history_handle().unwrap();
        assert_eq!(client.recent().err(), Some(Error::IdentityChanged));
        assert!(bridge.hello().is_none());
        owner.join().unwrap();
    }
}

#[test]
fn request_elapsed_repeat_timestamp_and_revision_never_extend_retained_lease() {
    let (mut bridge, owner) = server(vec![json_response(history_hello())]);
    bridge.connect().unwrap();
    let mut client = bridge.history_handle().unwrap();
    let generated = now_ms().unwrap() + 1000;
    let first = client
        .lease(
            (),
            generated,
            5000,
            Some(4),
            Instant::now() - Duration::from_millis(4000),
        )
        .unwrap();
    assert!(first.remaining_valid_ms() <= 1000);
    let second = client
        .lease((), generated, 5000, Some(4), Instant::now())
        .unwrap();
    assert!(second.expires <= first.expires);
    let newer = client
        .lease((), generated, 5000, Some(5), Instant::now())
        .unwrap();
    assert_eq!(first.remaining_valid_ms(), 0);
    assert!(newer.value().is_ok());
    assert_eq!(
        client
            .lease((), generated, 5000, Some(4), Instant::now())
            .err(),
        Some(Error::Revision)
    );
    client.disconnect();
    assert_eq!(newer.remaining_valid_ms(), 0);
    assert_eq!(client.recent().err(), Some(Error::Disconnected));
    assert_eq!(client.recent().err(), Some(Error::Disconnected)); // finish() must not revive a locally stopped handle.
    owner.join().unwrap();
}

#[test]
#[ignore = "Requires both unchanged synthetic Swift producer exports and external SHA-256 provenance"]
fn actual_swift_history_and_lifecycle_exports_validate_without_rewriting_facts() {
    fn load(name: &str) -> Value {
        let path = std::env::var(name)
            .expect("Explicit producer export path required; no missing-fixture pass");
        let bytes = std::fs::read(path).expect("Producer export must exist");
        assert!(!bytes.is_empty() && bytes.len() <= RESPONSE_BYTES);
        let value: Value = serde_json::from_slice(&bytes).expect("Actual producer JSON");
        assert_eq!(value["fixture_schema"], json!(1));
        assert_eq!(value["synthetic"], json!(true));
        value
    }
    // These exports are the actual Swift payload BEFORE Rust's public identity
    // wrapper. Add only that explicitly synthetic wrapper; never rewrite facts,
    // generation, revisions, counts, modes, timestamps, or completeness flags.
    fn wrap(raw: &Value) -> (Value, Hello) {
        let mut raw = raw.clone();
        let object = raw.as_object_mut().expect("Producer response object");
        for key in ["protocol_version", "server_instance", "workspace_id"] {
            assert!(!object.contains_key(key));
        }
        let generation = object["generation"]
            .as_str()
            .expect("Producer generation")
            .to_owned();
        object.insert("protocol_version".into(), json!(1));
        object.insert(
            "server_instance".into(),
            json!("synthetic-swift-history-replay"),
        );
        object.insert("workspace_id".into(), json!(generation));
        let hello = Hello {
            application: "OM0RX-xLog".into(),
            protocol_version: 1,
            server_instance: "synthetic-swift-history-replay".into(),
            workspace_id: generation,
            capabilities: vec!["logbook.read".into()],
        };
        (raw, hello)
    }
    fn check_history(raw: &Value, call: &str, band: Option<&str>, mode: Option<&str>) -> History {
        let (wrapped, hello) = wrap(raw);
        let value: History = serde_json::from_value(wrapped).unwrap();
        let query = HistoryQuery::new(call, band, mode).unwrap();
        // Historical clock is explicit contract replay, not a live lease.
        let anchor = value.generated_at_unix_ms;
        value.validate_at(&hello, call, anchor).unwrap();
        assert!(value
            .validate_read_model_at(&hello, &query, anchor)
            .unwrap()
            .is_some());
        assert_eq!(
            value.validate_read_model_at(&hello, &query, anchor + 5001),
            Err(Error::Stale)
        );
        if now_ms().unwrap().saturating_sub(anchor) >= 5000 {
            assert_eq!(value.validate_read_model(&hello, &query), Err(Error::Stale));
        }
        value
    }
    fn check_recent(raw: &Value) -> Recent {
        let (wrapped, hello) = wrap(raw);
        let value: Recent = serde_json::from_value(wrapped).unwrap();
        let anchor = value.generated_at_unix_ms;
        value.validate_at(&hello, anchor).unwrap();
        assert_eq!(value.validate_at(&hello, anchor + 5001), Err(Error::Stale));
        if now_ms().unwrap().saturating_sub(anchor) >= 5000 {
            assert_eq!(value.validate(&hello), Err(Error::Stale));
        }
        value
    }
    let main = load("XLOG_HISTORY_FIXTURE_INPUT");
    let lifecycle = load("XLOG_HISTORY_LIFECYCLE_INPUT");
    let full = check_history(&main["history"], "VK8RX", Some("20m"), Some("FT4"));
    let recent = check_recent(&main["recent"]);
    assert_eq!(full.generation.as_deref(), Some(recent.generation.as_str()));
    assert_eq!(full.logbook_revision, Some(recent.logbook_revision));
    assert_eq!(full.history_summary.as_ref().unwrap().total, 30);
    assert_eq!(full.contacts.len(), 25);
    assert_eq!(recent.total, 31);
    let combining = check_history(&main["combining_comment"], "K1ABC", None, None);
    assert_eq!(combining.contacts[0].details_complete, Some(false));
    assert_eq!(combining.contacts[0].contact_revision, None);
    let empty = check_recent(&lifecycle["empty_recent"]);
    assert_eq!(empty.total, 0);
    let before = check_history(&lifecycle["band_only_history"], "VK8RX", Some("20m"), None);
    let edited = check_history(&lifecycle["after_edit_history"], "VK8RX", Some("20m"), None);
    let deleted = check_recent(&lifecycle["after_delete_recent"]);
    assert_eq!(before.generation, edited.generation);
    assert_eq!(
        before.generation.as_deref(),
        Some(deleted.generation.as_str())
    );
    assert!(before.logbook_revision.unwrap() < edited.logbook_revision.unwrap());
    assert!(edited.logbook_revision.unwrap() < deleted.logbook_revision);
    assert_eq!(deleted.total, 0);
    let retired = lifecycle["wrong_generation"]
        .as_array()
        .expect("Both retired-owner responses required");
    assert_eq!(retired.len(), 2);
    for response in retired {
        assert_eq!(response["status"], json!("workspace_changed"));
        assert!(!response.as_object().unwrap().contains_key("contacts"));
    }
    println!("Actual Swift history contract replay: 7 accepted payloads +2 retired-workspace refusals; historical clock only, original timestamps unchanged.");
}
