// SPDX-License-Identifier: GPL-3.0-only
//! Opt-in actual SQLite-owner acceptance. No station process or device owners.
use crate::companion_bridge::Bridge;
use serde_json::{json, Value};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command as ProcessCommand, Stdio},
    sync::{mpsc, Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use xnexus_xlog_client::{
    outbox::{LogbookMutation, Outbox, QsoDraft},
    BridgeClient, Command, CommandStatus,
};

struct Owner {
    child: Child,
    input: Option<ChildStdin>,
    replies: mpsc::Receiver<String>,
}
impl Owner {
    fn start(executable: &Path, root: &Path) -> Self {
        let mut command = ProcessCommand::new(executable);
        command
            .arg(format!("--fixture-root={}", root.display()))
            .env("TEMP", root.parent().unwrap())
            .env("TMP", root.parent().unwrap())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(root.join("owner-stderr.log"))
                    .unwrap(),
            );
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x0800_0000);
        }
        let mut child = command
            .spawn()
            .expect("Launch explicit disposable owner fixture");
        let input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        let (sender, replies) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                match line {
                    Ok(line) if line.len() <= 262_144 => {
                        if sender.send(line).is_err() {
                            break;
                        }
                    }
                    _ => break,
                }
            }
        });
        Self {
            child,
            input,
            replies,
        }
    }
    fn request(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = uuid::Uuid::new_v4().to_string();
        writeln!(
            self.input.as_mut().ok_or("Owner input closed")?,
            "{}",
            json!({"id":id,"method":method,"params":params})
        )
        .map_err(|e| e.to_string())?;
        self.input
            .as_mut()
            .unwrap()
            .flush()
            .map_err(|e| e.to_string())?;
        let line = self
            .replies
            .recv_timeout(Duration::from_secs(20))
            .map_err(|e| e.to_string())?;
        let response: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if response["id"] != id || response["ok"] != true {
            return Err(response.to_string());
        }
        Ok(response["result"].clone())
    }
    fn close(&mut self) -> Result<(), String> {
        self.input.take();
        let end = Instant::now() + Duration::from_secs(20);
        while Instant::now() < end {
            if let Some(status) = self.child.try_wait().map_err(|e| e.to_string())? {
                return if status.success() {
                    Ok(())
                } else {
                    Err(format!("Disposable owner exited: {status}"))
                };
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        Err("Disposable owner did not exit after stdin closed".into())
    }
}
impl Drop for Owner {
    fn drop(&mut self) {
        if self.close().is_ok() {
            return;
        }
        // Only this explicitly launched, no-GUI disposable fixture is terminated.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
fn receipt(value: &Value) -> Value {
    json!({"status":"applied","contact_id":value["contact"]["qso"]["id"],"contact_revision":value["contact"]["revision"]})
}
struct RunningBridge(Option<Arc<Bridge>>);
impl std::ops::Deref for RunningBridge {
    type Target = Bridge;
    fn deref(&self) -> &Bridge {
        self.0.as_ref().unwrap()
    }
}
impl Drop for RunningBridge {
    fn drop(&mut self) {
        if let Some(bridge) = self.0.take() {
            bridge.revoke();
            let weak = Arc::downgrade(&bridge);
            drop(bridge);
            let end = Instant::now() + Duration::from_secs(3);
            while weak.strong_count() != 0 && Instant::now() < end {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}
fn start_bridge(owner: Arc<Mutex<Owner>>, workspace: &str) -> RunningBridge {
    let sequence = std::sync::atomic::AtomicU64::new(0);
    RunningBridge(Some(Bridge::start(workspace.into(), Arc::new(move |request| {
        let Some(request) = request else {
                let revision = sequence.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                return Ok(json!({"revision":revision,"generated_at_unix_ms":now(),"max_age_ms":3000,
                "station":{"connected":false},"rotor":{"connected":false},"spots":[]}));
        };
        let mut owner = owner.lock().map_err(|_| "Fixture owner lock failed")?;
        if let Some(call) = request["history_lookup"].as_str() {
            return owner.request("logbook_companion_history", json!({"callsign":call,"generation":request["workspace_id"]}));
        }
        if let Some(id) = request["receipt_lookup"].as_str() {
            let result = owner.request("logbook_companion_receipt", json!({"operationID":id,"generation":request["workspace_id"]}))?;
            return Ok(if result["receipt"].is_null() { Value::Null } else { receipt(&result["receipt"]) });
        }
        let command = &request["command"];
        if !matches!(command["kind"].as_str(),Some("logbook.create"|"logbook.update")) { return Err("Fixture forbids hardware commands".into()); }
        let mut params = json!({"operationID":request["request_id"],"generation":request["workspace_id"],"qso":command["qso"]});
        if command["kind"] == "logbook.update" { params["expectedRevision"] = command["expected_contact_revision"].clone(); }
        let result = owner.request("logbook_companion_apply", params)?;
        Ok(if result["status"] == "applied" { receipt(&result["receipt"]) } else { json!({"status":"rejected","message":"Owner refused stale or conflicting contact"}) })
    })).unwrap()))
}
fn connect(bridge: &Bridge) -> BridgeClient {
    let mut client = BridgeClient::new(&bridge.endpoint, &bridge.token).unwrap();
    client.connect().unwrap();
    client.poll().unwrap();
    client
}

#[test]
#[ignore = "Requires explicitly built xLog companion-owner-fixture and disposable storage"]
fn actual_swift_owner_restart_reconciles_contact_and_rejects_stale_edit() {
    let executable =
        PathBuf::from(std::env::var_os("XNEXUS_OWNER_FIXTURE").expect("Set XNEXUS_OWNER_FIXTURE"));
    assert!(executable.is_absolute() && executable.is_file());
    let evidence = PathBuf::from(
        std::env::var_os("XNEXUS_OWNER_EVIDENCE")
            .expect("Set XNEXUS_OWNER_EVIDENCE to an absolute disposable evidence directory"),
    );
    assert!(evidence.is_absolute());
    std::fs::create_dir_all(&evidence).unwrap();
    let root = evidence.join(format!("xlog-companion-owner-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join(".companion-owner-fixture"),
        uuid::Uuid::new_v4().to_string(),
    )
    .unwrap();
    let mut fixture = Owner::start(&executable, &root);
    fixture
        .request("workspace_create", json!({"mode":"empty"}))
        .unwrap();
    let workspace = fixture.request("workspace_status", json!({})).unwrap()["generation"]
        .as_str()
        .unwrap()
        .to_owned();
    let owner = Arc::new(Mutex::new(fixture));
    let bridge = start_bridge(owner.clone(), &workspace);
    let mut client = connect(&bridge);
    let qso = QsoDraft {
        contact_id: uuid::Uuid::new_v4().to_string(),
        callsign: "W1AW".into(),
        band: "20m".into(),
        mode: "CW".into(),
        started_at_unix_ms: 1_700_000_000_123,
        frequency_hz: Some(14025000),
        rst_sent: "599".into(),
        rst_received: "599".into(),
        grid: "FN31".into(),
        comment: "Actual owner\nDisposable contact".into(),
    };
    let operation = uuid::Uuid::new_v4().to_string();
    let queue_root = root.join("xnexus-outbox");
    let mut queue = Outbox::open(&queue_root).unwrap();
    queue
        .enqueue(
            &workspace,
            &operation,
            LogbookMutation::Create { qso: qso.clone() },
        )
        .unwrap();
    queue.mark_attempted(&operation, &workspace).unwrap();
    let first = client
        .command(&operation, Command::LogbookCreate { qso: qso.clone() })
        .unwrap();
    assert_eq!(first.status, CommandStatus::Applied);
    assert_eq!(first.contact_revision, Some(1));
    // Model loss after owner commit and before local settlement, then restart BOTH sides.
    drop(queue);
    drop(client);
    bridge.revoke();
    drop(bridge);
    let mut fixture = Arc::try_unwrap(owner)
        .ok()
        .expect("No retained provider after bridge stopped")
        .into_inner()
        .unwrap();
    fixture.close().unwrap();
    drop(fixture);
    let mut fixture = Owner::start(&executable, &root);
    assert_eq!(
        fixture.request("workspace_status", json!({})).unwrap()["generation"],
        workspace
    );
    let owner = Arc::new(Mutex::new(fixture));
    let bridge = start_bridge(owner.clone(), &workspace);
    let mut client = connect(&bridge);
    let mut queue = Outbox::open(&queue_root).unwrap();
    assert_eq!(
        queue.submit(&mut client, &operation).unwrap().status,
        CommandStatus::Applied
    );
    let history = client.history("W1AW").unwrap();
    assert_eq!(history.contacts.len(), 1);
    assert_eq!(
        history.contacts[0].started_at_unix_ms,
        qso.started_at_unix_ms
    );
    let mut corrected = qso.clone();
    corrected.comment = "Correction\nPreserved millisecond time".into();
    let correction = uuid::Uuid::new_v4().to_string();
    queue
        .enqueue(
            &workspace,
            &correction,
            LogbookMutation::Update {
                qso: corrected.clone(),
                expected_contact_revision: 1,
            },
        )
        .unwrap();
    client.poll().unwrap();
    assert_eq!(
        queue
            .submit(&mut client, &correction)
            .unwrap()
            .contact_revision,
        Some(2)
    );
    let conflict = uuid::Uuid::new_v4().to_string();
    queue
        .enqueue(
            &workspace,
            &conflict,
            LogbookMutation::Update {
                qso: qso.clone(),
                expected_contact_revision: 1,
            },
        )
        .unwrap();
    client.poll().unwrap();
    assert_eq!(
        queue.submit(&mut client, &conflict).unwrap().status,
        CommandStatus::Rejected
    );
    client.poll().unwrap();
    assert_eq!(
        client
            .command(&operation, Command::LogbookCreate { qso })
            .unwrap()
            .contact_revision,
        Some(1)
    );
    let history = client.history("W1AW").unwrap();
    assert_eq!(history.contacts.len(), 1);
    assert_eq!(history.contacts[0].comment, corrected.comment);
    assert_eq!(history.contacts[0].contact_revision, Some(2));
    bridge.revoke();
    std::fs::write(root.join("acceptance.json"),json!({"passed":true,"scope":"actual Swift owner and production HTTP/client; synthetic station snapshot","workspace":workspace,"contact_id":corrected.contact_id,"create_operation":operation,"correction_operation":correction,"conflict_operation":conflict}).to_string()).unwrap();
    eprintln!("Actual-owner acceptance evidence: {}", root.display());
    drop(bridge);
    owner.lock().unwrap().close().unwrap();
    drop(owner);
}
