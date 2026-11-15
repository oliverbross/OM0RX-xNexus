use super::*;
use serde_json::{json, Value};

fn corpus() -> Value {
    serde_json::from_slice(include_bytes!("../../fixtures/bearing-swift-v1.json")).unwrap()
}
fn fixture(name: &str) -> Value {
    corpus()[name].clone()
}
fn decode(value: &Value) -> Result<Bearing, Error> {
    Bearing::decode(
        &serde_json::to_vec(value).unwrap(),
        value["requested_callsign"].as_str().unwrap_or(""),
    )
}

// Swift JSON may emit an integral Double as `1`; serde emits the same f64 as
// `1.0`. Normalize only schema-declared floating-point fields. All other values,
// including historical timestamps/revisions, keep their exact JSON types.
fn floating_fields(mut value: Value) -> Value {
    for path in [
        "/origin/latitude",
        "/origin/longitude",
        "/target/latitude",
        "/target/longitude",
        "/geometry/short_path_deg",
        "/geometry/long_path_deg",
        "/geometry/distance_km",
    ] {
        if let Some(field) = value.pointer_mut(path) {
            let number = field.as_f64().expect("fixture coordinate/geometry number");
            assert!(number.is_finite());
            *field = json!(number);
        }
    }
    value
}

#[test]
fn actual_swift_corpus_preserves_all_four_producer_states() {
    let values = corpus();
    assert_eq!(values.as_object().unwrap().len(), 4);
    for (name, value) in values.as_object().unwrap() {
        let decoded = decode(value).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(
            floating_fields(serde_json::to_value(decoded.data()).unwrap()),
            floating_fields(value.clone()),
            "{name}"
        );
    }
    let original = fixture("historical_grid");
    let mut changed = original.clone();
    changed["target"]["latitude"] = json!(original["target"]["latitude"].as_f64().unwrap() + 0.5);
    assert_ne!(floating_fields(original), floating_fields(changed));
    assert_eq!(
        decode(&fixture("coincident")).unwrap().data().reason,
        Some(Reason::Coincident)
    );
    assert_eq!(
        decode(&fixture("historical_grid"))
            .unwrap()
            .data()
            .target
            .as_ref()
            .unwrap()
            .grid
            .as_deref(),
        Some("PH57AB")
    );
}

#[test]
fn every_key_is_required_including_explicit_nullable_fields() {
    for name in [
        "historical_grid",
        "cty_approximation",
        "reference_unavailable",
    ] {
        let source = fixture(name);
        for group in [None, Some("origin"), Some("target"), Some("geometry")] {
            let object = group.map_or(&source, |k| &source[k]);
            let Some(fields) = object.as_object() else {
                continue;
            };
            for key in fields.keys() {
                let mut missing = source.clone();
                let object = match group {
                    None => &mut missing,
                    Some(k) => &mut missing[k],
                };
                object.as_object_mut().unwrap().remove(key);
                assert!(
                    decode(&missing).is_err(),
                    "accepted {name}/{group:?}/{key} omission"
                );
            }
        }
    }
}

#[test]
fn unknown_and_duplicate_fields_never_silently_replace_facts() {
    for group in [None, Some("origin"), Some("target"), Some("geometry")] {
        let mut v = fixture("historical_grid");
        let object = match group {
            None => &mut v,
            Some(k) => &mut v[k],
        };
        object["unexpected"] = json!(true);
        assert!(decode(&v).is_err());
    }
    let raw = serde_json::to_string(&fixture("historical_grid")).unwrap();
    let duplicate = raw.replacen('{', "{\"motion_authority\":true,", 1);
    assert!(Bearing::decode(duplicate.as_bytes(), "VK8RX/P").is_err());
}

#[test]
fn exact_operating_call_is_not_trimmed_folded_to_base_or_unicode_normalized() {
    let mut v = fixture("historical_grid");
    assert!(Bearing::decode(&serde_json::to_vec(&v).unwrap(), "VK8RX").is_err());
    for call in ["vk8rx/p", "GB50", "9A2024", "EA8/OM0RX/P"] {
        v["requested_callsign"] = json!(call);
        v["normalized_callsign"] = json!(call.to_ascii_uppercase());
        assert_eq!(decode(&v).unwrap().data().requested_callsign, call);
    }
    for call in [
        " VK8RX", "VK8RX ", "VK8RX//P", "/VK8RX", "VK8RX/", "VК8RX", "ß1A", "AAA", "123", "A1",
        "A1\nA",
    ] {
        v["requested_callsign"] = json!(call);
        v["normalized_callsign"] = json!(call.to_ascii_uppercase());
        assert!(decode(&v).is_err(), "accepted {call:?}");
    }
    v["requested_callsign"] = json!("vk8rx/p");
    v["normalized_callsign"] = json!("VK8RX");
    assert!(decode(&v).is_err());
    v["normalized_callsign"] = json!("vk8rx/p");
    assert!(decode(&v).is_err());
}

#[test]
fn schema_and_authority_booleans_are_not_optional_or_truthy() {
    for (key, value) in [
        ("schema_version", json!(2)),
        ("motion_authority", json!(true)),
        ("transmit_authority", json!(true)),
        ("motion_authority", json!(null)),
        ("transmit_authority", json!(0)),
    ] {
        let mut v = fixture("historical_grid");
        v[key] = value;
        assert!(decode(&v).is_err());
    }
}

#[test]
fn source_precision_and_evidence_are_not_interchangeable() {
    for (name, field, value) in [
        (
            "historical_grid",
            "precision",
            json!("entity_approximation"),
        ),
        ("historical_grid", "reference_revision", json!("reference")),
        ("historical_grid", "matched_rule", json!("rule")),
        ("historical_grid", "country_status", json!("resolved")),
        ("historical_grid", "contact_id", json!("not-a-uuid")),
        ("historical_grid", "grid", Value::Null),
        (
            "cty_approximation",
            "precision",
            json!("historical_grid_cell_center"),
        ),
        (
            "cty_approximation",
            "contact_id",
            json!("abcdef12-3456-4abc-8def-123456abcdef"),
        ),
        ("cty_approximation", "contact_revision", json!(0)),
        ("cty_approximation", "contact_started_at_unix_ms", json!(0)),
        ("cty_approximation", "grid", json!("PH57AB")),
        ("cty_approximation", "country_status", json!("ambiguous")),
    ] {
        let mut v = fixture(name);
        v["target"][field] = value;
        assert!(decode(&v).is_err(), "{name}/{field}");
    }
    let mut v = fixture("historical_grid");
    for id in [
        "abcdef12-3456-4abc-8def-123456abcdef",
        "00000000-0000-0000-0000-000000000000",
    ] {
        v["target"]["contact_id"] = json!(id);
        assert!(decode(&v).is_ok());
    }
    v["target"]["contact_id"] = json!("ABCDEF12-3456-4ABC-8DEF-123456ABCDEF");
    assert!(decode(&v).is_err());
    let mut v = fixture("historical_grid");
    v["origin"]["source"] = json!("caller_coordinates");
    assert!(decode(&v).is_err());
}

#[test]
fn geometry_and_coordinates_are_bounded_without_computing_a_new_bearing() {
    for (group, field, number) in [
        ("origin", "latitude", 90.01),
        ("origin", "longitude", -180.01),
        ("target", "latitude", -90.01),
        ("target", "longitude", 180.01),
        ("geometry", "short_path_deg", 360.0),
        ("geometry", "long_path_deg", -0.01),
        ("geometry", "distance_km", -0.01),
    ] {
        let mut v = fixture("historical_grid");
        v[group][field] = json!(number);
        assert!(decode(&v).is_err());
    }
    let mut v = fixture("historical_grid");
    v["geometry"]["short_path_deg"] = json!(0.0);
    v["geometry"]["long_path_deg"] = json!(359.999);
    assert_eq!(
        decode(&v)
            .unwrap()
            .data()
            .geometry
            .as_ref()
            .unwrap()
            .long_path_deg,
        359.999
    );
    let mut raw: BearingData = serde_json::from_value(fixture("historical_grid")).unwrap();
    raw.geometry.as_mut().unwrap().short_path_deg = f64::NAN;
    assert!(raw.validate("VK8RX/P").is_err());
    raw = serde_json::from_value(fixture("historical_grid")).unwrap();
    raw.origin.as_mut().unwrap().latitude = f64::INFINITY;
    assert!(raw.validate("VK8RX/P").is_err());
}

#[test]
fn historical_signed_dates_and_revision_limits_are_preserved() {
    for stamp in [-SAFE, -1, 0, SAFE] {
        let mut v = fixture("historical_grid");
        v["target"]["contact_started_at_unix_ms"] = json!(stamp);
        assert_eq!(
            decode(&v)
                .unwrap()
                .data()
                .target
                .as_ref()
                .unwrap()
                .contact_started_at_unix_ms,
            Some(stamp)
        );
    }
    for (field, value) in [
        ("contact_started_at_unix_ms", SAFE + 1),
        ("contact_started_at_unix_ms", -SAFE - 1),
        ("contact_revision", -1),
        ("contact_revision", 8),
    ] {
        let mut v = fixture("historical_grid");
        v["target"][field] = json!(value);
        assert!(decode(&v).is_err());
    }
    for revision in [-1, SAFE + 1] {
        let mut v = fixture("historical_grid");
        v["logbook_revision"] = json!(revision);
        assert!(decode(&v).is_err());
    }
    let mut v = fixture("historical_grid");
    v["logbook_revision"] = json!(SAFE);
    v["target"]["contact_revision"] = json!(SAFE);
    assert!(decode(&v).is_ok());
}

#[test]
fn uppercase_grid_shape_is_not_silently_repaired_or_resolved() {
    for grid in [
        "JN",
        "jn88",
        "SN88",
        "JN8A",
        "JN88YY",
        "JN88AA1A",
        "JN88AA11AA",
    ] {
        let mut v = fixture("historical_grid");
        v["origin"]["grid"] = json!(grid);
        assert!(decode(&v).is_err());
        v = fixture("historical_grid");
        v["target"]["grid"] = json!(grid);
        assert!(decode(&v).is_err());
    }
}

#[test]
fn status_reason_geometry_and_target_presence_agree() {
    for (field, value) in [
        ("status", json!("unavailable")),
        ("reason", json!("coincident")),
        ("origin", Value::Null),
        ("target", Value::Null),
        ("geometry", Value::Null),
    ] {
        let mut v = fixture("historical_grid");
        v[field] = value;
        assert!(decode(&v).is_err());
    }
    for reason in ["coincident", "antipodal", "invalid_geometry"] {
        let mut v = fixture("coincident");
        v["reason"] = json!(reason);
        assert!(decode(&v).is_ok());
        v["target"] = Value::Null;
        assert!(decode(&v).is_err());
    }
    let mut v = fixture("reference_unavailable");
    v["origin"] = Value::Null;
    assert!(decode(&v).is_err());
    v["reason"] = json!("origin_unavailable");
    assert!(decode(&v).is_ok());
    v["reason"] = json!("made_up_reason");
    assert!(decode(&v).is_err());
}

#[test]
fn reference_facts_stay_raw_optional_and_byte_bounded() {
    let mut v = fixture("cty_approximation");
    v["target"]["matched_reference_revision"] = Value::Null;
    v["target"]["matched_rule"] = Value::Null;
    assert!(decode(&v).is_ok());
    v["target"]["matched_reference_revision"] = json!("independently retained raw revision");
    v["target"]["matched_rule"] = json!("é".repeat(64));
    assert_eq!(
        decode(&v)
            .unwrap()
            .data()
            .target
            .as_ref()
            .unwrap()
            .matched_rule
            .as_deref(),
        v["target"]["matched_rule"].as_str()
    );
    v["target"]["matched_rule"] = json!("é".repeat(65));
    assert!(decode(&v).is_err());
    v = fixture("cty_approximation");
    v["reference_revision"] = json!("other");
    assert!(decode(&v).is_err());
    v = fixture("cty_approximation");
    v["target"]["matched_reference_revision"] = json!("x".repeat(2049));
    assert!(decode(&v).is_err());
    v = fixture("cty_approximation");
    v["reference_revision"] = json!("x".repeat(2049));
    v["target"]["reference_revision"] = v["reference_revision"].clone();
    assert!(decode(&v).is_err());
}

#[test]
fn exact_body_size_limit_is_applied_before_deserialization() {
    let mut bytes = serde_json::to_vec(&fixture("historical_grid")).unwrap();
    bytes.resize(MAX_BEARING_BYTES, b' ');
    assert!(Bearing::decode(&bytes, "VK8RX/P").is_ok());
    bytes.push(b' ');
    assert!(matches!(
        Bearing::decode(&bytes, "VK8RX/P"),
        Err(Error::TooLarge)
    ));
}
