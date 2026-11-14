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

The Windows `-Audio` option additionally compiles the owner's actual
`companion_rx_buffer.rs` and exercises selected-channel PCM through its real
HTTP endpoint and the xNexus audio client. It checks paged cursor continuity,
stop/restart with a new route/epoch, and revocation of queued audio. Owner buffer
tests also run. Only synthetic PCM enters the buffer; no WASAPI device is opened.
The runner records the buffer's hash in addition to the transport hash. This
feature requires the receive-owner source checkout and fails if it is absent.

For packaged desktop UI checks, build the separate `xnexus-ui-owner` binary with
the same `XNEXUS_XLOG_SOURCE` coordinate. The Windows launcher
`scripts/xnexus/start-ui-owner.ps1 -Executable <absolute exe> -ExpectedSha256 <hash>`
creates a private marked TEMP directory and verifies the exact live process path,
PID and argument before returning. It opens no application or station device.
Pairing credentials stay in its private `pairing.json`; do not print them.
The fixture provides a synthetic W1AW spot, in-memory contacts and synthetic
radio/rotor actions, recording commands in `events.jsonl`. It exports no audio.
Create its `STOP` file and verify normal process exit when finished; preserve the
fixture evidence. Launch the actual candidate separately with its explicit
private data root and verify that process before any GUI binding. This fixture
cannot establish actual xLog database or physical station acceptance.
