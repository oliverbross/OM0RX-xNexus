# Paired implementation acceptance

This opt-in test crate compiles the real xNexus client and the real HTTP transport
from a separately checked-out xLog. It is outside both application workspaces and
is never packaged. The default provider is synthetic: passing it proves wire
compatibility, queue recovery and command admission, not SQLite or hardware.

On Windows, run `scripts/xnexus/test-paired.ps1 -XLogPath <checkout>`.
The command prints both source commits and reports dirty source explicitly.
Evidence must retain those coordinates; an uncommitted owner is not a release.

On another platform, set `XNEXUS_XLOG_SOURCE` to the absolute path of xLog's
`prototype/src-tauri/src/companion_bridge.rs`, then run
`cargo test --locked --manifest-path integrations/xlog-paired/Cargo.toml`.

The separate ignored `actual_swift_owner` case requires xLog's explicitly built
`scripts/companion-owner-tests` fixture. Pass its absolute executable path as
`-OwnerFixture` and optionally `-OwnerEvidenceDirectory`. Run in the prepared
Swift shell with the Swift runtime and matching SQLite DLL directory on PATH.
The runner records the fixture SHA-256 and preserves disposable evidence.
It creates a marked empty workspace, passes QSO operations through the actual
client, HTTP transport and Swift owner, restarts both sides after a simulated
lost acknowledgement, reconciles the durable receipt, then checks correction,
stale revision rejection, original-operation replay and history. The station
snapshot remains synthetic; no hardware commands or background station owners
are enabled. This does not substitute for native application acceptance.
