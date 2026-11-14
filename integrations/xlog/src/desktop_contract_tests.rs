// SPDX-License-Identifier: GPL-3.0-only
//! The UI renders this exact wire JSON. Exact serde round-trips reject silently
//! ignored old/misspelled fields as well as missing nullable fields.
use super::*;
use serde::de::DeserializeOwned;

fn round_trip<T: DeserializeOwned + Serialize>(value: &serde_json::Value) {
    let typed: T = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), *value);
}

#[test]
fn desktop_fixture_matches_native_wire_contract_exactly() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../ui/src/xlog/fixtures/bridge-contract.json"
    ))
    .unwrap();
    round_trip::<Hello>(&fixture["hello"]);
    round_trip::<Snapshot>(&fixture["snapshot"]);
    round_trip::<History>(&fixture["history"]);
    round_trip::<CommandReceipt>(&fixture["receipt"]);
    round_trip::<outbox::OutboxEntry>(&fixture["outbox"]);
    round_trip::<outbox::LogbookMutation>(&fixture["outbox"]["queued"]["mutation"]);
}
