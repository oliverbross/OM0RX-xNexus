#![allow(dead_code)]
include!(concat!(env!("OUT_DIR"), "/owner_transport.rs"));
#[cfg(test)]
mod swift_owner;
#[cfg(test)]
mod paired {
    use super::companion_bridge::Bridge;
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    };
    use xnexus_xlog_client::{BridgeClient, Command, CommandStatus};
    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
    #[test]
    fn production_client_and_production_transport_pair_control_stop_revoke() {
        let effects = Arc::new(AtomicU64::new(0));
        let observed = effects.clone();
        let bridge=Bridge::start("workspace-fixture".into(), Arc::new(move |command| {
            if let Some(command)=command {
                if command.get("receipt_lookup").is_some(){return Ok(serde_json::Value::Null);}
                observed.fetch_add(1,Ordering::SeqCst);
                return Ok(json!({"status":"observed","message":"synthetic owner only"}));
            }
            Ok(json!({"revision":1,"generated_at_unix_ms":now(),"max_age_ms":3000,
                "station":{"connected":true,"radio_id":"radio-fixture","frequency_hz":14074000,"mode":"FT8","transmitting":false},
                "rotor":{"connected":true,"rotor_id":"rotor-fixture","azimuth_deg":45.0},"spots":[]}))
        })).unwrap();
        let mut wrong =
            BridgeClient::new(&bridge.endpoint, "wrong-valid-length-token-fixture").unwrap();
        assert!(wrong.connect().is_err());
        let mut client = BridgeClient::new(&bridge.endpoint, &bridge.token).unwrap();
        assert_eq!(client.connect().unwrap().workspace_id, "workspace-fixture");
        client.poll().unwrap();
        let stop = client.stop_handle().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let receipt = client
            .command(
                &id,
                Command::StationTune {
                    radio_id: "radio-fixture".into(),
                    frequency_hz: 14075000,
                    mode: "FT8".into(),
                },
            )
            .unwrap();
        assert_eq!(receipt.status, CommandStatus::Observed);
        assert_eq!(client.receipt(&id).unwrap().unwrap(), receipt);
        assert_eq!(effects.load(Ordering::SeqCst), 1);
        // Stop remains a separate transport after the ordinary client's observation is invalidated.
        stop.stop_rotor(&uuid::Uuid::new_v4().to_string(), "rotor-fixture")
            .unwrap();
        assert_eq!(effects.load(Ordering::SeqCst), 2);
        bridge.revoke();
        assert!(client.poll().is_err());
        assert!(stop
            .stop_rotor(&uuid::Uuid::new_v4().to_string(), "rotor-fixture")
            .is_err());
        assert_eq!(effects.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn uncertain_qso_survives_reopen_and_requires_explicit_identical_retry() {
        use xnexus_xlog_client::outbox::{
            DeliveryError, LogbookMutation, Outbox, OutboxError, OutboxState, QsoDraft,
        };
        let attempts = Arc::new(AtomicU64::new(0));
        let observed = attempts.clone();
        let contact_id = uuid::Uuid::new_v4().to_string();
        let owner_contact = contact_id.clone();
        let bridge = Bridge::start(
            "queue-workspace".into(),
            Arc::new(move |request| {
                if let Some(request) = request {
                    if request.get("receipt_lookup").is_some() {
                        return Ok(serde_json::Value::Null);
                    }
                    if observed.fetch_add(1, Ordering::SeqCst) == 0 {
                        return Err("Synthetic interruption before owner commit".into());
                    }
                    return Ok(
                        json!({"status":"applied","contact_id":owner_contact,"contact_revision":1}),
                    );
                }
                Ok(
                    json!({"revision":1,"generated_at_unix_ms":now(),"max_age_ms":3000,
                "station":{"connected":false},"rotor":{"connected":false},"spots":[]}),
                )
            }),
        )
        .unwrap();
        let root = std::env::temp_dir().join(format!("xnexus-paired-{}", uuid::Uuid::new_v4()));
        let request_id = uuid::Uuid::new_v4().to_string();
        let mut queue = Outbox::open(&root).unwrap();
        let qso = QsoDraft {
            contact_id,
            callsign: "OM0RX/P".into(),
            band: "20m".into(),
            mode: "FT8".into(),
            started_at_unix_ms: now(),
            frequency_hz: Some(14074000),
            rst_sent: "-10".into(),
            rst_received: "-12".into(),
            grid: "JN98".into(),
            comment: "Synthetic acceptance\nNo real QSO".into(),
        };
        queue
            .enqueue(
                "queue-workspace",
                &request_id,
                LogbookMutation::Create { qso },
            )
            .unwrap();
        let mut client = BridgeClient::new(&bridge.endpoint, &bridge.token).unwrap();
        client.connect().unwrap();
        client.poll().unwrap();
        assert_eq!(
            queue.submit(&mut client, &request_id).unwrap().status,
            CommandStatus::Uncertain
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        drop(queue);
        let mut queue = Outbox::open(&root).unwrap();
        assert_eq!(queue.entries()[0].state, OutboxState::NeedsReceipt);
        // A second Submit only looks up the receipt; it cannot repeat the effect.
        assert_eq!(
            queue.submit(&mut client, &request_id),
            Err(DeliveryError::Queue(OutboxError::ReceiptRequired))
        );
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        client.poll().unwrap();
        let receipt = queue.retry_missing(&mut client, &request_id).unwrap();
        assert_eq!(receipt.status, CommandStatus::Applied);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(queue.submit(&mut client, &request_id).unwrap(), receipt);
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
        assert_eq!(
            queue.entry(&request_id, "another-workspace"),
            Err(OutboxError::WorkspaceChanged)
        );
        drop(queue);
        let recovered = Outbox::open(&root).unwrap();
        assert!(matches!(
            recovered.entries()[0].state,
            OutboxState::Settled(_)
        ));
        bridge.revoke();
        // Evidence is deliberately retained. It contains synthetic contacts only.
        eprintln!("Synthetic outbox evidence: {}", root.display());
    }

    #[test]
    fn portable_callsign_history_crosses_both_implementations() {
        let bridge = Bridge::start(
            "history-workspace".into(),
            Arc::new(move |request| {
                let Some(request) = request else {
                    return Ok(
                        json!({"revision":1,"generated_at_unix_ms":now(),"max_age_ms":3000,
                        "station":{"connected":false},"rotor":{"connected":false},"spots":[]}),
                    );
                };
                assert_eq!(request["history_lookup"], "OM0RX/P");
                Ok(
                    json!({"callsign":"OM0RX/P","generated_at_unix_ms":now(),"contacts":[],
                "matrix":[{"band":"20m","mode":"FT8","worked":true,"confirmed":false}]}),
                )
            }),
        )
        .unwrap();
        let mut client = BridgeClient::new(&bridge.endpoint, &bridge.token).unwrap();
        client.connect().unwrap();
        let history = client.history("om0rx/p").unwrap();
        assert_eq!(history.workspace_id, "history-workspace");
        assert!(history.matrix[0].worked);
        assert!(!history.matrix[0].confirmed);
        bridge.revoke();
    }
}
