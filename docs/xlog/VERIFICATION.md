# Integration verification — 14 November 2026

## Current execution checkpoint

The native client at `075cea571` passed **49 tests**, Clippy with warnings denied,
and formatting on Windows Rust 1.99.1 MSVC. Tests include the shared desktop wire
fixture, immutable outbox recovery, positive edit revisions, workspace binding,
receipt validation and independent Stop. These checks use locked dependencies.

The maintained `integrations/xlog-paired` crate compiles the actual client and
actual HTTP transport from an explicitly supplied xLog checkout. Its latest run
passed **14 tests**, with the actual Swift-owner case separately opted into and
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
`C:/Projects/OM0RX-xLog/.build/xnexus-paired-owner-evidence/xlog-companion-owner-4fa3217e-f16c-4403-a93a-e7a223cdef51`.
Fixture executable SHA-256:
`0efc1eabedb67c431af66a1e8f1432e7a3cde711478f71ab298588cb186421fe`.
The runner prints both checkout commits, dirty status, transport hash and fixture
hash. This test uses a synthetic station snapshot and a test provider mapping;
it is not native GUI, physical hardware or the complete Tauri host acceptance.
The owner fixture's final source/build receipt is maintained by the xLog worktree.

The isolated xLog branch passed six Swift durability/history tests, native Rust
compilation, TypeScript and transport/projection tests. Independent review found
stale Hamlib PTT projection and need-invalidation lifecycle defects; their fixes
are being verified before canonical integration. The existing xLog release owner
retains canonical ownership while fixing its separate startup acceptance failure.

Canonical xNexus `f96eed63c` passed the full native Windows GNU **all-targets
check**, including test compilation. Twenty build-script tests passed, including
Windows local/UNC path normalization. Native test execution on the subsequent
`d5cf11014` draft-recovery source is still building; compilation is not execution.
Six fork identity/workflow policy tests passed. No installer is qualified yet.

Desktop evidence includes the full TypeScript/Vite build, 31 companion tests at
`96de58dce`, and focused tests for subsequent history-draft and owner-band fixes.
The real-browser probe passed 14 viewport/theme combinations plus band/world
selection, history corrections, independent Stop and clean-console assertions.
Latest screenshots are retained under
`C:/Projects/OM0RX-xLog/.build/xnexus-worktrees/desktop/.build/xlog-browser-1794656076936`.
Browser mocks do not establish Tauri/native IPC or installed-app behavior.

GitHub run 43586230218 did not execute any job steps because of the account's
payment/spending restriction. Local macOS verification is being attempted on the
connected Mac; no platform pass is claimed before its result. Linux remains
unverified. No physical transmission, rotator movement, production-log mutation
or installation has been performed by these integration checks.

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
