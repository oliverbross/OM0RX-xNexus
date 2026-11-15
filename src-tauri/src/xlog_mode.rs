// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! The xLog-owned desktop. This path creates no Nexus station, device, logbook,
//! feed, updater or hosted Remote service. Only the explicit bridge commands below
//! exist in its invoke handler. Pairing secrets never cross into the webview.
//!
//! The connection generation prevents a late file-picker/network response from
//! restoring a connection after Disconnect/Forget. Polls and ordinary commands
//! are serialized by the native client; rotor Stop uses a separate transport.

use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::State;
#[path = "xlog_window.rs"]
mod private_window;
#[cfg(target_os = "macos")]
#[path = "xlog_profile_store.rs"]
mod profile_store;
use xnexus_xlog_client::outbox::{LogbookMutation, Outbox, OutboxEntry};
use xnexus_xlog_client::{
    BridgeClient, Command, CommandReceipt, Hello, History, Snapshot, StopClient,
};

#[tauri::command]
async fn xlog_history(state: State<'_, Shared>, callsign: String) -> Result<History, String> {
    let shared = Arc::clone(&state);
    let (generation, connection) = current(&shared)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut client = lock(&connection.client)?;
        if lock(&shared)?.generation != generation {
            return Err("Connection changed".into());
        }
        let history = client
            .history(&callsign)
            .map_err(|error| error.to_string())?;
        if lock(&shared)?.generation != generation {
            return Err("Connection changed".into());
        }
        Ok(history)
    })
    .await
    .map_err(|_| "xLog history request failed")?
}

type Queue = Arc<Mutex<Option<Outbox>>>;

/// A visible launch coordinate for isolated candidates; the same root holds the
/// companion queue and WebView profile. A malformed explicit root never falls back.
fn data_root_from(
    args: &[String],
    environment: Option<std::ffi::OsString>,
    fallback: std::path::PathBuf,
) -> Result<std::path::PathBuf, String> {
    let mut explicit = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--xlog-data-dir" {
            explicit = Some(std::path::PathBuf::from(
                args.next()
                    .filter(|value| !value.is_empty())
                    .ok_or("--xlog-data-dir requires an absolute path")?,
            ));
            break;
        }
        if let Some(value) = arg.strip_prefix("--xlog-data-dir=") {
            if value.is_empty() {
                return Err("--xlog-data-dir requires an absolute path".into());
            }
            explicit = Some(value.into());
            break;
        }
    }
    let root = explicit
        .or_else(|| {
            environment
                .filter(|value| !value.is_empty())
                .map(std::path::PathBuf::from)
        })
        .unwrap_or(fallback);
    if !root.is_absolute() {
        return Err("xLog companion data root must be absolute".into());
    }
    Ok(root)
}

fn data_root() -> Result<std::path::PathBuf, String> {
    data_root_from(
        &std::env::args().skip(1).collect::<Vec<_>>(),
        std::env::var_os("XNEXUS_DATA_DIR"),
        crate::config_dir().join("companion"),
    )
}

fn with_queue<T>(
    queue: &Queue,
    f: impl FnOnce(&mut Outbox) -> Result<T, String>,
) -> Result<T, String> {
    let mut guard = lock(queue)?;
    if guard.is_none() {
        let root = data_root()?.join("xlog-outbox");
        *guard = Some(Outbox::open(root).map_err(|error| error.to_string())?);
    }
    f(guard.as_mut().ok_or("QSO queue unavailable")?)
}

fn update_needs_gate(queue: &Queue, connection: &Connection) -> Result<(), String> {
    // Fail closed if persistent queue state cannot be inspected. No HTTP here.
    connection.needs_blocked.store(true, Ordering::Release);
    with_queue(queue, |outbox| {
        let blocked = crate::xlog_rx_worker::needs_blocked_by_outbox(
            &outbox.entries(),
            &connection.hello.workspace_id,
        );
        connection.needs_blocked.store(blocked, Ordering::Release);
        Ok(())
    })
}

#[tauri::command]
fn xlog_outbox_list(queue: State<'_, Queue>) -> Result<Vec<OutboxEntry>, String> {
    with_queue(&queue, |outbox| Ok(outbox.entries()))
}

#[tauri::command]
async fn xlog_outbox_enqueue(
    state: State<'_, Shared>,
    queue: State<'_, Queue>,
    request_id: String,
    mutation: LogbookMutation,
    expected_workspace_id: String,
) -> Result<Vec<OutboxEntry>, String> {
    let shared = Arc::clone(&state);
    let queue = Arc::clone(&queue);
    let (generation, connection) = current(&shared)?;
    tauri::async_runtime::spawn_blocking(move || {
        let client = lock(&connection.client)?;
        if lock(&shared)?.generation != generation {
            return Err("Connection changed".into());
        }
        if connection.hello.workspace_id != expected_workspace_id {
            return Err("The draft belongs to another xLog workspace".into());
        }
        client.snapshot().map_err(|error| error.to_string())?;
        if !connection
            .hello
            .capabilities
            .iter()
            .any(|capability| capability == "logbook.write")
        {
            return Err("xLog has not granted QSO writeback".into());
        }
        connection.needs_blocked.store(true, Ordering::Release);
        client.invalidate_needs();
        let result = with_queue(&queue, |outbox| {
            outbox
                .enqueue(&connection.hello.workspace_id, &request_id, mutation)
                .map_err(|error| error.to_string())?;
            Ok(outbox.entries())
        });
        let gate = update_needs_gate(&queue, &connection);
        client.invalidate_needs();
        gate?;
        result
    })
    .await
    .map_err(|_| "QSO queue request failed")?
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum DeliveryAction {
    Submit,
    Reconcile,
    RetryMissing,
}

#[tauri::command]
async fn xlog_outbox_deliver(
    state: State<'_, Shared>,
    queue: State<'_, Queue>,
    request_id: String,
    action: DeliveryAction,
) -> Result<CommandReceipt, String> {
    let shared = Arc::clone(&state);
    let queue = Arc::clone(&queue);
    let (generation, connection) = current(&shared)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut client = lock(&connection.client)?;
        if lock(&shared)?.generation != generation {
            return Err("Connection changed".into());
        }
        // The write/recovery may race an independent needs request. Revoke its
        // authority before dispatch and after any result, including uncertainty.
        // A durable unresolved entry also blocks attachment despite fresh snapshots.
        connection.needs_blocked.store(true, Ordering::Release);
        client.invalidate_needs();
        let result = with_queue(&queue, |outbox| {
            match action {
                DeliveryAction::Submit => outbox.submit(&mut client, &request_id),
                DeliveryAction::Reconcile => outbox.reconcile(&mut client, &request_id),
                DeliveryAction::RetryMissing => outbox.retry_missing(&mut client, &request_id),
            }
            .map_err(|error| error.to_string())
        });
        let gate = update_needs_gate(&queue, &connection);
        client.invalidate_needs();
        gate?;
        result
    })
    .await
    .map_err(|_| "QSO result unavailable; check its receipt before retrying")?
}

#[derive(Clone, Deserialize)]
struct Pairing {
    protocol_version: u32,
    application: String,
    endpoint: String,
    token: String,
}

struct Connection {
    client: Mutex<BridgeClient>,
    hello: Hello,
    needs_blocked: Arc<AtomicBool>,
}

#[derive(Default)]
struct Session {
    generation: u64,
    pairing: Option<Pairing>,
    connection: Option<Arc<Connection>>,
    stop: Option<StopClient>,
    rx: Option<crate::xlog_rx_worker::Worker>,
    rx_generation: u64,
}
type Shared = Arc<Mutex<Session>>;

#[derive(Serialize)]
struct View {
    paired: bool,
    connected: bool,
    endpoint: Option<String>,
    hello: Option<Hello>,
    snapshot: Option<Snapshot>,
}

fn view(session: &Session, snapshot: Option<Snapshot>) -> View {
    View {
        paired: session.pairing.is_some(),
        connected: session.connection.is_some(),
        endpoint: session.pairing.as_ref().map(|p| p.endpoint.clone()),
        hello: session.connection.as_ref().map(|c| c.hello.clone()),
        snapshot,
    }
}

fn lock<T>(mutex: &Mutex<T>) -> Result<std::sync::MutexGuard<'_, T>, String> {
    mutex
        .lock()
        .map_err(|_| "xLog connection state unavailable".into())
}

fn parse_pairing(bytes: &[u8]) -> Result<Pairing, String> {
    if bytes.len() > 16_384 {
        return Err("Pairing file exceeds the size limit".into());
    }
    let pairing: Pairing =
        serde_json::from_slice(bytes).map_err(|_| "Invalid xLog pairing file")?;
    if pairing.application != "OM0RX-xLog" || pairing.protocol_version != 1 {
        return Err("Unsupported xLog pairing file".into());
    }
    // Validates the literal loopback endpoint and secret without sending anything.
    BridgeClient::new(&pairing.endpoint, &pairing.token).map_err(|e| e.to_string())?;
    Ok(pairing)
}

fn begin(session: &Shared) -> Result<u64, String> {
    let mut session = lock(session)?;
    session.generation = session.generation.wrapping_add(1);
    session.rx = None;
    session.rx_generation = session.rx_generation.wrapping_add(1);
    session.connection = None;
    Ok(session.generation)
}

fn establish(shared: &Shared, generation: u64, pairing: Pairing) -> Result<View, String> {
    let mut client =
        BridgeClient::new(&pairing.endpoint, &pairing.token).map_err(|e| e.to_string())?;
    let hello = client.connect().map_err(|e| e.to_string())?.clone();
    let stop = if hello
        .capabilities
        .iter()
        .any(|capability| capability == "rotor.stop")
    {
        Some(client.stop_handle().map_err(|e| e.to_string())?)
    } else {
        None
    };
    let snapshot = client.poll().map_err(|e| e.to_string())?.clone();
    let mut session = lock(shared)?;
    if session.generation != generation {
        return Err("Connection request cancelled".into());
    }
    session.pairing = Some(pairing);
    session.stop = stop;
    session.connection = Some(Arc::new(Connection {
        client: Mutex::new(client),
        hello,
        needs_blocked: Arc::new(AtomicBool::new(true)),
    }));
    Ok(view(&session, Some(snapshot)))
}

#[tauri::command]
fn xlog_status(state: State<'_, Shared>) -> Result<View, String> {
    let session = lock(&state)?;
    Ok(view(&session, None))
}

#[tauri::command]
async fn xlog_import_pairing(
    app: tauri::AppHandle,
    state: State<'_, Shared>,
) -> Result<Option<View>, String> {
    use tauri_plugin_dialog::DialogExt;
    let shared = Arc::clone(&state);
    let generation = begin(&shared)?;
    lock(&shared)?.stop = None;
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .add_filter("xLog pairing", &["json"])
        .pick_file(move |picked| {
            let _ = tx.send(picked);
        });
    let Some(path) = rx
        .await
        .map_err(|_| "Pairing selection cancelled")?
        .and_then(|p| p.into_path().ok())
    else {
        return Ok(None);
    };
    tauri::async_runtime::spawn_blocking(move || {
        let mut bytes = Vec::new();
        File::open(path)
            .map_err(|_| "Cannot open pairing file")?
            .take(16_385)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read pairing file")?;
        let pairing = parse_pairing(&bytes)?;
        establish(&shared, generation, pairing).map(Some)
    })
    .await
    .map_err(|_| "Pairing request failed")?
}

#[tauri::command]
async fn xlog_connect(state: State<'_, Shared>) -> Result<View, String> {
    let shared = Arc::clone(&state);
    let generation = begin(&shared)?;
    lock(&shared)?.stop = None;
    let pairing = lock(&shared)?
        .pairing
        .clone()
        .ok_or("Import a pairing file from xLog first")?;
    tauri::async_runtime::spawn_blocking(move || establish(&shared, generation, pairing))
        .await
        .map_err(|_| "Connection request failed")?
}

fn current(shared: &Shared) -> Result<(u64, Arc<Connection>), String> {
    let session = lock(shared)?;
    Ok((
        session.generation,
        session.connection.clone().ok_or("xLog is disconnected")?,
    ))
}

#[tauri::command]
async fn xlog_poll(state: State<'_, Shared>) -> Result<View, String> {
    let shared = Arc::clone(&state);
    let (generation, connection) = current(&shared)?;
    tauri::async_runtime::spawn_blocking(move || {
        let (result, needs_observed) = {
            let mut client = lock(&connection.client)?;
            let result = client.poll().cloned().map_err(|e| e.to_string());
            // The base DTO may retain rejected optional metadata. Wake only
            // for an actually accepted SDK authority; no extra HTTP request.
            let valid = client
                .needs_handle()
                .is_ok_and(|needs| needs.remaining_authority_ms() > 0);
            let stamp = valid
                .then(|| {
                    result
                        .as_ref()
                        .ok()?
                        .needs
                        .as_ref()
                        .map(|m| m.generated_at_unix_ms)
                })
                .flatten();
            (result, stamp)
        };
        let mut session = lock(&shared)?;
        if session.generation != generation {
            return Err("Connection changed".into());
        }
        match result {
            Ok(snapshot) => {
                if let (Some(worker), Some(stamp)) = (&session.rx, needs_observed) {
                    worker.needs_snapshot_observed(stamp);
                }
                Ok(view(&session, Some(snapshot)))
            }
            Err(error) => {
                session.rx = None;
                session.connection = None;
                Err(error)
            }
        }
    })
    .await
    .map_err(|_| "Snapshot request failed")?
}

#[tauri::command]
fn xlog_rx_status(state: State<'_, Shared>) -> Result<crate::xlog_rx_worker::View, String> {
    lock(&state)?
        .rx
        .as_ref()
        .map_or_else(|| Ok(Default::default()), |worker| worker.view())
}

#[tauri::command]
fn xlog_rx_spectrum(state: State<'_, Shared>, after: Option<crate::xlog_rx_worker::SpectrumCursor>) -> Result<Option<crate::xlog_rx_worker::SpectrumFrame>, String> {
    lock(&state)?.rx.as_ref().map_or(Ok(None), |worker| worker.spectrum(after.as_ref()))
}

#[tauri::command]
fn xlog_rx_stop(state: State<'_, Shared>) -> Result<crate::xlog_rx_worker::View, String> {
    let mut session = lock(&state)?;
    session.rx_generation = session.rx_generation.wrapping_add(1);
    session.rx = None;
    Ok(Default::default())
}

#[tauri::command]
async fn xlog_rx_start(
    state: State<'_, Shared>,
    queue: State<'_, Queue>,
    mode: crate::xlog_rx_worker::Mode,
) -> Result<crate::xlog_rx_worker::View, String> {
    let shared = Arc::clone(&state);
    let queue = Arc::clone(&queue);
    let (generation, connection) = current(&shared)?;
    let rx_generation = {
        let mut session = lock(&shared)?;
        if session.generation != generation {
            return Err("Connection changed".into());
        }
        session.rx_generation = session.rx_generation.wrapping_add(1);
        session.rx = None;
        session.rx_generation
    };
    tauri::async_runtime::spawn_blocking(move || {
        let client = lock(&connection.client)?;
        update_needs_gate(&queue, &connection)?;
        client.snapshot().map_err(|error| error.to_string())?;
        let audio = client.audio_handle().map_err(|error| error.to_string())?;
        // Optional owner classification uses its own worker/HTTP lane. No fallback
        // to a local logbook or independently inferred entity/award state.
        let needs = client.needs_handle().ok();
        drop(client);
        let mut session = lock(&shared)?;
        if session.generation != generation || session.rx_generation != rx_generation {
            return Err("Receive request cancelled".into());
        }
        session.rx = None;
        let worker = crate::xlog_rx_worker::Worker::start_with_needs_gate(
            audio,
            connection.hello.server_instance.clone(),
            connection.hello.workspace_id.clone(),
            mode,
            needs,
            connection.needs_blocked.clone(),
        )?;
        let view = worker.view()?;
        session.rx = Some(worker);
        Ok(view)
    })
    .await
    .map_err(|_| "Receive start failed")?
}

#[tauri::command]
async fn xlog_command(
    state: State<'_, Shared>,
    request_id: String,
    command: Command,
) -> Result<CommandReceipt, String> {
    if matches!(
        command,
        Command::LogbookCreate { .. } | Command::LogbookUpdate { .. }
    ) {
        return Err("QSO changes must use the durable submission queue".into());
    }
    let shared = Arc::clone(&state);
    if let Command::RotorStop { rotor_id } = &command {
        let stop = lock(&shared)?
            .stop
            .clone()
            .ok_or("No paired rotator stop route")?;
        let rotor_id = rotor_id.clone();
        return tauri::async_runtime::spawn_blocking(move || {
            stop.stop_rotor(&request_id, &rotor_id)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|_| "Stop result unavailable; check xLog")?;
    }
    let (generation, connection) = current(&shared)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut client = lock(&connection.client)?;
        if lock(&shared)?.generation != generation {
            return Err("Connection changed".into());
        }
        // Stop has no poll mutex and is never held behind an ordinary command.
        let receipt = client
            .command(&request_id, command)
            .map_err(|e| e.to_string())?;
        if lock(&shared)?.generation != generation {
            return Err("Connection changed; check the action in xLog".into());
        }
        Ok(receipt)
    })
    .await
    .map_err(|_| "Command result unavailable; check xLog before retrying")?
}

#[tauri::command]
fn xlog_disconnect(state: State<'_, Shared>, forget: bool) -> Result<View, String> {
    begin(&state)?;
    let mut session = lock(&state)?;
    if forget {
        session.pairing = None;
        session.stop = None;
    }
    Ok(view(&session, None))
}

pub(crate) fn requested() -> bool {
    // The fork starts as xLog's companion by default. Independent upstream
    // operation is a separate explicit launch, never a disconnected fallback.
    !std::env::args()
        .skip(1)
        .any(|arg| arg == "--independent-mode")
}

fn startup_failure(reason: &str) -> ! {
    eprintln!("OM0RX-xNexus could not start: {reason}");
    #[cfg(windows)]
    {
        extern "system" {
            fn MessageBoxW(
                hwnd: *mut std::ffi::c_void,
                text: *const u16,
                caption: *const u16,
                utype: u32,
            ) -> i32;
        }
        let wide = |value: &str| {
            value
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect::<Vec<u16>>()
        };
        let body = wide(&format!("OM0RX-xNexus could not open its companion profile.\n\n{reason}\n\nNo station or fallback profile was opened. Resolve the existing instance or storage issue, then launch again."));
        let title = wide("OM0RX-xNexus — companion startup failed");
        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                body.as_ptr(),
                title.as_ptr(),
                0x0001_0010,
            );
        }
    }
    std::process::exit(1)
}

// Keep the real framework adapter here; the small construction helper can be
// tested without opening any native window or depending on mock runtime internals.
impl<'a, R: tauri::Runtime, M: tauri::Manager<R>> private_window::Builder
    for tauri::WebviewWindowBuilder<'a, R, M>
{
    type Output = tauri::WebviewWindow<R>;
    type Error = tauri::Error;
    fn data_directory(self, directory: std::path::PathBuf) -> Self {
        tauri::WebviewWindowBuilder::data_directory(self, directory)
    }
    fn data_store_identifier(self, identifier: [u8; 16]) -> Self {
        tauri::WebviewWindowBuilder::data_store_identifier(self, identifier)
    }
    fn build(self) -> Result<Self::Output, Self::Error> {
        tauri::WebviewWindowBuilder::build(self)
    }
}

pub(crate) fn run() {
    let root = data_root().unwrap_or_else(|error| startup_failure(&error));
    // Acquire ownership BEFORE opening the WebView: its persistent draft and the
    // native queue must have one writer. A second instance fails, never falls back.
    let outbox = Outbox::open(root.join("xlog-outbox"))
        .unwrap_or_else(|error| startup_failure(&error.to_string()));
    let queue: Queue = Arc::new(Mutex::new(Some(outbox)));
    #[cfg(target_os = "macos")]
    let store =
        profile_store::for_current_system(&root).unwrap_or_else(|error| startup_failure(&error));
    #[cfg(target_os = "macos")]
    let store = Some(store);
    #[cfg(not(target_os = "macos"))]
    let store = None;
    let mut context = tauri::generate_context!();
    let mut window = context
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "main")
        .cloned()
        .unwrap_or_else(|| startup_failure("Companion main window configuration is missing"));
    // No automatic WebView may be created from configuration: the locked Tauri
    // conversion drops both storage fields. Preserve the window options, then
    // apply storage using the explicit builder setters in setup.
    context.config_mut().app.windows.clear();
    window.url = tauri::WebviewUrl::App("xlog.html".into());
    window.title = "OM0RX-xNexus · xLog companion".into();
    window.visible = true;
    window.min_width = Some(780.0);
    window.min_height = Some(438.0);
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(Shared::default())
        .manage(queue)
        .setup(move |app| {
            let builder = tauri::WebviewWindowBuilder::from_config(app, &window)?;
            private_window::build(builder, root.join("webview"), store)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            xlog_status,
            xlog_import_pairing,
            xlog_connect,
            xlog_poll,
            xlog_command,
            xlog_disconnect,
            xlog_outbox_list,
            xlog_outbox_enqueue,
            xlog_outbox_deliver,
            xlog_history,
            xlog_rx_status,
            xlog_rx_spectrum,
            xlog_rx_start,
            xlog_rx_stop
        ])
        .run(context)
        .expect("xLog companion desktop failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_private_root_is_shared_and_never_falls_back() {
        let fallback = std::env::temp_dir().join("xnexus-fallback");
        let explicit = std::env::temp_dir().join("xnexus-isolated");
        let args = vec![
            "--xlog-data-dir".into(),
            explicit.to_string_lossy().into_owned(),
        ];
        assert_eq!(
            data_root_from(&args, None, fallback.clone()).unwrap(),
            explicit
        );
        assert!(
            data_root_from(&["--xlog-data-dir=relative".into()], None, fallback.clone()).is_err()
        );
        assert!(data_root_from(&["--xlog-data-dir".into()], None, fallback).is_err());
    }

    #[test]
    fn read_only_bridge_connects_without_rotator_capability() {
        use std::io::Write;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let hello = serde_json::json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":"read-only-fixture","workspace_id":"fixture-workspace","capabilities":["snapshot.read"]});
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let snapshot = serde_json::json!({"protocol_version":1,"server_instance":"read-only-fixture","workspace_id":"fixture-workspace","revision":1,"generated_at_unix_ms":now,"max_age_ms":10000,"station":{"connected":false,"radio_id":null,"frequency_hz":null,"mode":null,"transmitting":null},"rotor":{"connected":false,"rotor_id":null,"azimuth_deg":null,"moving":null},"spots":[]});
        let server = std::thread::spawn(move || {
            for body in [hello.to_string(), snapshot.to_string()] {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 4096];
                stream.read(&mut request).unwrap();
                write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
            }
        });
        let shared = Shared::default();
        let generation = begin(&shared).unwrap();
        let pairing = Pairing {
            protocol_version: 1,
            application: "OM0RX-xLog".into(),
            endpoint,
            token: "synthetic-read-only-pairing-marker".into(),
        };
        assert!(establish(&shared, generation, pairing).unwrap().connected);
        assert!(lock(&shared).unwrap().stop.is_none());
        server.join().unwrap();
    }
    #[test]
    fn pairing_requires_xlog_loopback_and_valid_secret() {
        let mut value = serde_json::json!({"application":"OM0RX-xLog","protocol_version":1,"endpoint":"http://127.0.0.1:49123","token":"synthetic-pairing-marker-for-tests-only"});
        assert!(parse_pairing(&serde_json::to_vec(&value).unwrap()).is_ok());
        value["endpoint"] = "http://example.com:49123".into();
        assert!(parse_pairing(&serde_json::to_vec(&value).unwrap()).is_err());
        value["endpoint"] = "http://127.0.0.1:49123".into();
        value["application"] = "Nexus".into();
        assert!(parse_pairing(&serde_json::to_vec(&value).unwrap()).is_err());
        assert!(parse_pairing(&vec![b' '; 16_385]).is_err());
    }
    #[test]
    fn disconnect_invalidates_inflight_generation() {
        let shared = Shared::default();
        let first = begin(&shared).unwrap();
        let second = begin(&shared).unwrap();
        assert_ne!(first, second);
        assert!(current(&shared).is_err());
    }
}
