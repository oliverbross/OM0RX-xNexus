// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 OM0RX
//! A single-writer, immutable-file QSO outbox. Only xLog commits contacts.
//!
//! Enqueue and attempt markers are synced before reporting success/network dispatch.
//! Crash after dispatch leaves NeedsReceipt, never automatic resend. Records remain
//! bound to one persistent workspace. Terminal receipts may be archived, never forgotten.
//! Atomic publication never replaces an existing filename (Unix hard-link commit,
//! Windows write-through move); temporary files are not authoritative.
//! partial temporary files are not authoritative. Corrupt committed files fail closed.
use crate::{bounded, commands::valid_request_id, frequency, CommandReceipt, CommandStatus};
use crate::{BridgeClient, Command, Error};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
mod archive;
pub use archive::ArchiveOutcome;
const MAX_ACTIVE_ENTRIES: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QsoDraft {
    pub contact_id: String,
    pub callsign: String,
    pub band: String,
    pub mode: String,
    pub started_at_unix_ms: u64,
    pub frequency_hz: Option<u64>,
    pub rst_sent: String,
    pub rst_received: String,
    pub grid: String,
    pub comment: String,
}
impl QsoDraft {
    pub fn validate(&self) -> Result<(), OutboxError> {
        if !valid_request_id(&self.contact_id)
            || !bounded(&self.callsign, 32)
            || !self
                .callsign
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'/' || b == b'-')
            || !bounded(&self.band, 16)
            || !bounded(&self.mode, 16)
            || self.started_at_unix_ms > 4_102_444_800_000
            || self.frequency_hz.is_some_and(|f| !frequency(f))
            || !optional_text(&self.rst_sent, 16)
            || !optional_text(&self.rst_received, 16)
            || !optional_text(&self.grid, 12)
            || self.comment.len() > 4096
            || self
                .comment
                .chars()
                .any(|v| v.is_control() && !matches!(v, '\r' | '\n' | '\t'))
        {
            return Err(OutboxError::Invalid);
        }
        Ok(())
    }
}
fn optional_text(value: &str, max: usize) -> bool {
    value.is_empty() || bounded(value, max)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
pub enum LogbookMutation {
    #[serde(rename = "logbook.create")]
    Create { qso: QsoDraft },
    #[serde(rename = "logbook.update")]
    Update {
        qso: QsoDraft,
        expected_contact_revision: u64,
    },
}
impl From<LogbookMutation> for Command {
    fn from(m: LogbookMutation) -> Self {
        match m {
            LogbookMutation::Create { qso } => Self::LogbookCreate { qso },
            LogbookMutation::Update {
                qso,
                expected_contact_revision,
            } => Self::LogbookUpdate {
                qso,
                expected_contact_revision,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryError {
    Queue(OutboxError),
    Bridge(Error),
}
impl std::fmt::Display for DeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Queue(e) => e.fmt(f),
            Self::Bridge(e) => e.fmt(f),
        }
    }
}
impl std::error::Error for DeliveryError {}
impl From<OutboxError> for DeliveryError {
    fn from(e: OutboxError) -> Self {
        Self::Queue(e)
    }
}
impl From<Error> for DeliveryError {
    fn from(e: Error) -> Self {
        Self::Bridge(e)
    }
}
impl LogbookMutation {
    pub fn qso(&self) -> &QsoDraft {
        match self {
            Self::Create { qso } | Self::Update { qso, .. } => qso,
        }
    }
    fn validate(&self) -> Result<(), OutboxError> {
        self.qso().validate()?;
        if matches!(self,Self::Update{expected_contact_revision,..} if !(1..=9_007_199_254_740_990).contains(expected_contact_revision))
        {
            return Err(OutboxError::Invalid);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueuedQso {
    pub version: u32,
    pub request_id: String,
    pub workspace_id: String,
    pub mutation: LogbookMutation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutboxState {
    Pending,
    NeedsReceipt,
    Settled(CommandReceipt),
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboxEntry {
    pub queued: QueuedQso,
    pub state: OutboxState,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxError {
    Io,
    Locked,
    Corrupt,
    Invalid,
    Reused,
    WorkspaceChanged,
    NotFound,
    ReceiptRequired,
    Full,
}
impl std::fmt::Display for OutboxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Io => "QSO queue persistence failed; check the retained operation receipt before another submission",
            Self::Locked => "The QSO queue is already open in another process",
            Self::Corrupt => "The QSO queue requires recovery; its files were preserved",
            Self::Invalid => "Invalid QSO queue entry",
            Self::Reused => "This request identifier already belongs to another QSO change",
            Self::WorkspaceChanged => "The queued QSO belongs to a different xLog workspace",
            Self::NotFound => "Queued QSO not found",
            Self::ReceiptRequired => "Check the xLog receipt before retrying this QSO",
            Self::Full => "The active QSO queue or its retained journal files reached their limit",
        })
    }
}
impl std::error::Error for OutboxError {}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attempt {
    version: u32,
    request_id: String,
    workspace_id: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settled {
    version: u32,
    receipt: CommandReceipt,
}

pub struct Outbox {
    root: PathBuf,
    _lock: File,
    entries: BTreeMap<String, OutboxEntry>,
    archive_cleanup_pending: BTreeSet<String>,
    active_limit: usize,
}
impl Outbox {
    /// An explicit user action, never a timer. A previous uncertain attempt is
    /// reconciled by receipt lookup; an absent receipt is not automatic retry.
    pub fn submit(
        &mut self,
        client: &mut BridgeClient,
        request_id: &str,
    ) -> Result<CommandReceipt, DeliveryError> {
        let hello = client.hello().ok_or(Error::Disconnected)?;
        let entry = self.entry(request_id, &hello.workspace_id)?;
        match entry.state {
            OutboxState::Settled(receipt) => Ok(receipt),
            OutboxState::NeedsReceipt => self.reconcile(client, request_id),
            OutboxState::Pending => {
                if !hello.capabilities.iter().any(|c| c == "logbook.write") {
                    return Err(Error::Capability.into());
                }
                client.snapshot()?;
                self.mark_attempted(request_id, &entry.queued.workspace_id)?;
                self.send_queued(client, entry.queued)
            }
        }
    }
    pub fn reconcile(
        &mut self,
        client: &mut BridgeClient,
        request_id: &str,
    ) -> Result<CommandReceipt, DeliveryError> {
        let hello = client.hello().ok_or(Error::Disconnected)?;
        if let OutboxState::Settled(receipt) = self.entry(request_id, &hello.workspace_id)?.state {
            return Ok(receipt);
        }
        let receipt = client
            .receipt(request_id)?
            .ok_or(OutboxError::ReceiptRequired)?;
        if terminal(receipt.status) {
            self.settle(receipt.clone())?;
        }
        Ok(receipt)
    }
    /// Separate explicit retry button. Re-check durable absence at the current
    /// workspace immediately before repeating the SAME immutable operation ID.
    pub fn retry_missing(
        &mut self,
        client: &mut BridgeClient,
        request_id: &str,
    ) -> Result<CommandReceipt, DeliveryError> {
        let hello = client.hello().ok_or(Error::Disconnected)?;
        let entry = self.entry(request_id, &hello.workspace_id)?;
        if entry.state != OutboxState::NeedsReceipt {
            return self.submit(client, request_id);
        }
        if let Some(receipt) = client.receipt(request_id)? {
            if terminal(receipt.status) {
                self.settle(receipt.clone())?;
            }
            return Ok(receipt);
        }
        // The existing durable attempt marker still fences another crash.
        self.send_queued(client, entry.queued)
    }
    fn send_queued(
        &mut self,
        client: &mut BridgeClient,
        queued: QueuedQso,
    ) -> Result<CommandReceipt, DeliveryError> {
        let receipt = client.command(&queued.request_id, queued.mutation.into())?;
        if terminal(receipt.status) {
            self.settle(receipt.clone())?;
        }
        Ok(receipt)
    }
    pub fn open(root: impl AsRef<Path>) -> Result<Self, OutboxError> {
        Self::open_with_limit(root, MAX_ACTIVE_ENTRIES)
    }
    // The public API always uses MAX_ACTIVE_ENTRIES. A lower internal limit
    // lets crash/capacity tests exercise the real I/O path without 10,000 files.
    fn open_with_limit(root: impl AsRef<Path>, active_limit: usize) -> Result<Self, OutboxError> {
        if !(1..=MAX_ACTIVE_ENTRIES).contains(&active_limit) {
            return Err(OutboxError::Invalid);
        }
        let root = root.as_ref();
        fs::create_dir_all(root).map_err(|_| OutboxError::Io)?;
        archive::checked_directory(root)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(root, fs::Permissions::from_mode(0o700))
                .map_err(|_| OutboxError::Io)?;
        }
        archive::checked_optional_file(&root.join("queue.lock"))?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("queue.lock"))
            .map_err(|_| OutboxError::Io)?;
        lock.try_lock().map_err(|_| OutboxError::Locked)?;
        let mut entries = BTreeMap::new();
        let mut attempts = Vec::new();
        let mut settled = Vec::new();
        let mut archive_cleanup_pending = BTreeSet::new();
        let mut count = 0;
        for item in fs::read_dir(root).map_err(|_| OutboxError::Io)? {
            let item = item.map_err(|_| OutboxError::Io)?;
            let name = item.file_name();
            let name = name.to_str().ok_or(OutboxError::Corrupt)?;
            count += 1;
            // 10,000 active three-file journals, lock, and two private directories.
            // Archived contents are deliberately not enumerated at startup.
            if count > active_limit * 3 + 3 {
                return Err(OutboxError::Full);
            }
            if name == "queue.lock" || name.ends_with(".tmp") {
                continue;
            }
            if matches!(name, "archive-v1" | "exports-v1") {
                archive::checked_directory(&item.path())?;
                continue;
            }
            archive::checked_optional_file(&item.path())?;
            if let Some(id) = archive::journal_id(name) {
                if let Some(retained) = archive::read_archived(root, id).map_err(|error| {
                    if error == OutboxError::Invalid {
                        OutboxError::Corrupt
                    } else {
                        error
                    }
                })? {
                    archive::validate_fragment(&item.path(), name, &retained)?;
                    archive_cleanup_pending.insert(retained.queued.request_id);
                    continue;
                }
            }
            if let Some(id) = name.strip_suffix(".queued.json") {
                let queued: QueuedQso = read_json(&item.path())?;
                if queued.version != 1
                    || !valid_request_id(id)
                    || queued.request_id != id
                    || !bounded(&queued.workspace_id, 128)
                    || queued.mutation.validate().is_err()
                {
                    return Err(OutboxError::Corrupt);
                }
                if entries.len() >= active_limit {
                    return Err(OutboxError::Full);
                }
                if entries
                    .insert(
                        id.to_ascii_lowercase(),
                        OutboxEntry {
                            queued,
                            state: OutboxState::Pending,
                        },
                    )
                    .is_some()
                {
                    return Err(OutboxError::Corrupt);
                }
            } else if let Some(id) = name.strip_suffix(".attempted.json") {
                let attempt: Attempt = read_json(&item.path())?;
                if attempt.version != 1 || attempt.request_id != id || !valid_request_id(id) {
                    return Err(OutboxError::Corrupt);
                }
                attempts.push(attempt);
            } else if let Some(id) = name.strip_suffix(".settled.json") {
                let s: Settled = read_json(&item.path())?;
                if s.version != 1 || s.receipt.request_id != id {
                    return Err(OutboxError::Corrupt);
                }
                settled.push(s.receipt);
            } else {
                return Err(OutboxError::Corrupt);
            }
        }
        for a in attempts {
            let e = entries
                .get_mut(&a.request_id.to_ascii_lowercase())
                .ok_or(OutboxError::Corrupt)?;
            if e.queued.workspace_id != a.workspace_id || e.queued.request_id != a.request_id {
                return Err(OutboxError::Corrupt);
            }
            e.state = OutboxState::NeedsReceipt;
        }
        for r in settled {
            let e = entries
                .get_mut(&r.request_id.to_ascii_lowercase())
                .ok_or(OutboxError::Corrupt)?;
            if e.state != OutboxState::NeedsReceipt
                || e.queued.workspace_id != r.workspace_id
                || e.queued.request_id != r.request_id
                || !terminal(r.status)
                || !receipt_matches_contact(&r, &e.queued.mutation)
            {
                return Err(OutboxError::Corrupt);
            }
            e.state = OutboxState::Settled(r);
        }
        if entries.len() + archive_cleanup_pending.len() > active_limit {
            return Err(OutboxError::Full);
        }
        Ok(Self {
            root: root.to_path_buf(),
            _lock: lock,
            entries,
            archive_cleanup_pending,
            active_limit,
        })
    }
    pub fn entries(&self) -> Vec<OutboxEntry> {
        self.entries.values().cloned().collect()
    }
    pub fn enqueue(
        &mut self,
        workspace_id: &str,
        request_id: &str,
        mutation: LogbookMutation,
    ) -> Result<(), OutboxError> {
        if !bounded(workspace_id, 128) || !valid_request_id(request_id) {
            return Err(OutboxError::Invalid);
        }
        mutation.validate()?;
        let queued = QueuedQso {
            version: 1,
            request_id: request_id.into(),
            workspace_id: workspace_id.into(),
            mutation,
        };
        if let Some(e) = self.lookup(request_id)? {
            return if e.queued == queued {
                Ok(())
            } else {
                Err(OutboxError::Reused)
            };
        }
        // Incomplete archive cleanup still occupies up to three root files.
        // Do not grant new capacity until that operation's cleanup completes.
        if self.entries.len() + self.archive_cleanup_pending.len() >= self.active_limit {
            return Err(OutboxError::Full);
        }
        self.publish(&format!("{request_id}.queued.json"), &queued)?;
        self.entries.insert(
            request_id.to_ascii_lowercase(),
            OutboxEntry {
                queued,
                state: OutboxState::Pending,
            },
        );
        Ok(())
    }
    /// Returns an owned entry: archived terminal records are loaded only by exact ID.
    pub fn entry(&self, request_id: &str, workspace_id: &str) -> Result<OutboxEntry, OutboxError> {
        let e = self.lookup(request_id)?.ok_or(OutboxError::NotFound)?;
        if e.queued.workspace_id != workspace_id {
            return Err(OutboxError::WorkspaceChanged);
        }
        Ok(e)
    }
    fn lookup(&self, request_id: &str) -> Result<Option<OutboxEntry>, OutboxError> {
        if !valid_request_id(request_id) {
            return Err(OutboxError::Invalid);
        }
        // Archive wins even if cleanup was interrupted in this process.
        let retained = archive::read_archived(&self.root, request_id)?;
        let active = self.entries.get(&request_id.to_ascii_lowercase());
        if let (Some(retained), Some(active)) = (&retained, active) {
            if retained != active {
                return Err(OutboxError::Corrupt);
            }
        }
        let entry = retained.or_else(|| active.cloned());
        if entry
            .as_ref()
            .is_some_and(|e| e.queued.request_id != request_id)
        {
            return Err(OutboxError::Reused);
        }
        Ok(entry)
    }
    /// MUST succeed before the sender performs network IO. A persisted prior attempt
    /// requires receipt reconciliation; it is not permission to send again.
    pub fn mark_attempted(
        &mut self,
        request_id: &str,
        workspace_id: &str,
    ) -> Result<(), OutboxError> {
        let e = self.entry(request_id, workspace_id)?;
        if e.state != OutboxState::Pending {
            return Err(OutboxError::ReceiptRequired);
        }
        self.publish(
            &format!("{request_id}.attempted.json"),
            &Attempt {
                version: 1,
                request_id: request_id.into(),
                workspace_id: workspace_id.into(),
            },
        )?;
        self.entries
            .get_mut(&request_id.to_ascii_lowercase())
            .expect("checked entry")
            .state = OutboxState::NeedsReceipt;
        Ok(())
    }
    pub fn settle(&mut self, receipt: CommandReceipt) -> Result<(), OutboxError> {
        if !terminal(receipt.status)
            || !bounded(&receipt.server_instance, 128)
            || receipt.message.as_ref().is_some_and(|m| !bounded(m, 512))
        {
            return Err(OutboxError::Invalid);
        }
        let e = self.entry(&receipt.request_id, &receipt.workspace_id)?;
        if !receipt_matches_contact(&receipt, &e.queued.mutation) {
            return Err(OutboxError::Invalid);
        }
        if let OutboxState::Settled(existing) = &e.state {
            return if existing == &receipt {
                Ok(())
            } else {
                Err(OutboxError::Reused)
            };
        }
        if e.state != OutboxState::NeedsReceipt {
            return Err(OutboxError::ReceiptRequired);
        }
        self.publish(
            &format!("{}.settled.json", receipt.request_id),
            &Settled {
                version: 1,
                receipt: receipt.clone(),
            },
        )?;
        let request_id = receipt.request_id.clone();
        self.entries
            .get_mut(&request_id.to_ascii_lowercase())
            .expect("checked entry")
            .state = OutboxState::Settled(receipt);
        Ok(())
    }
    fn publish(&self, name: &str, value: &impl Serialize) -> Result<(), OutboxError> {
        publish_json(&self.root, name, value, 32768)
    }
}
fn publish_json(
    root: &Path,
    name: &str,
    value: &impl Serialize,
    max_bytes: usize,
) -> Result<(), OutboxError> {
    publish_json_with_hook(root, name, value, max_bytes, |_| Ok(()))
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum PublicationPhase {
    BeforePublish,
    BeforeDirectorySync,
}
fn publish_json_with_hook(
    root: &Path,
    name: &str,
    value: &impl Serialize,
    max_bytes: usize,
    mut hook: impl FnMut(PublicationPhase) -> Result<(), OutboxError>,
) -> Result<(), OutboxError> {
    let data = serde_json::to_vec(value).map_err(|_| OutboxError::Invalid)?;
    if data.len() > max_bytes {
        return Err(OutboxError::Invalid);
    }
    let final_path = root.join(name);
    if archive::checked_optional_file(&final_path)? {
        return Err(OutboxError::Corrupt);
    }
    // A prior partial write is never overwritten. It remains recoverable evidence.
    let mut temp = None;
    for suffix in 0..1024 {
        let path = root.join(format!("{name}.{suffix}.tmp"));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                temp = Some((path, file));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(OutboxError::Io),
        }
    }
    let (path, mut file) = temp.ok_or(OutboxError::Full)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|_| OutboxError::Io)?;
    }
    file.write_all(&data)
        .and_then(|_| file.sync_all())
        .map_err(|_| OutboxError::Io)?;
    drop(file);
    hook(PublicationPhase::BeforePublish)?;
    publish_file(&path, &final_path).map_err(|_| OutboxError::Io)?;
    hook(PublicationPhase::BeforeDirectorySync)?;
    #[cfg(unix)]
    {
        File::open(root)
            .and_then(|d| d.sync_all())
            .map_err(|_| OutboxError::Io)?;
    }
    Ok(())
}
#[cfg(not(windows))]
fn publish_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    // Atomic no-replace even across independent callers/processes. The file is
    // already synced and closed. No replacing rename or copy fallback is safe.
    // After link success an unlink/sync error means possibly published; callers
    // must not infer that destination is absent from an error result.
    fs::hard_link(source, destination)?;
    fs::remove_file(source)
}
#[cfg(windows)]
fn publish_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    // Same-directory publication, no replacement and no copy/delete fallback.
    // Request write-through rather than std::fs::rename's default Windows flags.
    // https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(source: *const u16, destination: *const u16, flags: u32) -> i32;
    }
    let wide = |path: &Path| -> std::io::Result<Vec<u16>> {
        let path = std::path::absolute(path)?;
        let value: Vec<u16> = path.as_os_str().encode_wide().collect();
        if value.contains(&0) {
            return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
        }
        Ok(value.into_iter().chain(Some(0)).collect())
    };
    let source = wide(source)?;
    let destination = wide(destination)?;
    if unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), 0x8) } == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn receipt_matches_contact(receipt: &CommandReceipt, mutation: &LogbookMutation) -> bool {
    bounded(&receipt.server_instance, 128)
        && receipt.message.as_ref().is_none_or(|m| bounded(m, 512))
        && (receipt.status == CommandStatus::Rejected
            || (receipt
                .contact_id
                .as_ref()
                .is_some_and(|id| id.eq_ignore_ascii_case(&mutation.qso().contact_id))
                && receipt
                    .contact_revision
                    .is_some_and(|revision| (1..=9_007_199_254_740_990).contains(&revision))))
}
fn terminal(status: CommandStatus) -> bool {
    matches!(
        status,
        CommandStatus::Applied | CommandStatus::Observed | CommandStatus::Rejected
    )
}
fn read_json<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<T, OutboxError> {
    let mut data = Vec::new();
    File::open(path)
        .map_err(|_| OutboxError::Io)?
        .take(32769)
        .read_to_end(&mut data)
        .map_err(|_| OutboxError::Io)?;
    if data.len() > 32768 {
        return Err(OutboxError::Corrupt);
    }
    serde_json::from_slice(&data).map_err(|_| OutboxError::Corrupt)
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "00000000-0000-4000-8000-000000000001";
    fn draft() -> LogbookMutation {
        LogbookMutation::Create {
            qso: QsoDraft {
                contact_id: ID.into(),
                callsign: "W1AW".into(),
                band: "20m".into(),
                mode: "FT8".into(),
                started_at_unix_ms: 1_700_000_000_000,
                frequency_hz: Some(14074000),
                rst_sent: "-10".into(),
                rst_received: "-12".into(),
                grid: "FN31".into(),
                comment: "synthetic".into(),
            },
        }
    }
    struct Fixture(PathBuf);
    #[test]
    fn correction_without_an_owner_revision_never_enters_durable_queue() {
        let f = Fixture::new();
        let mut queue = Outbox::open(&f.0).unwrap();
        let mutation = LogbookMutation::Update {
            qso: draft().qso().clone(),
            expected_contact_revision: 0,
        };
        assert_eq!(
            queue.enqueue("workspace", ID, mutation),
            Err(OutboxError::Invalid)
        );
        drop(queue);
        assert!(Outbox::open(&f.0).unwrap().entries().is_empty());
    }
    #[test]
    fn multiline_history_comments_survive_edit_queue_without_normalization() {
        let f = Fixture::new();
        let mut mutation = draft();
        let LogbookMutation::Create { ref mut qso } = mutation else {
            unreachable!()
        };
        qso.comment = "First line\r\nSecond\tcolumn".into();
        let mut queue = Outbox::open(&f.0).unwrap();
        queue.enqueue("workspace", ID, mutation.clone()).unwrap();
        drop(queue);
        let queue = Outbox::open(&f.0).unwrap();
        assert_eq!(queue.entries()[0].queued.mutation, mutation);
        let LogbookMutation::Create { ref mut qso } = mutation else {
            unreachable!()
        };
        qso.comment.push('\0');
        assert_eq!(qso.validate(), Err(OutboxError::Invalid));
    }
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "xnexus-outbox-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            for item in fs::read_dir(&self.0).unwrap() {
                let path = item.unwrap().path();
                assert_eq!(path.parent(), Some(self.0.as_path()));
                fs::remove_file(path).unwrap();
            }
            fs::remove_dir(&self.0).unwrap();
        }
    }
    fn receipt() -> CommandReceipt {
        CommandReceipt {
            request_id: ID.into(),
            workspace_id: "workspace".into(),
            server_instance: "server".into(),
            status: CommandStatus::Applied,
            message: None,
            contact_id: Some(ID.into()),
            contact_revision: Some(1),
        }
    }
    #[test]
    fn applied_receipt_requires_matching_contact_and_positive_revision() {
        let f = Fixture::new();
        let mut q = Outbox::open(&f.0).unwrap();
        q.enqueue("workspace", ID, draft()).unwrap();
        q.mark_attempted(ID, "workspace").unwrap();
        for (contact_id, contact_revision) in [
            (None, Some(1)),
            (Some("00000000-0000-4000-8000-000000000002".into()), Some(1)),
            (Some(ID.into()), None),
            (Some(ID.into()), Some(0)),
        ] {
            let mut r = receipt();
            r.contact_id = contact_id;
            r.contact_revision = contact_revision;
            assert_eq!(q.settle(r), Err(OutboxError::Invalid));
            assert_eq!(q.entries()[0].state, OutboxState::NeedsReceipt);
        }
    }
    #[test]
    fn settlement_io_failure_does_not_claim_no_send_or_discard_attempt() {
        let f = Fixture::new();
        let mut q = Outbox::open(&f.0).unwrap();
        q.enqueue("workspace", ID, draft()).unwrap();
        q.mark_attempted(ID, "workspace").unwrap();
        // Inject an unavailable publication directory after the owner has replied.
        let existing_root = q.root.clone();
        q.root = q.root.join("missing-test-directory");
        let error = q.settle(receipt()).unwrap_err();
        assert_eq!(error, OutboxError::Io);
        assert!(!error.to_string().contains("no contact was sent"));
        assert_eq!(q.entries()[0].state, OutboxState::NeedsReceipt);
        q.root = existing_root;
        drop(q);
        assert_eq!(
            Outbox::open(&f.0).unwrap().entries()[0].state,
            OutboxState::NeedsReceipt
        );
    }
    #[test]
    fn persisted_before_send_and_uncertainty_survives_restart() {
        let f = Fixture::new();
        {
            let mut q = Outbox::open(&f.0).unwrap();
            q.enqueue("workspace", ID, draft()).unwrap();
            q.mark_attempted(ID, "workspace").unwrap();
        }
        let mut q = Outbox::open(&f.0).unwrap();
        assert_eq!(q.entries()[0].state, OutboxState::NeedsReceipt);
        assert_eq!(
            q.mark_attempted(ID, "workspace"),
            Err(OutboxError::ReceiptRequired)
        );
    }
    #[test]
    fn queue_is_single_writer() {
        let f = Fixture::new();
        let _first = Outbox::open(&f.0).unwrap();
        assert!(matches!(Outbox::open(&f.0), Err(OutboxError::Locked)));
    }
    #[test]
    fn terminal_receipt_survives_restart_and_is_idempotent() {
        let f = Fixture::new();
        {
            let mut q = Outbox::open(&f.0).unwrap();
            q.enqueue("workspace", ID, draft()).unwrap();
            q.mark_attempted(ID, "workspace").unwrap();
            q.settle(receipt()).unwrap();
            q.settle(receipt()).unwrap();
        }
        let q = Outbox::open(&f.0).unwrap();
        assert_eq!(q.entries()[0].state, OutboxState::Settled(receipt()));
    }
    #[test]
    fn reused_id_and_foreign_workspace_are_refused() {
        let f = Fixture::new();
        let mut q = Outbox::open(&f.0).unwrap();
        q.enqueue("workspace", ID, draft()).unwrap();
        q.enqueue("workspace", ID, draft()).unwrap();
        assert_eq!(q.enqueue("other", ID, draft()), Err(OutboxError::Reused));
        assert_eq!(
            q.mark_attempted(ID, "other"),
            Err(OutboxError::WorkspaceChanged)
        );
    }
    #[test]
    fn partial_temp_is_not_a_committed_contact() {
        let f = Fixture::new();
        fs::write(f.0.join(format!("{ID}.queued.json.0.tmp")), "{").unwrap();
        let mut q = Outbox::open(&f.0).unwrap();
        assert!(q.entries().is_empty());
        q.enqueue("workspace", ID, draft()).unwrap();
        assert_eq!(q.entries().len(), 1);
    }
    #[test]
    fn corrupt_committed_record_is_preserved_and_refused() {
        let f = Fixture::new();
        let path = f.0.join(format!("{ID}.queued.json"));
        fs::write(&path, "{").unwrap();
        assert!(matches!(Outbox::open(&f.0), Err(OutboxError::Corrupt)));
        assert_eq!(fs::read_to_string(path).unwrap(), "{");
    }
    #[test]
    fn accepted_or_uncertain_is_not_a_saved_contact() {
        let f = Fixture::new();
        let mut q = Outbox::open(&f.0).unwrap();
        q.enqueue("workspace", ID, draft()).unwrap();
        q.mark_attempted(ID, "workspace").unwrap();
        for status in [CommandStatus::Accepted, CommandStatus::Uncertain] {
            let mut r = receipt();
            r.status = status;
            assert_eq!(q.settle(r), Err(OutboxError::Invalid));
        }
        assert_eq!(q.entries()[0].state, OutboxState::NeedsReceipt);
    }
    #[test]
    fn no_receipt_before_attempt_and_no_path_injection() {
        let f = Fixture::new();
        let mut q = Outbox::open(&f.0).unwrap();
        assert_eq!(
            q.enqueue("workspace", "../other", draft()),
            Err(OutboxError::Invalid)
        );
        q.enqueue("workspace", ID, draft()).unwrap();
        assert_eq!(q.settle(receipt()), Err(OutboxError::ReceiptRequired));
    }
}
