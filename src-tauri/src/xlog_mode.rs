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
#[path = "xlog_bearing_read.rs"]
mod passive_bearing;
#[path = "xlog_contact_detail_read.rs"]
mod passive_contact_detail;
#[path = "xlog_history_read.rs"]
mod passive_history;
#[path = "xlog_window.rs"]
mod private_window;
#[cfg(target_os = "macos")]
#[path = "xlog_profile_store.rs"]
mod profile_store;
use xnexus_xlog_client::outbox::{LogbookMutation, Outbox, OutboxEntry};
use xnexus_xlog_client::{
    BridgeClient, Command, CommandReceipt, Hello, History, HistoryQuery, Recent, Snapshot,
    StopClient,
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

fn current_read(shared: &Shared, expected_token: &str) -> Result<(u64, Arc<Connection>), String> {
    let (generation, connection) = current(shared)?;
    if connection.history.token() != expected_token {
        return Err("Connection changed".into());
    }
    Ok((generation, connection))
}

fn check_read_connection(
    shared: &Shared,
    generation: u64,
    connection: &Arc<Connection>,
) -> Result<(), String> {
    let session = lock(shared)?;
    if session.generation != generation
        || !session
            .connection
            .as_ref()
            .is_some_and(|live| Arc::ptr_eq(live, connection))
    {
        return Err("Connection changed".into());
    }
    Ok(())
}

#[tauri::command]
async fn xlog_history_read(
    state: State<'_, Shared>,
    callsign: String,
    band: Option<String>,
    exact_mode: Option<String>,
    connection_token: String,
) -> Result<passive_history::Response<History>, String> {
    let query = HistoryQuery::new(&callsign, band.as_deref(), exact_mode.as_deref())
        .map_err(|error| error.to_string())?;
    let shared = Arc::clone(&state);
    let (generation, connection) = current_read(&shared, &connection_token)?;
    tauri::async_runtime::spawn_blocking(move || {
        check_read_connection(&shared, generation, &connection)?;
        let response = connection.history.history(&connection_token, &query)?;
        check_read_connection(&shared, generation, &connection)?;
        Ok(response)
    })
    .await
    .map_err(|_| "xLog history read failed")?
}

#[tauri::command]
async fn xlog_recent(
    state: State<'_, Shared>,
    connection_token: String,
) -> Result<passive_history::Response<Recent>, String> {
    let shared = Arc::clone(&state);
    let (generation, connection) = current_read(&shared, &connection_token)?;
    tauri::async_runtime::spawn_blocking(move || {
        check_read_connection(&shared, generation, &connection)?;
        let response = connection.history.recent(&connection_token)?;
        check_read_connection(&shared, generation, &connection)?;
        Ok(response)
    })
    .await
    .map_err(|_| "xLog recent contacts read failed")?
}

#[tauri::command]
async fn xlog_bearing_read(
    state: State<'_, Shared>,
    connection_token: String,
    callsign: String,
) -> Result<passive_bearing::Response, String> {
    let shared = Arc::clone(&state);
    let (generation, connection) = current_read(&shared, &connection_token)?;
    let prepared = connection
        .bearing
        .capture(&connection_token, &callsign, || {
            let client = connection
                .client
                .try_lock()
                .map_err(|_| "xLog bearing context is busy or unavailable")?;
            client.bearing_context().map_err(|error| error.to_string())
        })?;
    tauri::async_runtime::spawn_blocking(move || {
        check_read_connection(&shared, generation, &connection)?;
        let response = connection.bearing.read(prepared)?;
        check_read_connection(&shared, generation, &connection)?;
        Ok(response)
    })
    .await
    .map_err(|_| "xLog bearing read failed")?
}

#[tauri::command]
async fn xlog_contact_detail_read(
    state: State<'_, Shared>,
    connection_token: String,
    contact_id: String,
) -> Result<passive_contact_detail::Response, String> {
    let shared = Arc::clone(&state);
    let (generation, connection) = current_read(&shared, &connection_token)?;
    // Capture selection and original read start before entering the blocking pool.
    let prepared = connection.detail.capture(&connection_token, &contact_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        check_read_connection(&shared, generation, &connection)?;
        let response = connection.detail.read(prepared)?;
        check_read_connection(&shared, generation, &connection)?;
        Ok(response)
    })
    .await
    .map_err(|_| "xLog contact detail read failed")?
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
        deliver_queued(
            &shared,
            &queue,
            generation,
            &connection,
            &request_id,
            action,
        )
    })
    .await
    .map_err(|_| "QSO result unavailable; check its receipt before retrying")?
}

fn deliver_queued(
    shared: &Shared,
    queue: &Queue,
    generation: u64,
    connection: &Arc<Connection>,
    request_id: &str,
    action: DeliveryAction,
) -> Result<CommandReceipt, String> {
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
    history: passive_history::Lane,
    detail: passive_contact_detail::Lane,
    bearing: passive_bearing::Lane,
    hello: Hello,
    ft: Option<Arc<crate::xlog_ft_host::Host>>,
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
    ft_last: Option<Arc<crate::xlog_ft_host::Host>>,
}
type Shared = Arc<Mutex<Session>>;

#[derive(Serialize)]
struct View {
    paired: bool,
    connected: bool,
    endpoint: Option<String>,
    hello: Option<Hello>,
    snapshot: Option<Snapshot>,
    connection_token: Option<String>,
}

fn view(session: &Session, snapshot: Option<Snapshot>) -> View {
    View {
        paired: session.pairing.is_some(),
        connected: session.connection.is_some(),
        endpoint: session.pairing.as_ref().map(|p| p.endpoint.clone()),
        hello: session.connection.as_ref().map(|c| c.hello.clone()),
        snapshot,
        connection_token: session
            .connection
            .as_ref()
            .map(|c| c.history.token().to_owned()),
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
    Ok(retire_connection(&mut session))
}

fn retire_connection(session: &mut Session) -> u64 {
    if let Some(connection) = &session.connection {
        if let Some(ft) = &connection.ft {
            ft.revoke();
        }
        connection.history.revoke();
        connection.detail.revoke();
        connection.bearing.revoke();
    }
    session.generation = session.generation.wrapping_add(1);
    session.rx = None;
    session.rx_generation = session.rx_generation.wrapping_add(1);
    session.connection = None;
    session.generation
}

fn published_pairing() -> Result<Pairing, String> {
    let root = std::env::var_os("LOCALAPPDATA").ok_or("Waiting for the local xLog connection")?;
    let path = std::path::PathBuf::from(root)
        .join("OM0RX/xLog/companion/connection.json");
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| "Waiting for xLog to start")?
        .take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| "Waiting for the local xLog connection")?;
    parse_pairing(&bytes)
}

fn same_pairing(left: Option<&Pairing>, right: &Pairing) -> bool {
    left.is_some_and(|left| left.endpoint == right.endpoint && left.token == right.token)
}

fn connect_published(shared: &Shared, expected: u64, pairing: Pairing) -> Result<View, String> {
    let generation = {
        let mut session = lock(shared)?;
        if session.generation != expected {
            return Err("Connection request cancelled".into());
        }
        if session.connection.is_some() && same_pairing(session.pairing.as_ref(), &pairing) {
            return Ok(view(&session, None));
        }
        let generation = retire_connection(&mut session);
        session.stop = None;
        session.pairing = Some(pairing.clone());
        generation
    };
    establish(shared, generation, pairing)
}

fn transient_poll_error(error: &xnexus_xlog_client::Error) -> bool {
    use xnexus_xlog_client::Error;
    matches!(error, Error::Transport | Error::Stale | Error::Http(408 | 429 | 500 | 502 | 503 | 504))
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
    let history = passive_history::Lane::new(&client)?;
    let detail = passive_contact_detail::Lane::new(&client, history.token())?;
    let bearing = passive_bearing::Lane::new(&client, history.token())?;
    let prior_pending = lock(shared)?
        .ft_last
        .as_ref()
        .is_some_and(|h| h.has_release());
    let ft = if prior_pending {
        None
    } else {
        crate::xlog_ft_host::Host::attach(&client, history.token().into())
            .ok()
            .map(Arc::new)
    };
    let mut session = lock(shared)?;
    if session.generation != generation {
        return Err("Connection request cancelled".into());
    }
    if let Some(ft) = &ft {
        session.ft_last = Some(ft.clone());
    }
    session.pairing = Some(pairing);
    session.stop = stop;
    session.connection = Some(Arc::new(Connection {
        client: Mutex::new(client),
        history,
        detail,
        bearing,
        hello,
        ft,
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
    let generation = lock(&shared)?.generation;
    tauri::async_runtime::spawn_blocking(move || {
        connect_published(&shared, generation, published_pairing()?)
    })
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
        // The owner atomically replaces this descriptor on a real restart or
        // workspace change. An unchanged descriptor never re-handshakes a
        // healthy (or temporarily slow) session.
        let pairing = published_pairing()?;
        let replaced = {
            let session = lock(&shared)?;
            if session.generation != generation { return Err("Connection changed".into()); }
            !same_pairing(session.pairing.as_ref(), &pairing)
        };
        if replaced { return connect_published(&shared, generation, pairing); }
        let (result, needs_observed) = {
            let mut client = lock(&connection.client)?;
            let result = client.poll().cloned();
            // The base DTO may retain rejected optional metadata. Wake only
            // for an actually accepted SDK authority; no extra HTTP request.
            let valid = client.needs_remaining_authority_ms() > 0;
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
                if transient_poll_error(&error) {
                    // Observation leases expire independently. Do not retire
                    // the audio cursor, FT session or retained exact Stop lane.
                    return Err(error.to_string());
                }
                if let Some(ft) = &connection.ft {
                    ft.revoke();
                }
                connection.history.revoke();
                connection.detail.revoke();
                connection.bearing.revoke();
                session.rx = None;
                session.connection = None;
                Err(error.to_string())
            }
        }
    })
    .await
    .map_err(|_| "Snapshot request failed")?
}

fn ft_handle(
    shared: &Shared,
    token: &str,
    stop: bool,
) -> Result<Arc<crate::xlog_ft_host::Host>, String> {
    let session = lock(shared)?;
    if let Some(c) = &session.connection {
        if c.history.token() == token {
            return c
                .ft
                .clone()
                .or_else(|| session.ft_last.clone())
                .ok_or("Owner digital TX capability unavailable".into());
        }
    }
    if stop {
        if let Some(h) = &session.ft_last {
            if h.token() == token {
                return Ok(h.clone());
            }
        }
    }
    Err("FT connection changed".into())
}
#[tauri::command]
fn xlog_ft_status(
    state: State<'_, Shared>,
    connection_token: String,
) -> Result<crate::xlog_ft_host::FtView, String> {
    let h = ft_handle(&state, &connection_token, true)?;
    let mut view = h.view()?;
    if h.token() != connection_token {
        view.connection_token = connection_token;
        view.available = false;
        view.armed = false;
        view.remaining_valid_ms = 0;
        view.reason = "Previous exact TX release remains pending; reconnect after release".into();
    }
    Ok(view)
}
#[tauri::command]
async fn xlog_ft_action(
    state: State<'_, Shared>,
    connection_token: String,
    action: crate::xlog_ft_host::Action,
) -> Result<crate::xlog_ft_host::FtView, String> {
    let stop = matches!(
        action,
        crate::xlog_ft_host::Action::Stop | crate::xlog_ft_host::Action::Tune { enabled: false }
    );
    let h = ft_handle(&state, &connection_token, stop)?;
    if stop {
        h.stop();
        return h.view();
    }
    if h.token() != connection_token {
        return Err("Previous exact TX release remains pending".into());
    }
    if matches!(
        action,
        crate::xlog_ft_host::Action::Enable { enabled: false }
    ) {
        h.disarm_future()?;
        return h.view();
    }
    tauri::async_runtime::spawn_blocking(move || h.action(action))
        .await
        .map_err(|_| "FT action failed")?
}
#[tauri::command]
async fn xlog_ft_log(
    state: State<'_, Shared>,
    queue: State<'_, Queue>,
    connection_token: String,
) -> Result<Vec<OutboxEntry>, String> {
    let shared = Arc::clone(&state);
    let queue = Arc::clone(&queue);
    let (generation, c) = current_read(&shared, &connection_token)?;
    let h = c.ft.as_ref().ok_or("FT runtime unavailable")?;
    let (request, qso, workspace) = h.log_draft()?;
    if workspace != c.hello.workspace_id {
        return Err("QSO belongs to another owner workspace".into());
    }
    tauri::async_runtime::spawn_blocking(move || {
        check_read_connection(&shared, generation, &c)?;
        {
            let client = lock(&c.client)?;
            client.snapshot().map_err(|e| e.to_string())?;
            if !c.hello.capabilities.iter().any(|x| x == "logbook.write") {
                return Err("xLog has not granted QSO writeback".into());
            }
            client.invalidate_needs();
            c.needs_blocked.store(true, Ordering::Release);
        }
        let state = with_queue(&queue, |outbox| {
            outbox
                .enqueue(
                    &c.hello.workspace_id,
                    &request,
                    LogbookMutation::Create { qso },
                )
                .map_err(|e| e.to_string())?;
            Ok(outbox
                .entry(&request, &c.hello.workspace_id)
                .map_err(|e| e.to_string())?
                .state
                .clone())
        })?;
        use xnexus_xlog_client::outbox::OutboxState;
        let action = match state {
            OutboxState::Pending => Some(DeliveryAction::Submit),
            OutboxState::NeedsReceipt => Some(DeliveryAction::Reconcile),
            OutboxState::Settled(_) => None,
        };
        if let Some(action) = action {
            let _ = deliver_queued(&shared, &queue, generation, &c, &request, action);
        }
        update_needs_gate(&queue, &c)?;
        check_read_connection(&shared, generation, &c)?;
        with_queue(&queue, |outbox| {
            Ok(vec![outbox
                .entry(&request, &c.hello.workspace_id)
                .map_err(|e| e.to_string())?
                .clone()])
        })
    })
    .await
    .map_err(|_| "QSO result unavailable; durable queue retained")?
}

#[tauri::command]
fn xlog_rx_status(state: State<'_, Shared>) -> Result<crate::xlog_rx_worker::View, String> {
    lock(&state)?
        .rx
        .as_ref()
        .map_or_else(|| Ok(Default::default()), |worker| worker.view())
}

#[tauri::command]
fn xlog_rx_spectrum(
    state: State<'_, Shared>,
    after: Option<crate::xlog_rx_worker::SpectrumCursor>,
) -> Result<Option<crate::xlog_rx_worker::SpectrumFrame>, String> {
    lock(&state)?
        .rx
        .as_ref()
        .map_or(Ok(None), |worker| worker.spectrum(after.as_ref()))
}

#[tauri::command]
fn xlog_rx_stop(state: State<'_, Shared>) -> Result<crate::xlog_rx_worker::View, String> {
    let mut session = lock(&state)?;
    if let Some(ft) = &session.ft_last {
        ft.feed().retire();
    }
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
        let worker = crate::xlog_rx_worker::Worker::start_with_ft(
            audio,
            connection.hello.server_instance.clone(),
            connection.hello.workspace_id.clone(),
            mode,
            needs,
            connection.needs_blocked.clone(),
            connection.ft.as_ref().map(|ft| ft.feed()),
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

/// The mode argument a relaunch of this process must carry so it comes back in the
/// same operating mode. Companion mode is the default, so an independent-mode
/// process that relaunched itself without the flag (radio picker, client/station
/// switch) would silently come back as xLog's companion instead.
pub(crate) fn relaunch_mode_args() -> &'static [&'static str] {
    mode_args(requested())
}

fn mode_args(companion: bool) -> &'static [&'static str] {
    if companion {
        &[]
    } else {
        &["--independent-mode"]
    }
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
            xlog_ft_status,
            xlog_ft_action,
            xlog_ft_log,
            xlog_import_pairing,
            xlog_connect,
            xlog_poll,
            xlog_command,
            xlog_disconnect,
            xlog_outbox_list,
            xlog_outbox_enqueue,
            xlog_outbox_deliver,
            xlog_history,
            xlog_history_read,
            xlog_recent,
            xlog_contact_detail_read,
            xlog_bearing_read,
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
    fn a_relaunch_keeps_independent_mode_and_companion_mode_adds_nothing() {
        assert_eq!(mode_args(false), ["--independent-mode"]);
        assert!(mode_args(true).is_empty());
        // The flag a relaunch carries is exactly the one `requested` reads.
        assert!(mode_args(false).iter().all(|a| *a == "--independent-mode"));
    }
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
        let hello = serde_json::json!({"application":"OM0RX-xLog","protocol_version":1,"server_instance":"read-only-fixture","workspace_id":"fixture-workspace","capabilities":["snapshot.read","logbook.read"]});
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let snapshot = serde_json::json!({"protocol_version":1,"server_instance":"read-only-fixture","workspace_id":"fixture-workspace","revision":1,"generated_at_unix_ms":now,"max_age_ms":10000,"station":{"connected":false,"radio_id":null,"frequency_hz":null,"mode":null,"transmitting":null},"rotor":{"connected":false,"rotor_id":null,"azimuth_deg":null,"moving":null},"spots":[]});
        let history = serde_json::json!({"protocol_version":1,"server_instance":"read-only-fixture","workspace_id":"fixture-workspace","callsign":"K1ABC","generated_at_unix_ms":now,"contacts":[],"matrix":[]});
        let server = std::thread::spawn(move || {
            for body in [hello.to_string(), snapshot.to_string(), history.to_string()] {
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
        let (_, retained) = current(&shared).unwrap();
        let old_token = retained.history.token().to_owned();
        let response = retained
            .history
            .history(&old_token, &HistoryQuery::new("K1ABC", None, None).unwrap())
            .unwrap();
        assert!(serde_json::to_value(&response).is_ok());
        server.join().unwrap();
        assert_eq!(
            view(&lock(&shared).unwrap(), None)
                .connection_token
                .as_deref(),
            Some(old_token.as_str())
        );
        begin(&shared).unwrap();
        assert!(serde_json::to_value(&response).is_err());
        // Retaining the full connection (and its BridgeClient) cannot postpone
        // retirement, and a view never exposes the previous connection token.
        assert!(check_read_connection(&shared, generation, &retained).is_err());
        assert!(
            matches!(retained.history.recent(&old_token), Err(error) if error == "Connection changed")
        );
        assert!(view(&lock(&shared).unwrap(), None)
            .connection_token
            .is_none());
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
