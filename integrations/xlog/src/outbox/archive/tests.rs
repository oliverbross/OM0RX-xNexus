use super::*;
use serde_json::json;

const ID: &str = "00000000-0000-4000-8000-000000000001";
const EXPORT: &str = "a0000000-0000-4000-8000-000000000002";
const WORKSPACE: &str = "synthetic-workspace";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "xnexus-archive-{}-{}-{}",
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
        // Only this newly created direct child is owned. Rust remove_dir_all
        // removes directory links themselves rather than traversing targets.
        assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
        assert!(self
            .0
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("xnexus-archive-"));
        // Preserve evidence rather than delete if the owned root was replaced
        // by a link/reparse point or no longer resolves under the temp root.
        let metadata = fs::symlink_metadata(&self.0).unwrap();
        assert!(metadata.is_dir() && !reparse(&metadata));
        let resolved = self.0.canonicalize().unwrap();
        let temp_root = std::env::temp_dir().canonicalize().unwrap();
        assert!(resolved.is_absolute());
        assert_eq!(resolved.parent(), Some(temp_root.as_path()));
        fs::remove_dir_all(&resolved).unwrap();
    }
}
fn draft(id: &str) -> LogbookMutation {
    LogbookMutation::Create {
        qso: QsoDraft {
            contact_id: id.into(),
            callsign: "W1AW".into(),
            band: "20m".into(),
            mode: "FT8".into(),
            started_at_unix_ms: 1_700_000_000_000,
            frequency_hz: Some(14_074_000),
            rst_sent: "-10".into(),
            rst_received: "-12".into(),
            grid: "FN31".into(),
            comment: "Synthetic archive fixture\r\nExact text".into(),
        },
    }
}
fn receipt() -> CommandReceipt {
    CommandReceipt {
        request_id: ID.into(),
        server_instance: "synthetic-server".into(),
        workspace_id: WORKSPACE.into(),
        status: CommandStatus::Applied,
        message: None,
        contact_id: Some(ID.into()),
        contact_revision: Some(7),
    }
}
fn settled(f: &Fixture) -> Outbox {
    let mut queue = Outbox::open(&f.0).unwrap();
    queue.enqueue(WORKSPACE, ID, draft(ID)).unwrap();
    queue.mark_attempted(ID, WORKSPACE).unwrap();
    queue.settle(receipt()).unwrap();
    queue
}

#[test]
fn cleanup_projection_survives_restart_and_refuses_corruption_without_enumerating_archives() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    let retained = queue.entries()[0].clone();
    let result = queue
        .archive_with_hook(ID, WORKSPACE, |phase| {
            if phase == ArchivePhase::Published {
                Err(OutboxError::Io)
            } else {
                Ok(())
            }
        })
        .unwrap();
    assert!(!result.cleanup_complete);
    assert_eq!(
        queue.pending_archive_cleanup_entries().unwrap(),
        vec![retained.clone()]
    );
    drop(queue);
    let queue = Outbox::open(&f.0).unwrap();
    assert!(queue.entries().is_empty());
    assert_eq!(
        queue.pending_archive_cleanup_entries().unwrap(),
        vec![retained]
    );
    // Corruption discovered after opening must remain visible as an error,
    // not disappear from a UI recovery projection.
    let path = archive_directory(&f.0, ID, false)
        .unwrap()
        .unwrap()
        .join(format!("{ID}.json"));
    fs::write(path, b"{}").unwrap();
    assert_eq!(
        queue.pending_archive_cleanup_entries(),
        Err(OutboxError::Corrupt)
    );
}

#[test]
fn cleanup_projection_missing_archive_is_an_error_and_does_not_free_capacity() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    queue
        .archive_with_hook(ID, WORKSPACE, |phase| {
            if phase == ArchivePhase::Published {
                Err(OutboxError::Io)
            } else {
                Ok(())
            }
        })
        .unwrap();
    drop(queue);
    let mut queue = Outbox::open_with_limit(&f.0, 1).unwrap();
    let retained = queue.pending_archive_cleanup_entries().unwrap();
    assert_eq!(retained.len(), 1);
    let path = archive_directory(&f.0, ID, false)
        .unwrap()
        .unwrap()
        .join(format!("{ID}.json"));
    let original = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert_eq!(
        queue.pending_archive_cleanup_entries(),
        Err(OutboxError::Corrupt)
    );
    assert_eq!(queue.pending_archive_cleanup(), vec![ID.to_owned()]);
    let other = "00000000-0000-4000-8000-000000000003";
    assert_eq!(
        queue.enqueue(WORKSPACE, other, draft(other)),
        Err(OutboxError::Full)
    );
    // Restore the exact fixture bytes, not a synthesized success receipt.
    fs::write(&path, original).unwrap();
    assert_eq!(queue.pending_archive_cleanup_entries().unwrap(), retained);
    assert!(
        queue
            .archive_terminal(ID, WORKSPACE)
            .unwrap()
            .cleanup_complete
    );
    queue.enqueue(WORKSPACE, other, draft(other)).unwrap();
}

#[test]
fn archived_update_preserves_expected_revision_observed_receipt_and_no_network_recovery() {
    let f = Fixture::new();
    let LogbookMutation::Create { qso } = draft(ID) else {
        unreachable!()
    };
    let mutation = LogbookMutation::Update {
        qso,
        expected_contact_revision: 6,
    };
    let mut queue = Outbox::open(&f.0).unwrap();
    queue.enqueue(WORKSPACE, ID, mutation.clone()).unwrap();
    queue.mark_attempted(ID, WORKSPACE).unwrap();
    let mut observed = receipt();
    observed.status = CommandStatus::Observed;
    observed.message = Some("Synthetic owner receipt; exact revision retained".into());
    queue.settle(observed.clone()).unwrap();
    let original = queue.entry(ID, WORKSPACE).unwrap();
    queue.archive_terminal(ID, WORKSPACE).unwrap();
    drop(queue);
    let mut queue = Outbox::open(&f.0).unwrap();
    assert_eq!(queue.entry(ID, WORKSPACE).unwrap(), original);
    queue.enqueue(WORKSPACE, ID, mutation.clone()).unwrap();
    let mut changed = mutation;
    let LogbookMutation::Update {
        expected_contact_revision,
        ..
    } = &mut changed
    else {
        unreachable!()
    };
    *expected_contact_revision = 7;
    assert_eq!(
        queue.enqueue(WORKSPACE, ID, changed),
        Err(OutboxError::Reused)
    );
    assert_eq!(
        queue.entry(ID, "foreign"),
        Err(OutboxError::WorkspaceChanged)
    );
    let path = queue
        .export_terminal(WORKSPACE, EXPORT, &[ID.into()])
        .unwrap();
    let exported: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    assert_eq!(exported["entries"][0], json!(original));
    let (mut client, server) =
        crate::tests::server(vec![crate::tests::json_response(crate::tests::hello())]);
    client.connect().unwrap();
    assert_eq!(server.join().unwrap().len(), 1);
    assert_eq!(queue.submit(&mut client, ID).unwrap(), observed);
    assert_eq!(queue.reconcile(&mut client, ID).unwrap(), observed);
    assert_eq!(queue.retry_missing(&mut client, ID).unwrap(), observed);
    assert!(queue.entries().is_empty());
}

#[test]
fn archived_record_read_accepts_exact_byte_limit_and_refuses_one_extra_byte() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    let expected = queue.entry(ID, WORKSPACE).unwrap();
    queue.archive_terminal(ID, WORKSPACE).unwrap();
    let path = archive_directory(&f.0, ID, false)
        .unwrap()
        .unwrap()
        .join(format!("{ID}.json"));
    // JSON trailing whitespace is valid and does not alter any retained fact.
    let mut bytes = fs::read(&path).unwrap();
    assert!(bytes.len() < 32768);
    bytes.resize(32768, b' ');
    fs::write(&path, &bytes).unwrap();
    assert_eq!(queue.entry(ID, WORKSPACE).unwrap(), expected);
    bytes.push(b' ');
    fs::write(&path, &bytes).unwrap();
    assert_eq!(queue.entry(ID, WORKSPACE), Err(OutboxError::Corrupt));
    assert_eq!(fs::metadata(&path).unwrap().len(), 32769);
}

#[test]
fn publication_primitive_enforces_exact_record_and_export_byte_limits_before_writing() {
    // This directly exercises the shared publication primitive, not a claim
    // that valid bounded QSO fields can fill a one-MiB export with 32 entries.
    for limit in [32768, 1_048_576] {
        let f = Fixture::new();
        let exact = "x".repeat(limit - 2); // Serialized quotes occupy two bytes.
        assert_eq!(serde_json::to_vec(&exact).unwrap().len(), limit);
        publish_json(&f.0, "exact.json", &exact, limit).unwrap();
        assert_eq!(
            fs::metadata(f.0.join("exact.json")).unwrap().len(),
            limit as u64
        );
        let excessive = format!("{exact}x");
        assert_eq!(
            publish_json(&f.0, "excessive.json", &excessive, limit),
            Err(OutboxError::Invalid)
        );
        let names: Vec<_> = fs::read_dir(&f.0)
            .unwrap()
            .map(|v| v.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("exact.json")]);
        assert_eq!(
            fs::read(f.0.join("exact.json")).unwrap(),
            serde_json::to_vec(&exact).unwrap()
        );
    }
}

#[test]
fn interruption_at_every_archive_boundary_preserves_terminal_identity() {
    for phase in [
        ArchivePhase::BeforePublication,
        ArchivePhase::Published,
        ArchivePhase::RemovedQueued,
        ArchivePhase::RemovedAttempted,
        ArchivePhase::RemovedSettled,
    ] {
        let f = Fixture::new();
        let mut queue = settled(&f);
        let result = queue.archive_with_hook(ID, WORKSPACE, |observed| {
            if observed == phase {
                Err(OutboxError::Io)
            } else {
                Ok(())
            }
        });
        if phase == ArchivePhase::BeforePublication {
            assert_eq!(result, Err(OutboxError::Io));
        } else {
            assert!(!result.unwrap().cleanup_complete);
        }
        drop(queue);
        let mut queue = Outbox::open(&f.0).unwrap();
        assert_eq!(
            queue.entry(ID, WORKSPACE).unwrap().state,
            OutboxState::Settled(receipt())
        );
        assert_eq!(
            queue.enqueue("other", ID, draft(ID)),
            Err(OutboxError::Reused)
        );
        assert_eq!(
            queue.mark_attempted(ID, WORKSPACE),
            Err(OutboxError::ReceiptRequired)
        );
        assert!(
            queue
                .archive_terminal(ID, WORKSPACE)
                .unwrap()
                .cleanup_complete
        );
        assert!(queue.entries().is_empty());
        assert!(queue.pending_archive_cleanup().is_empty());
        drop(queue);
        let queue = Outbox::open(&f.0).unwrap();
        assert!(queue.entries().is_empty());
        assert_eq!(
            queue.entry(ID, WORKSPACE).unwrap().queued.mutation,
            draft(ID)
        );
    }
}

#[test]
fn unresolved_entries_cannot_be_archived_or_exported() {
    let f = Fixture::new();
    let mut queue = Outbox::open(&f.0).unwrap();
    queue.enqueue(WORKSPACE, ID, draft(ID)).unwrap();
    for attempted in [false, true] {
        if attempted {
            queue.mark_attempted(ID, WORKSPACE).unwrap();
        }
        assert_eq!(
            queue.archive_terminal(ID, WORKSPACE),
            Err(OutboxError::ReceiptRequired)
        );
        assert_eq!(
            queue.export_terminal(WORKSPACE, EXPORT, &[ID.into()]),
            Err(OutboxError::ReceiptRequired)
        );
    }
    assert!(!f.0.join("archive-v1").exists());
    drop(queue);
    assert_eq!(
        Outbox::open(&f.0).unwrap().entries()[0].state,
        OutboxState::NeedsReceipt
    );
}

#[test]
fn archived_submit_and_reconcile_return_receipt_with_retired_http_server() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    queue.archive_terminal(ID, WORKSPACE).unwrap();
    drop(queue);
    let mut queue = Outbox::open(&f.0).unwrap();
    let (mut client, server) =
        crate::tests::server(vec![crate::tests::json_response(crate::tests::hello())]);
    client.connect().unwrap();
    assert_eq!(server.join().unwrap().len(), 1); // Listener is now gone: any network call fails.
    assert_eq!(queue.submit(&mut client, ID).unwrap(), receipt());
    assert_eq!(queue.reconcile(&mut client, ID).unwrap(), receipt());
    assert_eq!(queue.retry_missing(&mut client, ID).unwrap(), receipt());
    client.hello.as_mut().unwrap().workspace_id = "other".into();
    assert_eq!(
        queue.submit(&mut client, ID),
        Err(DeliveryError::Queue(OutboxError::WorkspaceChanged))
    );
    assert_eq!(
        queue.reconcile(&mut client, ID),
        Err(DeliveryError::Queue(OutboxError::WorkspaceChanged))
    );
}

#[test]
fn rejected_receipt_is_retained_without_assuming_owner_database_receipt() {
    let f = Fixture::new();
    let mut queue = Outbox::open(&f.0).unwrap();
    queue.enqueue(WORKSPACE, ID, draft(ID)).unwrap();
    queue.mark_attempted(ID, WORKSPACE).unwrap();
    let mut rejected = receipt();
    rejected.status = CommandStatus::Rejected;
    rejected.contact_id = None;
    rejected.contact_revision = None;
    queue.settle(rejected.clone()).unwrap();
    queue.archive_terminal(ID, WORKSPACE).unwrap();
    drop(queue);
    let mut queue = Outbox::open(&f.0).unwrap();
    queue.enqueue(WORKSPACE, ID, draft(ID)).unwrap();
    assert!(queue.entries().is_empty());
    assert_eq!(
        queue.entry(ID, WORKSPACE).unwrap().state,
        OutboxState::Settled(rejected)
    );
    let mut changed = draft(ID);
    let LogbookMutation::Create { qso } = &mut changed else {
        unreachable!()
    };
    qso.comment.push('!');
    assert_eq!(
        queue.enqueue(WORKSPACE, ID, changed),
        Err(OutboxError::Reused)
    );
}

#[test]
fn corrupt_archive_and_conflicting_orphan_fragments_fail_closed() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    queue
        .archive_with_hook(ID, WORKSPACE, |phase| {
            if phase == ArchivePhase::RemovedQueued {
                Err(OutboxError::Io)
            } else {
                Ok(())
            }
        })
        .unwrap();
    drop(queue);
    // No queued record remains, but the complete archive prevents a pending resurrection.
    assert!(Outbox::open(&f.0).unwrap().entries().is_empty());
    let attempt = f.0.join(format!("{ID}.attempted.json"));
    fs::write(
        &attempt,
        serde_json::to_vec(&Attempt {
            version: 1,
            request_id: ID.into(),
            workspace_id: "foreign".into(),
        })
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(Outbox::open(&f.0), Err(OutboxError::Corrupt)));
    fs::remove_file(attempt).unwrap();
    let archived = archive_directory(&f.0, ID, false)
        .unwrap()
        .unwrap()
        .join(format!("{ID}.json"));
    fs::write(&archived, b"{").unwrap();
    assert!(matches!(Outbox::open(&f.0), Err(OutboxError::Corrupt)));
    assert_eq!(fs::read(&archived).unwrap(), b"{");
}

#[test]
fn export_is_private_bounded_complete_and_never_overwrites_or_imports() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    queue.archive_terminal(ID, WORKSPACE).unwrap();
    assert_eq!(
        queue.export_terminal("other", EXPORT, &[ID.into()]),
        Err(OutboxError::WorkspaceChanged)
    );
    assert_eq!(
        queue.export_terminal(WORKSPACE, EXPORT, &vec![ID.into(); 33]),
        Err(OutboxError::Invalid)
    );
    let directory = f.0.join("exports-v1");
    private_directory(&directory).unwrap();
    let partial = directory.join(format!("{EXPORT}.json.0.tmp"));
    fs::write(&partial, b"{").unwrap();
    let path = queue
        .export_terminal(WORKSPACE, EXPORT, &[ID.into()])
        .unwrap();
    let bytes = fs::read(&path).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let exported: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        exported["entries"][0],
        json!(queue.entry(ID, WORKSPACE).unwrap())
    );
    assert_eq!(fs::read(&partial).unwrap(), b"{");
    assert!(queue
        .export_terminal(WORKSPACE, EXPORT, &[ID.into()])
        .is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    fs::remove_file(path).unwrap(); // An exported copy has no authority over the tombstone.
    drop(queue);
    assert_eq!(
        Outbox::open(&f.0)
            .unwrap()
            .entry(ID, WORKSPACE)
            .unwrap()
            .state,
        OutboxState::Settled(receipt())
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}

#[test]
fn old_archives_are_loaded_only_by_id_and_uuid_paths_are_validated() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    queue.archive_terminal(ID, WORKSPACE).unwrap();
    let path = archive_directory(&f.0, ID, false)
        .unwrap()
        .unwrap()
        .join(format!("{ID}.json"));
    fs::write(path, b"{").unwrap();
    drop(queue);
    // Startup does not scan a lifetime-sized archive. Exact access still fails closed.
    let mut queue = Outbox::open(&f.0).unwrap();
    assert!(queue.entries().is_empty());
    assert_eq!(queue.entry(ID, WORKSPACE), Err(OutboxError::Corrupt));
    assert_eq!(
        queue.archive_terminal("../escape", WORKSPACE),
        Err(OutboxError::Invalid)
    );
    assert_eq!(
        queue.export_terminal(WORKSPACE, "../escape", &[ID.into()]),
        Err(OutboxError::Invalid)
    );
    assert_eq!(
        queue.enqueue(WORKSPACE, ID, draft(ID)),
        Err(OutboxError::Corrupt)
    );
}

#[test]
fn archiving_frees_active_limit_and_restart_keeps_duplicate_fence() {
    let f = Fixture::new();
    let production = Outbox::open(&f.0).unwrap();
    assert_eq!(production.active_limit, 10_000);
    drop(production);
    // Populate valid on-disk pending journals directly; all production reads,
    // admission, archive publication, and restart use the same implementation
    // with a smaller private cap, avoiding a heavy 10,000-file test fixture.
    for n in 1..=3 {
        let id = format!("00000000-0000-4000-8000-{n:012x}");
        let queued = QueuedQso {
            version: 1,
            request_id: id.clone(),
            workspace_id: WORKSPACE.into(),
            mutation: draft(&id),
        };
        fs::write(
            f.0.join(format!("{id}.queued.json")),
            serde_json::to_vec(&queued).unwrap(),
        )
        .unwrap();
    }
    let extra = "00000000-0000-4000-8000-00000000ffff";
    let mut queue = Outbox::open_with_limit(&f.0, 3).unwrap();
    assert_eq!(queue.entries.len(), 3);
    assert_eq!(
        queue.enqueue(WORKSPACE, extra, draft(extra)),
        Err(OutboxError::Full)
    );
    queue.mark_attempted(ID, WORKSPACE).unwrap();
    queue.settle(receipt()).unwrap();
    assert!(
        queue
            .archive_terminal(ID, WORKSPACE)
            .unwrap()
            .cleanup_complete
    );
    queue.enqueue(WORKSPACE, extra, draft(extra)).unwrap();
    drop(queue);
    let mut queue = Outbox::open_with_limit(&f.0, 3).unwrap();
    assert_eq!(queue.entries.len(), 3);
    queue.enqueue(WORKSPACE, ID, draft(ID)).unwrap();
    assert_eq!(queue.entries.len(), 3);
    assert_eq!(
        queue.entry(ID, WORKSPACE).unwrap().state,
        OutboxState::Settled(receipt())
    );
    assert!(matches!(Outbox::open(&f.0), Err(OutboxError::Locked)));
}

#[test]
fn case_aliases_cannot_bypass_retained_operation_identity() {
    let f = Fixture::new();
    let mut queue = Outbox::open(&f.0).unwrap();
    let lower = "abcdef00-0000-4000-8000-000000000001";
    queue.enqueue(WORKSPACE, lower, draft(lower)).unwrap();
    assert_eq!(
        queue.enqueue(WORKSPACE, &lower.to_ascii_uppercase(), draft(lower)),
        Err(OutboxError::Reused)
    );
    queue.mark_attempted(lower, WORKSPACE).unwrap();
    let mut r = receipt();
    r.request_id = lower.into();
    r.contact_id = Some(lower.into());
    queue.settle(r).unwrap();
    queue.archive_terminal(lower, WORKSPACE).unwrap();
    drop(queue);
    let mut queue = Outbox::open(&f.0).unwrap();
    assert_eq!(
        queue.enqueue(WORKSPACE, &lower.to_ascii_uppercase(), draft(lower)),
        Err(OutboxError::Reused)
    );
}

#[test]
fn publication_failure_keeps_original_journals_and_no_cleanup_claim() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    fs::write(f.0.join("archive-v1"), b"occupied").unwrap();
    assert_eq!(
        queue.archive_terminal(ID, WORKSPACE),
        Err(OutboxError::Corrupt)
    );
    assert_eq!(queue.entries().len(), 1);
    for suffix in ["queued", "attempted", "settled"] {
        assert!(f.0.join(format!("{ID}.{suffix}.json")).is_file());
    }
    assert!(queue.pending_archive_cleanup().is_empty());
    assert_eq!(fs::read(f.0.join("archive-v1")).unwrap(), b"occupied");
}

#[test]
fn visible_archive_after_publication_sync_failure_is_resynced_before_cleanup() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    assert_eq!(
        queue.archive_with_hook(ID, WORKSPACE, |phase| {
            if phase == ArchivePhase::PublishedBeforeDirectorySync {
                Err(OutboxError::Io)
            } else {
                Ok(())
            }
        }),
        Err(OutboxError::Io)
    );
    assert!(read_archived(&f.0, ID).unwrap().is_some());
    for suffix in ["queued", "attempted", "settled"] {
        assert!(f.0.join(format!("{ID}.{suffix}.json")).is_file());
    }
    drop(queue);
    let mut queue = Outbox::open(&f.0).unwrap();
    let mut barrier_seen = false;
    assert_eq!(
        queue.archive_with_hook(ID, WORKSPACE, |phase| {
            assert_ne!(
                phase,
                ArchivePhase::PublishedBeforeDirectorySync,
                "existing archive must not be republished"
            );
            if phase == ArchivePhase::BeforeArchiveSync {
                barrier_seen = true;
                Err(OutboxError::Io)
            } else {
                Ok(())
            }
        }),
        Err(OutboxError::Io)
    );
    assert!(barrier_seen);
    for suffix in ["queued", "attempted", "settled"] {
        assert!(f.0.join(format!("{ID}.{suffix}.json")).is_file());
    }
    assert!(
        queue
            .archive_terminal(ID, WORKSPACE)
            .unwrap()
            .cleanup_complete
    );
}

#[test]
fn incomplete_cleanup_keeps_capacity_charged_and_restart_recovery_available() {
    let f = Fixture::new();
    let mut queue = settled(&f);
    queue.active_limit = 1;
    let extra = "00000000-0000-4000-8000-000000000002";
    for _ in 0..3 {
        let outcome = queue
            .archive_with_hook(ID, WORKSPACE, |phase| {
                if phase == ArchivePhase::Published {
                    Err(OutboxError::Io)
                } else {
                    Ok(())
                }
            })
            .unwrap();
        assert!(!outcome.cleanup_complete);
        assert!(queue.entries().is_empty());
        assert_eq!(
            queue.enqueue(WORKSPACE, extra, draft(extra)),
            Err(OutboxError::Full)
        );
    }
    drop(queue);
    let mut queue = Outbox::open_with_limit(&f.0, 1).unwrap();
    assert_eq!(queue.pending_archive_cleanup(), vec![ID.to_owned()]);
    assert_eq!(
        queue.enqueue(WORKSPACE, extra, draft(extra)),
        Err(OutboxError::Full)
    );
    assert!(
        queue
            .archive_terminal(ID, WORKSPACE)
            .unwrap()
            .cleanup_complete
    );
    queue.enqueue(WORKSPACE, extra, draft(extra)).unwrap();
    drop(queue);
    let queue = Outbox::open_with_limit(&f.0, 1).unwrap();
    assert_eq!(queue.entries().len(), 1);
    assert_eq!(
        queue.entry(ID, WORKSPACE).unwrap().state,
        OutboxState::Settled(receipt())
    );
}

#[test]
fn simultaneous_same_export_id_never_replaces_the_winning_selection() {
    use std::sync::{Arc, Barrier};
    let f = Fixture::new();
    let mut queue = settled(&f);
    let second = "00000000-0000-4000-8000-000000000002";
    queue.enqueue(WORKSPACE, second, draft(second)).unwrap();
    queue.mark_attempted(second, WORKSPACE).unwrap();
    let mut r = receipt();
    r.request_id = second.into();
    r.contact_id = Some(second.into());
    queue.settle(r).unwrap();
    let queue = Arc::new(queue);
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = [ID, second]
        .into_iter()
        .map(|id| {
            let queue = Arc::clone(&queue);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                (
                    id,
                    queue.export_with_hook(WORKSPACE, EXPORT, &[id.into()], |phase| {
                        if phase == PublicationPhase::BeforePublish {
                            barrier.wait();
                        }
                        Ok(())
                    }),
                )
            })
        })
        .collect();
    let outcomes: Vec<_> = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(
        outcomes.iter().filter(|(_, result)| result.is_ok()).count(),
        1
    );
    let (winner, result) = outcomes.iter().find(|(_, result)| result.is_ok()).unwrap();
    let exported: serde_json::Value =
        serde_json::from_slice(&fs::read(result.as_ref().unwrap()).unwrap()).unwrap();
    assert_eq!(exported["entries"][0]["queued"]["request_id"], *winner);
}

#[test]
fn external_destination_collision_after_precheck_is_preserved() {
    let f = Fixture::new();
    let queue = settled(&f);
    let destination = f.0.join("exports-v1").join(format!("{EXPORT}.json"));
    let result = queue.export_with_hook(WORKSPACE, EXPORT, &[ID.into()], |phase| {
        if phase == PublicationPhase::BeforePublish {
            fs::write(&destination, b"existing external selection").unwrap();
        }
        Ok(())
    });
    assert!(result.is_err());
    assert_eq!(
        fs::read(destination).unwrap(),
        b"existing external selection"
    );
}

#[cfg(unix)]
#[test]
fn archive_and_export_directory_links_are_refused_without_touching_target() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let other = Fixture::new();
    let mut queue = settled(&f);
    symlink(&other.0, f.0.join("archive-v1")).unwrap();
    assert_eq!(
        queue.archive_terminal(ID, WORKSPACE),
        Err(OutboxError::Corrupt)
    );
    assert!(fs::read_dir(&other.0).unwrap().next().is_none());
    fs::remove_file(f.0.join("archive-v1")).unwrap();
    queue.archive_terminal(ID, WORKSPACE).unwrap();
    symlink(&other.0, f.0.join("exports-v1")).unwrap();
    assert_eq!(
        queue.export_terminal(WORKSPACE, EXPORT, &[ID.into()]),
        Err(OutboxError::Corrupt)
    );
    assert!(fs::read_dir(&other.0).unwrap().next().is_none());
}

#[cfg(windows)]
struct OwnedReparse {
    root: PathBuf,
    path: PathBuf,
    directory: bool,
}

#[cfg(windows)]
impl Drop for OwnedReparse {
    fn drop(&mut self) {
        // Delete the link itself, never recursively traverse its destination.
        assert!(self.path.starts_with(&self.root));
        assert_ne!(self.path, self.root);
        assert!(self
            .path
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap()
            .starts_with(self.root.canonicalize().unwrap()));
        assert!(reparse(&fs::symlink_metadata(&self.path).unwrap()));
        if self.directory {
            fs::remove_dir(&self.path).unwrap();
        } else {
            fs::remove_file(&self.path).unwrap();
        }
    }
}

#[cfg(windows)]
fn owned_junction(owner: &Fixture, name: &str, target: &Fixture) -> OwnedReparse {
    use std::os::windows::{ffi::OsStrExt, fs::OpenOptionsExt, io::AsRawHandle};
    #[link(name = "kernel32")]
    extern "system" {
        fn DeviceIoControl(
            handle: *mut std::ffi::c_void,
            control: u32,
            input: *const std::ffi::c_void,
            input_bytes: u32,
            output: *mut std::ffi::c_void,
            output_bytes: u32,
            returned: *mut u32,
            overlapped: *mut std::ffi::c_void,
        ) -> i32;
    }
    let path = owner.0.join(name);
    assert!(path.starts_with(&owner.0));
    assert!(path
        .parent()
        .unwrap()
        .canonicalize()
        .unwrap()
        .starts_with(owner.0.canonicalize().unwrap()));
    checked_directory(&target.0).unwrap();
    let target_path = target.0.canonicalize().unwrap();
    let display = target_path.to_str().unwrap().strip_prefix(r"\\?\").unwrap();
    assert!(
        display.as_bytes().get(1) == Some(&b':'),
        "test target must be a local absolute drive path"
    );
    let substitute: Vec<u16> = std::ffi::OsStr::new(&format!(r"\??\{display}"))
        .encode_wide()
        .collect();
    let print: Vec<u16> = std::ffi::OsStr::new(display).encode_wide().collect();
    let substitute_bytes = u16::try_from(substitute.len() * 2).unwrap();
    let print_bytes = u16::try_from(print.len() * 2).unwrap();
    let print_offset = substitute_bytes.checked_add(2).unwrap();
    let data_bytes = 8u16
        .checked_add(print_offset)
        .unwrap()
        .checked_add(print_bytes)
        .unwrap()
        .checked_add(2)
        .unwrap();
    let mut buffer = Vec::new();
    buffer.extend_from_slice(&0xa000_0003u32.to_le_bytes()); // IO_REPARSE_TAG_MOUNT_POINT
    buffer.extend_from_slice(&data_bytes.to_le_bytes());
    buffer.extend_from_slice(&0u16.to_le_bytes());
    for value in [0, substitute_bytes, print_offset, print_bytes] {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    for value in substitute.into_iter().chain([0]).chain(print).chain([0]) {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
    fs::create_dir(&path).unwrap(); // Never replace an existing entry.
    let file = OpenOptions::new()
        .write(true)
        .custom_flags(0x0220_0000)
        .open(&path)
        .unwrap();
    let mut returned = 0;
    let ok = unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            0x0009_00a4,
            buffer.as_ptr().cast(),
            u32::try_from(buffer.len()).unwrap(),
            std::ptr::null_mut(),
            0,
            &mut returned,
            std::ptr::null_mut(),
        )
    };
    let error = std::io::Error::last_os_error();
    drop(file);
    assert_ne!(ok, 0, "creating owned junction failed: {error}");
    let link = OwnedReparse {
        root: owner.0.clone(),
        path,
        directory: true,
    };
    assert!(reparse(&fs::symlink_metadata(&link.path).unwrap()));
    assert_eq!(link.path.canonicalize().unwrap(), target_path);
    link
}

#[cfg(windows)]
#[test]
fn windows_owned_junctions_refuse_root_archive_shard_and_export_redirection() {
    let f = Fixture::new();
    let other = Fixture::new();
    let sentinel = other.0.join("untouched.txt");
    fs::write(&sentinel, b"task-owned target remains unchanged").unwrap();
    let mut queue = settled(&f);
    let root_link = owned_junction(&f, "profile-link", &other);
    assert!(matches!(
        Outbox::open(&root_link.path),
        Err(OutboxError::Corrupt)
    ));
    drop(root_link);
    let archive_link = owned_junction(&f, "archive-v1", &other);
    assert_eq!(
        queue.archive_terminal(ID, WORKSPACE),
        Err(OutboxError::Corrupt)
    );
    drop(archive_link);
    private_directory(&f.0.join("archive-v1")).unwrap();
    let shard_link = owned_junction(&f, "archive-v1/00", &other);
    assert_eq!(
        queue.archive_terminal(ID, WORKSPACE),
        Err(OutboxError::Corrupt)
    );
    drop(shard_link);
    assert!(
        queue
            .archive_terminal(ID, WORKSPACE)
            .unwrap()
            .cleanup_complete
    );
    let export_link = owned_junction(&f, "exports-v1", &other);
    assert_eq!(
        queue.export_terminal(WORKSPACE, EXPORT, &[ID.into()]),
        Err(OutboxError::Corrupt)
    );
    drop(export_link);
    assert!(queue
        .export_terminal(WORKSPACE, EXPORT, &[ID.into()])
        .unwrap()
        .is_file());
    assert_eq!(fs::read_dir(&other.0).unwrap().count(), 1);
    assert_eq!(
        fs::read(sentinel).unwrap(),
        b"task-owned target remains unchanged"
    );
}

#[cfg(windows)]
#[test]
fn windows_owned_file_symlink_is_refused_or_explicitly_unqualified() {
    let f = Fixture::new();
    let other = Fixture::new();
    let mut queue = settled(&f);
    let expected = queue.entry(ID, WORKSPACE).unwrap();
    queue.archive_terminal(ID, WORKSPACE).unwrap();
    let path = archive_directory(&f.0, ID, false)
        .unwrap()
        .unwrap()
        .join(format!("{ID}.json"));
    let bytes = fs::read(&path).unwrap();
    let target = other.0.join("retained.json");
    fs::write(&target, &bytes).unwrap();
    assert_eq!(queue.entry(ID, WORKSPACE).unwrap(), expected); // Ordinary-file positive control.
    fs::remove_file(&path).unwrap();
    if let Err(error) = std::os::windows::fs::symlink_file(&target, &path) {
        fs::write(&path, &bytes).unwrap();
        if error.raw_os_error() == Some(1314) {
            // The runner must report this branch as unqualified, never count
            // a missing symbolic-link privilege as a tested security property.
            println!("WINDOWS_FILE_LINK_UNQUALIFIED: privilege not held (1314)");
            return;
        }
        panic!("creating owned file symlink failed: {error}");
    }
    let link = OwnedReparse {
        root: f.0.clone(),
        path: path.clone(),
        directory: false,
    };
    assert_eq!(
        link.path.canonicalize().unwrap(),
        target.canonicalize().unwrap()
    );
    assert_eq!(queue.entry(ID, WORKSPACE), Err(OutboxError::Corrupt));
    assert_eq!(
        queue.export_terminal(WORKSPACE, EXPORT, &[ID.into()]),
        Err(OutboxError::Corrupt)
    );
    assert_eq!(fs::read(&target).unwrap(), bytes);
    drop(link);
    fs::write(path, bytes).unwrap();
    assert_eq!(queue.entry(ID, WORKSPACE).unwrap(), expected);
    println!("WINDOWS_FILE_LINK_QUALIFIED: actual task-owned symlink refused");
}
