# Foundation verification — 14 November 2026

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

Release blockers remain M1b–M8 in PROJECT_PLAN.md. In particular, upstream
identity, data directories, stable/beta updater and hosted services are still
present in the copied desktop. Do not call this a usable integrated build.

The upstream helper passed PowerShell parser validation and a real report-only
fetch. It detected upstream advancing to
`6aadc44d9cc7bbbbd655c5265f466525054c3e31` (six commits after the inspected pin)
and performed no merge or source change. The prepared source remains pinned to
`c88740a94172b3e78ae587d62bd0a54394d1e85f`; review those newer changes in the first
upstream merge after bootstrap. The merge-preparation branch path has not been
exercised against this still-uncommitted bootstrap.
