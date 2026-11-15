# Terminal outbox retention — SDK qualification

Base `7e0d77d3da03e2d5f60caaabb6f7af81e51f5fc8`. This is an SDK-only port of the
preserved archive draft, with additional regressions. Windows SDK tests,
formatting and Clippy passed within the qualifications below.
There is no native command, UI, migration, automatic maintenance, HTTP endpoint,
profile access or hardware change. Dependencies and the lock file are unchanged.

The original outbox Git blob is identical at the old draft base `75a59`, `bbae`
and this base: `4754c33f5929d77be4508981af30e856bf4ae615`. Only the three reviewed
SDK files were copied from the unique old worktree; its native/custom-dashboard
and probe changes remain there, untouched. Reviewed source hashes before this
port's added tests and scoped formatting were:

- `outbox.rs`: `D90E5A0BCAC303B87952EFC1D2F1322DDC383001188ECF19C3ACD28399E106ED`
- `outbox/archive.rs`: `4051D9D6C5EC933FB186D3C74D5F5871472E4E576ACFAAE8B67A3A122FC43D08`
- `outbox/archive/tests.rs`: `9FD7543FDC7572382552CD1977E2E3A5DC57E620AE764317CD3DC1B554CB9CBF`

## Retention and recovery contract

Explicit archive accepts only Applied, Observed or Rejected terminal records.
The complete immutable queued mutation and receipt remain in a versioned,
UUID-sharded archive permanently. Pending/NeedsReceipt cannot be archived.
An identical operation remains an idempotent duplicate after restart; a reused
ID with different mutation, expected revision or workspace is refused. There is
no archive import, tombstone deletion or automatic mutation replay.

Publication precedes removal of active journal fragments. Existing published
records are resynchronized before retrying cleanup. An incomplete cleanup keeps
the exact operation charged against the active capacity, including after restart;
its projection fails visibly if the retained archive later disappears or corrupts.
Startup bounds root enumeration and reads archives only for exact IDs, without
enumerating lifetime terminal retention. Retained submit/reconcile/retry returns
the exact receipt without HTTP, after the existing workspace check.

Exports select one to 32 exact terminal operations, publish without replacement
and do not change authority. The shared publication primitive enforces 32,768-byte
record and 1,048,576-byte export limits. Valid bounded QSO fields and the 32-entry
limit currently make a full one-MiB export unreachable; that exact byte-boundary
test exercises the real publication primitive rather than inventing oversized
valid QSO fields. The record read boundary uses valid JSON trailing whitespace.

`entry` now returns an owned `OutboxEntry` so exact archived records can be loaded
on demand. Existing current repository callers do not retain a required borrowed
entry. SDK tests and all-target Clippy passed; native caller compilation remains
a separate integration gate.

Future native/UI integration must preserve frozen draft recovery: current
`xlog_outbox_list` returns active entries only, and editor/manual recovery reads
that list. Before exposing archive maintenance, provide an exact-ID local reader
and corresponding recovery path for archived receipts, or prevent archive while
a retained draft still depends on the active entry. Do not strand an Applied
draft by hiding its terminal receipt. Archive/export I/O belongs on a Queue-only
blocking lane, without holding the main client or independent rotor Stop path.
Those changes are deliberately outside this unwired SDK port.

## Filesystem boundary and limits of evidence

The caller supplies a trusted private profile root. Directory and file checks
reject symlinks and Windows reparse points at the root and archive/export paths.
This is not a hostile concurrent-filesystem sandbox; ancestors outside that
private-root contract and external deletion/replacement are not made safe by
these checks. Lifetime archive storage grows intentionally to retain duplicate
fences; the API never silently discards them.

Unix publication uses a synced file and atomic no-replace hard link, then removes
the temporary name and synchronizes directories. Windows publication requests a
write-through move without replacement. Injected failure tests demonstrate the
implemented recovery ordering; they do not demonstrate empirical power-loss
durability, disk-controller guarantees or Unix behavior on Windows.

Windows junction tests create only verified fixture-owned local paths using a
mount-point reparse record, verify its target and remove the link nonrecursively.
The target has an unchanged sentinel. Root, archive base, archive shard and
export-directory junctions were created and their redirection refused. The file
symlink attempt recorded an explicit unqualified marker because privilege 1314
prevented creation. The runner reported that limitation separately; a harness `ok` in that branch is
not proof of file-symlink rejection. No privilege or global system setting changes
are permitted. Unix link/no-replace/permissions checks need a separate Unix gate.

## Windows SDK qualification

Twenty-one Windows archive cases (15 preserved and six additional) ran alongside the
unchanged 173-test SDK baseline: 194 listed, 191 passed and three existing
external-input replays ignored; zero failures. The file-symlink case returned after
error 1314, so twenty archive cases exercised their intended rejection/recovery
behavior, while file-symlink rejection remains explicitly unqualified. Actual
task-owned junction checks passed. The six additions cover missing archive cleanup
projection/capacity, Update/Observed exact revision and no-network recovery,
record read byte boundaries, publication byte boundaries, actual Windows
junctions and file symlinks. There is one separate Unix-only archive-link case.

The completed runner used pinned GNU Rust 1.93.1, offline/locked one-job Cargo,
all SDK library tests, all-target Clippy and scoped formatting checks, all successful.
It copied
only hash-pinned third-party dependency cache files into a fresh attempt target,
never current SDK artifacts or incremental outputs; donor hashes were checked
before and after. Private Job limits were 2 GiB/300 seconds, BelowNormal,
8/6 GiB physical/commit admission, 2/2 GiB floors and three-second natural drain.
Any failure is preserved without automatic retry or widening. Full Tauri/native,
UI/browser, actual owner and installed/power-loss qualification are not claimed.

### Exact retained attempt

`target/sdk-terminal-archive-preparation/attempt-0168036e518f4487b0dbea3b97093f4c`
completed on 2026-11-15 at 20:51:34 UTC. Guarded root PID 22564 exited zero
naturally, active processes were zero before cleanup and at final verification,
cleanup was confirmed, and neither memory-limit nor abnormal-exit notifications
occurred. Peak Job memory was 748,826,624 bytes. Initial physical/commit headroom
was 20,733,788,160 / 19,284,668,416 bytes. No retry or policy change was used.

- Guard receipt: `C0BC7E92D1EA583811C485AE78FBF8775941076AD2144553B89ED0A4E9358391`.
- Check summary: `BC5B0A4AE58779BA310BFD93C3B4FE3858CE034FA59249F902675AA5E872310A`.
- Actual library test log: `DBA69B28F38F4D1A4AB0B2FABDB411B27ECEC54CA1A6BF0AA40C19F7168FECAD`.
- Clippy log: `AFD42F7A7434241A18B43C569BBFBB2316E00EA8A66AAB6F64CA215D6730834D`.
- Identical before/after source manifests: `DFA4612F7C6BD11C431E190E3909713B3D3A05BC81526BC30E27AA6B3F76D838`.
- Newly compiled SDK test executable: `2ED1BE2CC2403D42ACB6D31E8294137A04CD0E7DE9A27DF4BE03238F3887B689`.
- Fresh-source Cargo artifact record: `3367045638C348BD120EA0BF2DC21CCA57C825A09183F4158600F7A30ED91CA6`.
- Source inventory: `AE30E2BF5C9974047A7FA229979DBF402F33132E7325E2917BD0E83B8598AB3F`.
- Runner/tool pins: `F80BBB53B3EA1E3608CC7B8D78EDD80BC539A0D127502DB8538F824737C6F8AC`.
- Donor inventory: `5127A8A980A205946A44458C5F098AC1B69DC3A5EA41137DC0F806EC3CED6C5E`; 1,702 retained files verified, 1,659 third-party files copied into a new private target, zero SDK or incremental files copied. Donor remained unchanged.

Qualified production hashes are `outbox.rs` D90E (full hash above) and
`outbox/archive.rs` `24FBF9F4FD8B6A87CB7F0C8E96BC996037E300D49C6AFB93FF0A7CDBC5F06DAC`;
the latter differs from the old draft only by scoped formatting. The added test
source is `FC8EAFAAFA673CACAE557F5BAB18FE86945704062E55E6D3E10B6B401D2E739A`.
Only this qualification document changed after the passing source checks.
The old worktree and all its unique native/UI source remain untouched.
