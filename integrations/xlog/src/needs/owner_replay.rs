// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! Explicit historical DTO replay, not a live bridge or installed-app test.
use super::*;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayQuery {
    query_id: String,
    callsign: String,
    dial_frequency_hz: u64,
    mode: DecoderMode,
}

#[test]
#[ignore = "requires hash-verified private synthetic Swift export via tests/replay-owner.ps1"]
fn actual_swift_export_contract_replay() {
    let root =
        PathBuf::from(std::env::var_os("XN_OWNER_REPLAY_ROOT").expect("explicit export root"));
    let expected: usize = std::env::var("XN_OWNER_REPLAY_CASES")
        .expect("explicit case count")
        .parse()
        .expect("numeric case count");
    assert!(expected > 0);
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(root.join("replies-manifest.json")).expect("required manifest"),
    )
    .unwrap();
    assert_eq!(manifest["nativeWireBytesUnmodified"], true);
    let files = manifest["files"].as_array().expect("manifest files");
    assert_eq!(files.len(), expected * 2);
    let mut names = HashSet::new();
    let mut cases = 0;
    let mut categories = BTreeMap::<String, usize>::new();
    for entry in files {
        let name = entry["file"].as_str().unwrap();
        assert_eq!(
            std::path::Path::new(name)
                .file_name()
                .and_then(|s| s.to_str()),
            Some(name)
        );
        assert!(names.insert(name));
        let bytes = std::fs::read(root.join("replies").join(name)).expect("required export file");
        assert_eq!(bytes.len() as u64, entry["bytes"].as_u64().unwrap());
        if !name.ends_with(".request.json") {
            continue;
        }
        let request: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(request["method"], "cluster_needs_query");
        let response_name = name.replace(".request.json", ".response.json");
        assert!(files.iter().any(|f| f["file"] == response_name));
        let response: Value = serde_json::from_slice(
            &std::fs::read(root.join("replies").join(&response_name)).unwrap(),
        )
        .unwrap();
        assert_eq!(request["id"], response["id"]);
        assert_eq!(response["ok"], true);
        assert_eq!(
            request["params"]["batch_id"],
            response["result"]["batch_id"]
        );
        let workspace = response["result"]["workspace_id"].as_str().unwrap();
        let queries: Vec<NeedsQuery> =
            serde_json::from_value::<Vec<ReplayQuery>>(request["params"]["queries"].clone())
                .unwrap()
                .into_iter()
                .map(|q| NeedsQuery {
                    query_id: q.query_id,
                    callsign: q.callsign,
                    dial_frequency_hz: q.dial_frequency_hz,
                    mode: q.mode,
                })
                .collect();
        assert!((1..=32).contains(&queries.len()));
        assert!(queries.iter().all(NeedsQuery::validate));
        assert_eq!(
            queries
                .iter()
                .map(|q| &q.query_id)
                .collect::<HashSet<_>>()
                .len(),
            queries.len()
        );
        // The Swift result is the exact public payload BEFORE Rust HTTP wrapping.
        // Add only explicitly synthetic transport identity, never rewrite facts/time.
        let public = response["result"]["result"].clone();
        let mut envelope = public.clone();
        let obj = envelope.as_object_mut().unwrap();
        assert!(
            !obj.contains_key("protocol_version")
                && !obj.contains_key("server_instance")
                && !obj.contains_key("workspace_id")
        );
        obj.insert("protocol_version".into(), json!(1));
        obj.insert(
            "server_instance".into(),
            json!("synthetic-swift-structural-replay"),
        );
        obj.insert("workspace_id".into(), json!(workspace));
        let wire: WireResponse = serde_json::from_value(envelope.clone()).unwrap();
        let anchor = wire.generated_at_unix_ms;
        let metadata = NeedsMetadata {
            generated_at_unix_ms: anchor,
            max_age_ms: wire.max_age_ms,
            readiness: wire.readiness,
            authority: wire.authority.clone(),
        };
        let mut bridge = BridgeClient::new("http://127.0.0.1:1", &crate::tests::token()).unwrap();
        bridge.hello = Some(Hello {
            application: "OM0RX-xLog".into(),
            protocol_version: 1,
            server_instance: "synthetic-swift-structural-replay".into(),
            workspace_id: workspace.into(),
            capabilities: vec!["logbook.classify".into()],
        });
        bridge.audio_session = Arc::new(AtomicBool::new(true));
        bridge
            .needs_authority
            .lock()
            .unwrap()
            .observe(Some(&metadata), anchor)
            .unwrap();
        let mut client = bridge.needs_handle().unwrap();
        let lease = client.authority.lock().unwrap().lease().unwrap();
        let batch = client
            .accept(wire, &queries, lease, anchor)
            .unwrap_or_else(|error| panic!("historical contract {name}: {error:?}"));
        assert_eq!(
            serde_json::to_value(batch.results().unwrap()).unwrap(),
            public["results"],
            "owner facts unchanged: {name}"
        );
        assert_eq!(batch.metadata().unwrap(), &metadata);
        *categories
            .entry(metadata.readiness.code().to_owned())
            .or_default() += 1;
        for query in &queries {
            *categories.entry(format!("{:?}", query.mode)).or_default() += 1;
        }
        let current = now_ms().unwrap();
        assert!(
            current.saturating_sub(anchor) >= metadata.max_age_ms,
            "export must be expired for current-clock negative control"
        );
        let lease = client.authority.lock().unwrap().lease().unwrap();
        assert!(matches!(
            client.accept(
                serde_json::from_value(envelope).unwrap(),
                &queries,
                lease,
                current
            ),
            Err(Error::Stale)
        ));
        cases += 1;
    }
    assert_eq!(cases, expected);
    assert!(categories.get("FT8").copied().unwrap_or(0) >= 6);
    assert!(categories.get("FT4").copied().unwrap_or(0) >= 6);
    assert!(categories.get("ready").copied().unwrap_or(0) >= 12);
    assert!(categories.get("stale").copied().unwrap_or(0) > 0);
    println!("Historical synthetic Swift contract replay: {cases} original envelopes; {categories:?}; all rejected at current wall clock. HTTP identity wrapping is synthetic; no installed/UI/hardware qualification.");
}
