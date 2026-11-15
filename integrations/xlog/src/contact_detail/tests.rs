use super::*;
use serde_json::{json, Value};

fn cell(value: Value) -> Value {
    json!({"state": if value.is_null() {"empty"} else {"value"}, "value":value,"reason":null})
}
fn unavailable(reason: &str) -> Value {
    json!({"state":"unavailable","value":null,"reason":reason})
}
fn empty_group(names: &str) -> Value {
    Value::Object(
        names
            .split_whitespace()
            .map(|n| (n.into(), cell(Value::Null)))
            .collect(),
    )
}
fn fixture() -> Value {
    let mut contact=empty_group("callsign band mode submode started_at_unix_ms ended_at_unix_ms stored_time_unit original_start_precision original_end_precision raw_qso_date raw_time_on raw_qso_date_off raw_time_off rst_sent rst_received grid comment notes");
    for (k, v) in [
        ("callsign", json!("N0TEST")),
        ("band", json!("20m")),
        ("mode", json!("MFSK")),
        ("submode", json!("FT8")),
        ("started_at_unix_ms", json!(1_700_000_000_000_u64)),
        ("stored_time_unit", json!("unix_milliseconds")),
        ("rst_sent", json!("-05")),
        ("rst_received", json!("-08")),
        ("comment", json!("first\nsecond")),
    ] {
        contact[k] = cell(v);
    }
    let mut confirmations = json!({});
    for source in ["paper", "lotw", "eqsl", "qrz"] {
        let mut value=empty_group("source evidence_kind sent_status sent_date sent_via received_status received_date received_via match_age_seconds");
        value["source"] = cell(json!(source));
        value["evidence_kind"] = cell(json!("retained_contact_field"));
        confirmations[source] = value;
    }
    let mut uploads = json!({});
    for source in ["lotw", "eqsl", "qrz", "clublog"] {
        let mut value = empty_group(
            "source state receipt_revision last_attempt_at_unix_ms is_current_revision",
        );
        value["source"] = cell(json!(source));
        for key in [
            "state",
            "receipt_revision",
            "last_attempt_at_unix_ms",
            "is_current_revision",
        ] {
            value[key] = unavailable("receipt_not_projected");
        }
        uploads[source] = value;
    }
    json!({
        "schema_version":1,"contact_id":"abcdef12-3456-4abc-8def-123456abcdef","status":"found","reason":null,
        "logbook_revision":9,"contact_revision":2,
        "fields":{
            "contact":contact,
            "radio":empty_group("frequency_hz receive_frequency_hz receive_band tx_power_raw tx_power_w propagation_mode antenna_path satellite_name"),
            "station":empty_group("name qth state county country dxcc continent cq_zone itu_zone"),
            "own_station":empty_group("operator_call callsign grid recorded_rig radio_label station_profile_id station_location"),
            "references":{"their":empty_group("pota sota wwff iota sig sig_info"),"my":empty_group("pota sota wwff iota sig sig_info"),"dok":cell(Value::Null)},
            "confirmations":confirmations,"awards":empty_group("credit_granted credit_submitted authority"),"uploads":uploads,
            "contest":empty_group("contest_id stx srx stx_string srx_string")
        },
        "editability":{"allowed":true,"reason":"complete_core","allowed_fields":["started_at_unix_ms","frequency_hz","rst_sent","rst_received","grid","comment"],"base":{
            "contact_id":"ABCDEF12-3456-4ABC-8DEF-123456ABCDEF","callsign":"N0TEST","band":"20m","mode":"MFSK",
            "started_at_unix_ms":1_700_000_000_000_u64,"frequency_hz":null,"rst_sent":"-05","rst_received":"-08","grid":"","comment":"first\nsecond"
        }},"locate":unavailable("paged_logbook_not_supported")
    })
}
fn read_only(v: &mut Value) {
    v["editability"] =
        json!({"allowed":false,"reason":"core_not_lossless","allowed_fields":[],"base":null});
}
fn decode(v: &Value) -> Result<ContactDetail, Error> {
    ContactDetail::decode(&serde_json::to_vec(v).unwrap())
}

#[test]
fn exact_nullable_roundtrip_preserves_uuid_case_and_raw_provenance() {
    let mut v = fixture();
    v["fields"]["confirmations"]["paper"]["received_status"] = cell(json!("Y? raw\nstatus"));
    v["fields"]["references"]["their"]["pota"] = cell(json!("X-0001, X-0002"));
    let detail = decode(&v).unwrap();
    assert_eq!(serde_json::to_value(&detail).unwrap(), v);
    let base = detail.editable_base().unwrap();
    assert_eq!(base.contact_id, "ABCDEF12-3456-4ABC-8DEF-123456ABCDEF");
    assert_eq!(base.frequency_hz, None);
}
#[test]
fn nullable_keys_cannot_be_omitted_at_any_level() {
    for key in ["reason", "contact_revision", "fields"] {
        let mut v = fixture();
        v.as_object_mut().unwrap().remove(key);
        assert!(decode(&v).is_err(), "{key}");
    }
    for key in ["state", "value", "reason"] {
        let mut v = fixture();
        v["locate"].as_object_mut().unwrap().remove(key);
        assert!(decode(&v).is_err(), "{key}");
    }
    let mut v = fixture();
    v["editability"].as_object_mut().unwrap().remove("base");
    assert!(decode(&v).is_err());
    let keys: Vec<_> = fixture()["editability"]["base"]
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert_eq!(keys.len(), 10);
    for key in keys {
        let mut v = fixture();
        v["editability"]["base"]
            .as_object_mut()
            .unwrap()
            .remove(&key);
        assert!(decode(&v).is_err(), "{key}");
    }
}
#[test]
fn outer_uuid_is_canonical_while_equivalent_foundation_base_case_is_preserved() {
    let v = fixture();
    assert!(decode(&v).is_ok());
    let mut upper = v.clone();
    upper["contact_id"] = json!("ABCDEF12-3456-4ABC-8DEF-123456ABCDEF");
    assert!(decode(&upper).is_err());
    read_only(&mut upper);
    assert!(decode(&upper).is_err());
    let mut lower_base = v;
    lower_base["editability"]["base"]["contact_id"] = lower_base["contact_id"].clone();
    assert!(decode(&lower_base).is_ok());
}
#[test]
fn raw_duplicate_keys_and_non_integer_revisions_are_rejected() {
    let text = serde_json::to_string(&fixture()).unwrap();
    let duplicate = text.replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(ContactDetail::decode(duplicate.as_bytes()).is_err());
    let duplicate = text.replacen(
        "\"frequency_hz\":null",
        "\"frequency_hz\":null,\"frequency_hz\":null",
        1,
    );
    assert!(ContactDetail::decode(duplicate.as_bytes()).is_err());
    let fractional = text.replacen("\"logbook_revision\":9", "\"logbook_revision\":9.5", 1);
    assert!(ContactDetail::decode(fractional.as_bytes()).is_err());
}
#[test]
fn fixed_groups_schema_types_and_unknown_fields_are_strict() {
    for key in [
        "contact",
        "radio",
        "station",
        "own_station",
        "references",
        "confirmations",
        "awards",
        "uploads",
        "contest",
    ] {
        let mut v = fixture();
        v["fields"].as_object_mut().unwrap().remove(key);
        assert!(decode(&v).is_err(), "{key}");
        let mut v = fixture();
        v["fields"][key]["invented"] = json!(true);
        assert!(decode(&v).is_err(), "{key}");
    }
    for (path, bad) in [
        ("/schema_version", json!(2)),
        ("/status", json!("missing")),
        ("/contact_id", json!("not-uuid")),
        ("/fields/radio/tx_power_w/value", json!("12")),
    ] {
        let mut v = fixture();
        *v.pointer_mut(path).unwrap() = bad;
        assert!(decode(&v).is_err(), "{path}");
    }
    let mut v = fixture();
    v["unknown"] = json!(1);
    assert!(decode(&v).is_err());
    let mut v = fixture();
    v["editability"]["base"]["submode"] = json!("FT8");
    assert!(decode(&v).is_err());
}
#[test]
fn state_value_empty_and_unavailable_are_not_interchangeable() {
    for bad in [
        json!({"state":"value","value":null,"reason":null}),
        json!({"state":"value","value":"","reason":null}),
        json!({"state":"value","value":"x","reason":"why"}),
        json!({"state":"empty","value":"x","reason":null}),
        json!({"state":"empty","value":null,"reason":"why"}),
        json!({"state":"unavailable","value":null,"reason":null}),
        json!({"state":"unavailable","value":null,"reason":""}),
        json!({"state":"unavailable","value":false,"reason":"why"}),
        json!({"state":"unavailable","value":null,"reason":"bad\u{0}reason"}),
    ] {
        let mut v = fixture();
        v["fields"]["station"]["name"] = bad;
        assert!(decode(&v).is_err());
    }
}
#[test]
fn raw_read_only_controls_and_invalid_retained_core_remain_displayable() {
    let mut v = fixture();
    read_only(&mut v);
    v["fields"]["contact"]["callsign"] = cell(json!("<raw>\u{0}\n"));
    v["fields"]["contact"]["mode"] = cell(Value::Null);
    v["fields"]["contact"]["started_at_unix_ms"] = cell(json!(-1));
    v["fields"]["radio"]["frequency_hz"] = unavailable("invalid");
    v["fields"]["contact"]["notes"] = cell(json!("\tline\r\n<script>inert</script>\u{0}"));
    let detail = decode(&v).unwrap();
    assert!(detail.editable_base().is_none());
    assert_eq!(serde_json::to_value(detail).unwrap(), v);
}
#[test]
fn utf8_byte_caps_apply_to_text_comment_notes_and_reasons() {
    for (path, max) in [
        ("/fields/station/name", 4096),
        ("/fields/contact/comment", 4096),
        ("/fields/contact/notes", 16384),
    ] {
        let mut v = fixture();
        read_only(&mut v);
        *v.pointer_mut(path).unwrap() = cell(json!("é".repeat(max / 2)));
        assert!(decode(&v).is_ok(), "{path}");
        *v.pointer_mut(path).unwrap() = cell(json!("é".repeat(max / 2) + "x"));
        assert!(decode(&v).is_err(), "{path}");
    }
    let mut v = fixture();
    v["locate"]["reason"] = json!("x".repeat(4097));
    assert!(decode(&v).is_err());
}
#[test]
fn signed_json_safe_facts_and_nonnegative_revision_bounds() {
    for n in [-SAFE, SAFE] {
        let mut v = fixture();
        v["fields"]["station"]["dxcc"] = cell(json!(n));
        assert!(decode(&v).is_ok());
    }
    for n in [-SAFE - 1, SAFE + 1] {
        let mut v = fixture();
        v["fields"]["station"]["dxcc"] = cell(json!(n));
        assert!(decode(&v).is_err());
    }
    for path in ["/logbook_revision", "/contact_revision"] {
        for n in [-1, SAFE + 1] {
            let mut v = fixture();
            *v.pointer_mut(path).unwrap() = json!(n);
            assert!(decode(&v).is_err());
        }
    }
    let mut v = fixture();
    v["fields"]["radio"]["tx_power_w"] = cell(json!(12.25));
    assert!(decode(&v).is_ok());
    v["fields"]["radio"]["tx_power_w"] = cell(json!(SAFE as f64 + 2.0));
    assert!(decode(&v).is_err());
    let mut d = decode(&fixture()).unwrap();
    d.data.fields.as_mut().unwrap().radio.tx_power_w = Field {
        state: FieldState::Value,
        value: Some(f64::NAN),
        reason: None,
    };
    assert!(serde_json::to_value(d).is_err());
}
#[test]
fn absent_statuses_preserve_explicit_optional_revisions_without_inventing_facts() {
    for status in ["deleted", "not_found", "unavailable"] {
        for revision in [Value::Null, json!(0), json!(SAFE)] {
            let mut v = fixture();
            read_only(&mut v);
            v["status"] = json!(status);
            v["fields"] = Value::Null;
            v["contact_revision"] = revision;
            let detail = decode(&v).unwrap();
            assert_eq!(serde_json::to_value(detail).unwrap(), v);
            v["fields"] = fixture()["fields"].clone();
            assert!(decode(&v).is_err());
        }
    }
    let mut v = fixture();
    v["contact_revision"] = json!(0);
    assert!(decode(&v).is_err());
    read_only(&mut v);
    assert!(decode(&v).is_ok());
    v["contact_revision"] = Value::Null;
    assert!(decode(&v).is_err());
}
#[test]
fn editing_requires_exact_six_fields_positive_revision_and_valid_qso() {
    for fields in [
        json!([]),
        json!([
            "started_at_unix_ms",
            "frequency_hz",
            "rst_sent",
            "rst_received",
            "grid",
            "grid"
        ]),
        json!([
            "started_at_unix_ms",
            "frequency_hz",
            "rst_sent",
            "rst_received",
            "grid",
            "submode"
        ]),
    ] {
        let mut v = fixture();
        v["editability"]["allowed_fields"] = fields;
        assert!(decode(&v).is_err());
    }
    let mut v = fixture();
    v["editability"]["base"]["callsign"] = json!("BAD#");
    v["fields"]["contact"]["callsign"] = cell(json!("BAD#"));
    assert!(decode(&v).is_err());
    let mut v = fixture();
    v["editability"]["reason"] = json!("invented_permission");
    assert!(decode(&v).is_err());
    let mut v = fixture();
    v["editability"]["allowed"] = json!(false);
    assert!(decode(&v).is_err());
}
#[test]
fn editing_requires_exact_source_values_and_semantic_uuid_equality() {
    for (key, bad) in [
        ("contact_id", json!("00000000-0000-0000-0000-000000000001")),
        ("callsign", json!("N1TEST")),
        ("band", json!("40m")),
        ("mode", json!("FT8")),
        ("started_at_unix_ms", json!(1)),
        ("frequency_hz", json!(14_074_000)),
        ("rst_sent", json!("-09")),
        ("rst_received", json!("-10")),
        ("grid", json!("FN42")),
        ("comment", json!("changed")),
    ] {
        let mut v = fixture();
        v["editability"]["base"][key] = bad;
        assert!(decode(&v).is_err(), "{key}");
    }
    let mut v = fixture();
    v["fields"]["radio"]["frequency_hz"] = unavailable("invalid");
    assert!(decode(&v).is_err());
}
#[test]
fn editing_requires_owner_call_band_mode_and_lossless_maidenhead_form() {
    for (key, bad) in [
        ("callsign", "ab1"),
        ("callsign", "A1"),
        ("band", "20M"),
        ("band", "20-m"),
        ("band", "20 m"),
        ("mode", "ft8"),
        ("mode", "FT.8"),
        ("grid", "FN4"),
        ("grid", "fn42"),
        ("grid", "SN42"),
        ("grid", "FN42YY"),
        ("grid", "FN42ABx1"),
    ] {
        let mut v = fixture();
        v["editability"]["base"][key] = json!(bad);
        v["fields"]["contact"][key] = cell(json!(bad));
        assert!(decode(&v).is_err(), "{key} {bad}");
        read_only(&mut v);
        assert!(decode(&v).is_ok(), "read-only {key} {bad}");
    }
    for grid in ["", "AA00", "RR99XX", "FN42AB12"] {
        let mut v = fixture();
        v["editability"]["base"]["grid"] = json!(grid);
        v["fields"]["contact"]["grid"] = if grid.is_empty() {
            cell(Value::Null)
        } else {
            cell(json!(grid))
        };
        assert!(decode(&v).is_ok(), "valid {grid}");
    }
}
#[test]
fn total_wire_bound_and_serialization_revalidation_cannot_be_bypassed() {
    let bytes = serde_json::to_vec(&fixture()).unwrap();
    let mut oversized = bytes.clone();
    oversized.resize(MAX_DETAIL_BYTES + 1, b' ');
    assert_eq!(ContactDetail::decode(&oversized), Err(Error::TooLarge));
    fn enlarge(v: &mut Value) {
        if v.get("state").is_some() {
            *v = unavailable(&"x".repeat(4096));
        } else if let Value::Object(o) = v {
            for child in o.values_mut() {
                enlarge(child)
            }
        }
    }
    let mut v = fixture();
    read_only(&mut v);
    enlarge(&mut v["fields"]);
    assert!(serde_json::from_value::<ContactDetail>(v).is_err());
    let mut d = decode(&fixture()).unwrap();
    d.data.logbook_revision = -1;
    assert!(serde_json::to_value(d).is_err());
}
#[test]
fn replay_actual_swift_contact_detail() {
    let bytes = include_bytes!("../../fixtures/contact-detail-swift.json");
    let corpus: Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(corpus.as_object().unwrap().len(), 3);
    for (name, status) in [
        ("found", Status::Found),
        ("deleted", Status::Deleted),
        ("not_found", Status::NotFound),
    ] {
        let input = corpus.get(name).expect("all producer cases required");
        let detail = decode(input).unwrap();
        assert_eq!(detail.data().status, status);
        assert_eq!(serde_json::to_value(&detail).unwrap(), *input);
        if status == Status::Found {
            let base = detail.editable_base().unwrap();
            assert!(base.contact_id.bytes().any(|b| b.is_ascii_uppercase()));
            assert!(base
                .contact_id
                .eq_ignore_ascii_case(&detail.data().contact_id));
            assert_eq!(base.frequency_hz, None);
        }
    }
}
