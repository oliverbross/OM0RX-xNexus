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
