# Receive client development evidence — 14 November 2026

This extension is separate from the frozen first candidate `5e56be94c`, version
`1.24.1-xlog.1`. It is not a packaged or physically qualified audio receiver.

Client source `591348991c8c2935f95297e3e373bac497f70de4` passed 58 tests,
Clippy with warnings denied, formatting and whitespace checks on Windows with
the installed Rust 1.99.1 MSVC toolchain. The tests cover owner capability,
selected-channel PCM, sequence/frame gaps and repeats, route changes, bounds,
unknown UTC/RF timing, HTTP authentication/revocation, oversized/foreign replies,
local session replacement, queued expiry and receiver shutdown. Pinned 1.93.1
desktop compilation and complete receive-path qualification remain separate.

The additive base64 dependency reuses version 0.23.1 already in the lockfiles.
Only its client dependency edges changed; unrelated desktop dependency edges
were preserved after the newer resolver proposed unrelated Windows changes.

Against canonical xLog source `824d256d9299faf92c6216009c89f56a493ebcc8`, the
maintained paired harness passed 15 production HTTP/client checks. Its explicit
actual Swift-owner test then passed independently: create, process restart,
durable receipt reconciliation, corrected contact revision, stale edit refusal,
immutable create replay and single-contact history. That owner test uses real
SQLite/logbook methods with disposable storage, not the production application
or devices. Two newer-toolchain deprecation warnings originate in the owner's
existing atomic API; they are not runtime failures.

Owner transport SHA-256:
`7ef9230419f816b89f019da782a16cfbbed8be1305d9dcaf2d4c44c6b8da9c9f`.
The owner fixture was built from `9ec35b9e`, whose Swift owner source is unchanged
by the startup-baseline merge. Executable SHA-256:
`2ce88d4406539af2a0dcd8af60e26859f2ff73e020620f1935a2a70463763981`.
The local evidence directory identifier is
`xlog-companion-owner-198d0246-6f1a-487d-9447-ef15f2f42903`.

The operator confirmed FLEX slice A with DAX receive audio. The actual DAX channel
must come from observed FLEX metadata and explicit stable endpoint binding; the
example channel number is not a default. The separate xLog audio owner branch,
period assembler and desktop worker still need integrated transport/decoder/UI
checks. Real DAX input, endpoint-to-RF delay and UTC synchronization have not been
measured. No TX capability is provided by this client.
