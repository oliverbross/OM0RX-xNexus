// SPDX-License-Identifier: GPL-3.0-only
//! Terminal retention is not deletion: a complete immutable archive record is
//! the permanent duplicate fence. Export is a separate copy, never authority.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveOutcome {
    pub entry: OutboxEntry,
    /// False means publication succeeded, but old journal cleanup must be retried.
    pub cleanup_complete: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Archived {
    version: u32,
    entry: OutboxEntry,
}

#[derive(Serialize)]
struct Export<'a> {
    version: u32,
    export_id: &'a str,
    workspace_id: &'a str,
    entries: Vec<OutboxEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ArchivePhase {
    BeforePublication,
    PublishedBeforeDirectorySync,
    BeforeArchiveSync,
    Published,
    RemovedQueued,
    RemovedAttempted,
    RemovedSettled,
}

impl Outbox {
    /// Explicit local maintenance only; never performs HTTP or archives uncertainty.
    /// A successful publication remains authoritative even if cleanup is interrupted.
    pub fn archive_terminal(
        &mut self,
        request_id: &str,
        workspace_id: &str,
    ) -> Result<ArchiveOutcome, OutboxError> {
        self.archive_with_hook(request_id, workspace_id, |_| Ok(()))
    }

    fn archive_with_hook(
        &mut self,
        request_id: &str,
        workspace_id: &str,
        mut hook: impl FnMut(ArchivePhase) -> Result<(), OutboxError>,
    ) -> Result<ArchiveOutcome, OutboxError> {
        let entry = self.entry(request_id, workspace_id)?;
        validate_terminal(&entry, request_id)?;
        hook(ArchivePhase::BeforePublication)?;
        if read_archived(&self.root, request_id)?.is_none() {
            let directory =
                archive_directory(&self.root, request_id, true)?.ok_or(OutboxError::Io)?;
            publish_json_with_hook(
                &directory,
                &format!("{}.json", request_id.to_ascii_lowercase()),
                &Archived {
                    version: 1,
                    entry: entry.clone(),
                },
                32768,
                |phase| {
                    if phase == PublicationPhase::BeforeDirectorySync {
                        hook(ArchivePhase::PublishedBeforeDirectorySync)
                    } else {
                        Ok(())
                    }
                },
            )?;
        }
        // A previous publication may have renamed successfully but reported a
        // sync failure. Re-establish durability before removing old journals,
        // including when retrying an already visible archive after restart.
        let directory = archive_directory(&self.root, request_id, false)?.ok_or(OutboxError::Io)?;
        let path = directory.join(format!("{}.json", request_id.to_ascii_lowercase()));
        checked_optional_file(&path)?;
        hook(ArchivePhase::BeforeArchiveSync)?;
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .and_then(|file| file.sync_all())
            .map_err(|_| OutboxError::Io)?;
        sync_directory(&directory)?;
        sync_directory(directory.parent().ok_or(OutboxError::Invalid)?)?;
        sync_directory(&self.root)?;
        // Commit point precedes every removal. Keep the full receipt even if a
        // later unlink fails; never report that an archive was not created.
        self.entries.remove(&request_id.to_ascii_lowercase());
        self.archive_cleanup_pending.insert(request_id.to_owned());
        let cleanup = (|| {
            hook(ArchivePhase::Published)?;
            for suffix in ["queued", "attempted", "settled"] {
                let name = format!("{request_id}.{suffix}.json");
                let path = self.root.join(&name);
                if checked_optional_file(&path)? {
                    validate_fragment(&path, &name, &entry)?;
                }
            }
            for (suffix, phase) in [
                ("queued", ArchivePhase::RemovedQueued),
                ("attempted", ArchivePhase::RemovedAttempted),
                ("settled", ArchivePhase::RemovedSettled),
            ] {
                let path = self.root.join(format!("{request_id}.{suffix}.json"));
                if checked_optional_file(&path)? {
                    fs::remove_file(path).map_err(|_| OutboxError::Io)?;
                }
                hook(phase)?;
            }
            sync_directory(&self.root)
        })();
        if cleanup.is_ok() {
            self.archive_cleanup_pending.remove(request_id);
        }
        Ok(ArchiveOutcome {
            entry,
            cleanup_complete: cleanup.is_ok(),
        })
    }

    /// Exact IDs whose archive committed but whose active fragments may remain.
    /// Retrying archive_terminal is local-only and preserves the same receipt.
    pub fn pending_archive_cleanup(&self) -> Vec<String> {
        self.archive_cleanup_pending.iter().cloned().collect()
    }

    /// Bounded local recovery projection, including after restart without pairing.
    /// Reads only charged cleanup IDs, never enumerates lifetime archives.
    pub fn pending_archive_cleanup_entries(&self) -> Result<Vec<OutboxEntry>, OutboxError> {
        self.archive_cleanup_pending
            .iter()
            .map(|id| read_archived(&self.root, id)?.ok_or(OutboxError::Corrupt))
            .collect()
    }

    /// A bounded, non-overwriting export in this private profile. The final JSON
    /// filename is published only after all requested terminal entries are read
    /// and its complete bytes are synced. Partial .tmp files are not exports.
    /// This API deliberately provides no import, replay or tombstone removal.
    pub fn export_terminal(
        &self,
        workspace_id: &str,
        export_id: &str,
        request_ids: &[String],
    ) -> Result<PathBuf, OutboxError> {
        self.export_with_hook(workspace_id, export_id, request_ids, |_| Ok(()))
    }
    fn export_with_hook(
        &self,
        workspace_id: &str,
        export_id: &str,
        request_ids: &[String],
        hook: impl FnMut(PublicationPhase) -> Result<(), OutboxError>,
    ) -> Result<PathBuf, OutboxError> {
        if !valid_request_id(export_id)
            || !bounded(workspace_id, 128)
            || request_ids.is_empty()
            || request_ids.len() > 32
        {
            return Err(OutboxError::Invalid);
        }
        let mut ids = BTreeSet::new();
        let mut entries = Vec::with_capacity(request_ids.len());
        for id in request_ids {
            if !valid_request_id(id) || !ids.insert(id.to_ascii_lowercase()) {
                return Err(OutboxError::Invalid);
            }
            let entry = self.entry(id, workspace_id)?;
            validate_terminal(&entry, id)?;
            entries.push(entry);
        }
        let directory = self.root.join("exports-v1");
        private_directory(&directory)?;
        let name = format!("{}.json", export_id.to_ascii_lowercase());
        publish_json_with_hook(
            &directory,
            &name,
            &Export {
                version: 1,
                export_id,
                workspace_id,
                entries,
            },
            1_048_576,
            hook,
        )?;
        Ok(directory.join(name))
    }
}

fn validate_terminal(entry: &OutboxEntry, request_id: &str) -> Result<(), OutboxError> {
    let OutboxState::Settled(receipt) = &entry.state else {
        return Err(OutboxError::ReceiptRequired);
    };
    if entry.queued.version != 1
        || !valid_request_id(request_id)
        || entry.queued.request_id != request_id
        || receipt.request_id != request_id
        || !bounded(&entry.queued.workspace_id, 128)
        || receipt.workspace_id != entry.queued.workspace_id
        || entry.queued.mutation.validate().is_err()
        || !terminal(receipt.status)
        || !receipt_matches_contact(receipt, &entry.queued.mutation)
    {
        return Err(OutboxError::Corrupt);
    }
    Ok(())
}

pub(super) fn read_archived(root: &Path, id: &str) -> Result<Option<OutboxEntry>, OutboxError> {
    let Some(directory) = archive_directory(root, id, false)? else {
        return Ok(None);
    };
    let path = directory.join(format!("{}.json", id.to_ascii_lowercase()));
    if !checked_optional_file(&path)? {
        return Ok(None);
    }
    let record: Archived = read_json(&path)?;
    if record.version != 1 || !record.entry.queued.request_id.eq_ignore_ascii_case(id) {
        return Err(OutboxError::Corrupt);
    }
    validate_terminal(&record.entry, &record.entry.queued.request_id)
        .map_err(|_| OutboxError::Corrupt)?;
    Ok(Some(record.entry))
}

fn archive_directory(root: &Path, id: &str, create: bool) -> Result<Option<PathBuf>, OutboxError> {
    // Validate before slicing or constructing any path. Case aliases share one
    // tombstone on case-sensitive and case-insensitive filesystems alike.
    if !valid_request_id(id) {
        return Err(OutboxError::Invalid);
    }
    let base = root.join("archive-v1");
    let shard = base.join(&id.to_ascii_lowercase()[..2]);
    for directory in [&base, &shard] {
        if create {
            private_directory(directory)?;
        } else {
            match fs::symlink_metadata(directory) {
                Ok(_) => checked_directory(directory)?,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(_) => return Err(OutboxError::Io),
            }
        }
    }
    Ok(Some(shard))
}

pub(super) fn journal_id(name: &str) -> Option<&str> {
    [".queued.json", ".attempted.json", ".settled.json"]
        .iter()
        .find_map(|suffix| name.strip_suffix(suffix))
}

pub(super) fn validate_fragment(
    path: &Path,
    name: &str,
    entry: &OutboxEntry,
) -> Result<(), OutboxError> {
    let id = journal_id(name).ok_or(OutboxError::Corrupt)?;
    if id != entry.queued.request_id {
        return Err(OutboxError::Corrupt);
    }
    let correct = if name.ends_with(".queued.json") {
        read_json::<QueuedQso>(path)? == entry.queued
    } else if name.ends_with(".attempted.json") {
        let value: Attempt = read_json(path)?;
        value.version == 1
            && value.request_id == id
            && value.workspace_id == entry.queued.workspace_id
    } else {
        let value: Settled = read_json(path)?;
        value.version == 1 && entry.state == OutboxState::Settled(value.receipt)
    };
    if !correct {
        return Err(OutboxError::Corrupt);
    }
    Ok(())
}

fn reparse(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return true;
        }
    }
    metadata.file_type().is_symlink()
}
pub(super) fn checked_directory(path: &Path) -> Result<(), OutboxError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| OutboxError::Io)?;
    if reparse(&metadata) || !metadata.is_dir() {
        return Err(OutboxError::Corrupt);
    }
    Ok(())
}
pub(super) fn checked_optional_file(path: &Path) -> Result<bool, OutboxError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !reparse(&metadata) && metadata.is_file() => Ok(true),
        Ok(_) => Err(OutboxError::Corrupt),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(OutboxError::Io),
    }
}
fn private_directory(path: &Path) -> Result<(), OutboxError> {
    match fs::create_dir(path) {
        Ok(()) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                    .map_err(|_| OutboxError::Io)?;
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err(OutboxError::Io),
    }
    checked_directory(path)?;
    // Also cover a retry after create succeeded but parent sync failed.
    sync_directory(path.parent().ok_or(OutboxError::Invalid)?)
}
fn sync_directory(path: &Path) -> Result<(), OutboxError> {
    #[cfg(unix)]
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|_| OutboxError::Io)?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests;
