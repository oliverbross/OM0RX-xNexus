# Integration verification — 14 November 2026

## Current execution checkpoint

The native client at `075cea571` passed **49 tests**, Clippy with warnings denied,
and formatting on Windows Rust 1.99.1 MSVC. Tests include the shared desktop wire
fixture, immutable outbox recovery, positive edit revisions, workspace binding,
receipt validation and independent Stop. These checks use locked dependencies.

The maintained `integrations/xlog-paired` crate compiles the actual client and
actual HTTP transport from an explicitly supplied xLog checkout. Its latest run
passed **15 tests**, with the actual Swift-owner case separately opted into and
also passed. The default transport tests use synthetic providers. The separate
owner test uses production `LiveLogbookMethods` and its SQLite transaction code
in a marked empty disposable root, with no LiveWorker or device owners:

- Create through the client and HTTP bridge, then simulate losing the local
  acknowledgement after the owner commits.
- Exit both sides, reopen the same workspace/outbox, and recover the durable
  receipt without another contact.
- Apply an exact-revision correction, reject a stale correction, replay the
  original immutable create operation, and verify one history row at revision 2.
- Preserve the original millisecond UTC timestamp and multiline comment.

Passing evidence root:
`C:/Projects/OM0RX-xLog/.build/xnexus-paired-owner-evidence/xlog-companion-owner-daacbf94-577d-414a-aecb-5db8619e1651`.
Fixture executable SHA-256:
`1ba69de3acf1e0cbd23c453f8f6175ac9b44c9321f59aaf2308d6d1c91915618`.
The runner prints both checkout commits, dirty status, transport hash and fixture
hash. This test uses a synthetic station snapshot and a test provider mapping;
it is not native GUI, physical hardware or the complete Tauri host acceptance.
The owner fixture's final source/build receipt is maintained by the xLog worktree.

The isolated xLog bridge at `9ec35b9e` passed **21 Rust transport/projection
checks**, native compilation and **28 Swift tests**: ten classification cases,
twelve sync-owner cases (including real paper-QSL conflict-merge refresh), and
six durable companion cases. Source audit corrected W/New band/mode priorities,
stale PTT, no-effect mutation fences and recovered-receipt classification state.
Its final actual-owner fixture separately passed six create/restart/replay/history
checks; adjacent provenance records the exact clean source and object inventory.
The canonical xLog startup release remains owned by the other chat; no bridge
installation is implied by isolated tests.

Windows runtime source `43e305f1d65ec27f6f72255a14f2eb0775e9b349` passed the full
GNU **all-targets check**, eight fork-policy checks and **11 executed native tests**.
Those include actual FT8/FT4 generated-waveform decoding, private-root/connection
checks and retained update/profile/SSTV isolation checks. Twenty build-script
checks also passed, including Windows local/UNC path handling.

The first Windows lib-test executable compiled and linked but could not load:
Cargo's lib-test harness lacked Common-Controls v6 activation and selected an
older comctl32 without TaskDialogIndirect. The maintained runner copies only that
harness, embeds the standard activation manifest with Windows SDK mt.exe, records
both executable hashes, and verifies every requested test filter selects tests.
No application binary or system setting was patched. Passing receipt:
`C:/Projects/OM0RX-xNexus/target/xnexus-native-tests/43e305f1d65ec27f6f72255a14f2eb0775e9b349/receipt.json`.
This proves the executed native tests, not the packaged application lifecycle.

Mac arm64 source `1e456a30ceda1250eeb388952d38603081854b6f` passed native compilation
with all targets, **49 client tests**, **eight fork-policy checks**, the full
TypeScript/Vite build and **36 companion UI tests**. Two additional native helper
tests verified distinct stable WebView stores, canonical path aliases and the
macOS14 requirement. The machine runs macOS27.2/Xcode27.2; dependencies are locally
installed Homebrew builds. This does not qualify every macOS14 machine. Normal
Rust debug information avoids an independently reproduced Xcode loader defect in
zero-debug proc-macro dylibs. Logs and hashes are retained in
`/Users/oliver/Projects/OM0RX-xNexus/.build/mac-validation/`.

Windows also passed all **36 companion UI tests** and **60 existing shared
BandMap tests**. The real-browser probe passed 14 viewport/theme combinations
plus band/world selection, history correction, independent Stop and clean-console
checks. Latest screenshots:
`C:/Projects/OM0RX-xLog/.build/xnexus-worktrees/desktop/.build/xlog-browser-1794656076936`.
Browser mocks do not establish Tauri/native IPC or installed-app behavior.

The common package source adds tested packaging helpers/documentation to that
validated runtime. Its exact SHA belongs in each candidate receipt. No installer,
Mac app archive or installed native UI is qualified at this checkpoint. GitHub
run43586230218 executed no job steps due billing restrictions; Linux remains
unverified. No physical transmission, rotor movement or production-log mutation
was performed by these integration checks.

## Archived foundation evidence

The following describes the earlier bootstrap, not current integration status.

Scope: `integrations/xlog`, a standalone read-only reference client. This is not
a compiled Nexus desktop or a qualified live xLog integration.

Environment: Windows x86_64 MSVC; rustc 1.99.1 (58cf8f9c3 2026-10-13),
cargo 1.99.1 (5f94df478 2026-08-27). Dependencies resolved offline from the
existing registry cache and pinned in integrations/xlog/Cargo.lock. Clippy was
installed for the existing toolchain; no desktop or station software installed.

Passed:
- `cargo test --manifest-path integrations/xlog/Cargo.toml --locked --offline`:
  20 library tests; zero failures and zero skips.
- `cargo clippy --manifest-path integrations/xlog/Cargo.toml --locked --offline --all-targets -- -D warnings`.
- `cargo fmt --manifest-path integrations/xlog/Cargo.toml --check`.

Tests verify exact authenticated GET routes; protocol/application/capability
admission; malformed and oversized replies (declared and streaming lengths);
redirect refusal with a second listener proving it received no credentials;
changed server/workspace; stale/future data; monotonic expiry; same-revision replay;
revision rollback; explicit reconnect; unknown need status; evidence-required
needed rows; duplicate identifiers; rotor/frequency limits; HTTP denial clearing
prior authority; and bounded refused connections. All data and sockets are synthetic.

Initial failures exposed the test server's accepted sockets inheriting Windows'
nonblocking mode. The server now sets each accepted socket blocking with a read
deadline. The final green run includes all initially failing cases. An initial
text replacement missed rustfmt's split method call; that attempt did not fix the
fixture, and its failed run is not counted as a pass.

Upstream `node scripts/gates --list --unrunnable` inventoried 92 gates and reported
68 unavailable in the initial shell. This is a discovery result, not execution.
The Rust binaries were then found outside PATH for the independent client checks.
The full upstream desktop/DSP/browser/platform suites have NOT run. No UI was
changed, launched, observed or packaged. No real xLog data, credentials, radio,
rotator, cluster, audio or production profile was accessed by these tests.

At that bootstrap, release blockers remained M1b–M8. In particular, upstream
identity, data directories, stable/beta updater and hosted services are still
present in the copied desktop. The current isolation status is recorded above.

The upstream helper passed PowerShell parser validation and a real report-only
fetch. It detected upstream advancing to
`6aadc44d9cc7bbbbd655c5265f466525054c3e31` (six commits after the inspected pin)
and performed no merge or source change. The prepared source remains pinned to
`c88740a94172b3e78ae587d62bd0a54394d1e85f`; review those newer changes in the first
upstream merge after bootstrap. The merge-preparation branch path has not been
exercised against this still-uncommitted bootstrap.
