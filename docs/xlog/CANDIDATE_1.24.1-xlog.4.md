# Candidate 1.24.1-xlog.4 — xLog-owned decoded station needs

This private development candidate has a completed Windows build, static artifact
audit and independent retention at exact source
`ae35fc6435f861e38029049299833afe57de4885`. The [disposable install/uninstall](WINDOWS_INSTALLER_1.24.1-xlog.4.md)
passed, followed by native A→B→A profile/lifecycle checks and permanent current-user
installation. The installed application was left closed; paired UI remains pending.
It is unsigned and not a published release. See [the durable build evidence](WINDOWS_BUILD_1.24.1-xlog.4.md).
Exact pre-version source is
`7f08478c564382f4cc9eed775846faa9a088ba1e`.
Its production implementation matches tested integrated source
`0d01248b0504ae77f6f122fdada1b21e4a61806b`; later changes are documentation,
Linux validation tooling and the generated FT4 integration test. This version
commit changes only the five version manifests and candidate documentation.
The upstream pin remains `c88740a94172b3e78ae587d62bd0a54394d1e85f`.
The separate upstream merge/serial-alias review is not included.

The paired owner is canonical xLog 1.6.27 source
`16cac86739274073ab2f8000a3916b4e37bc57a5`, whose pre-version implementation is
`8bd486abd2e62aa079e8b4248126e1e186561452`. The xLog owner reports package,
installation and installed qualification passed; documentation is now at
`7789c569`. xLog is currently closed, with ordinary owner handoff and live
pairing pending. Installed qualification does not establish paired endpoint
or physical station acceptance.

## Changes and qualification

The standalone companion shows xLog's needs, score and evidence on decoded
FT8/FT4 rows. It preserves immutable decode/context identity, original evidence
deadlines and durable pending-write suppression. Unknown or expired evidence
does not become Needed. Lookup work is bounded and separate from PCM and Stop.
The profile construction repair, fork identity, disabled upstream updater and
reviewed installer ownership guards are included.

[Detailed evidence](DECODED_NEEDS_VERIFICATION.md) records owner tests and 40
actual Swift replies, 80 SDK tests, 19 native attachment tests, all 55 companion
UI tests, TypeScript/Vite, 14 production-browser cases, Windows all-targets
checking and a generated FT4 signal through the actual decoder/HTTP validator.
The last test uses a synthetic classifier; it is separate from the actual Swift
wire replay and does not exercise a live logbook or DAX hardware.

Updated Linux tooling fixtures and cache seeding passed, but contract/desktop
checks were held for host memory. Updated Mac checking remains held; the latest
32% resource probe was below its 35% admission threshold, with no new frontend
or compiler child. Earlier `.3` evidence does not qualify this new source.
Windows private profile/lifecycle, disposable install/uninstall and permanent
installation passed. Native paired acceptance remains open. The operator confirmed FLEX slice A → DAX Audio RX 1 and an
unlocked, available desktop. Exact live mapping, endpoint reception and timing
still require physical qualification. Digital transmit is not implemented or enabled.

## Exact build provenance

The packaged canonical source is `ae35fc6435f861e38029049299833afe57de4885`,
verified clean and equal to the fetched fork `origin/main`. The build receipt
binds that exact source, version, upstream pin, actual tools, lockfile,
configuration, native/model/license inputs and artifact hashes. Canonical source
remains frozen during acceptance. Do not reuse this version for different source.

Historical invocation for the one completed Windows attempt (not a retry request):

```powershell
pwsh -NoProfile -File C:\Projects\OM0RX-xLog\.build\xnexus-supervisor-12g-drain-ae35-review\run-package.ps1 `
  -Execute -ExpectedCommit ae35fc6435f861e38029049299833afe57de4885 `
  -MsysRoot C:\msys64 -EvidenceRoot C:\Projects\OM0RX-xLog\.build\xnexus-windows-guard-evidence
```

The helper invokes pinned GNU Rust 1.93.1 and Tauri CLI 2.11.5 with one
Cargo/CMake job, a 2,560 MiB Node heap, unchanged optimized release profile and
unsigned NSIS packaging from canonical `C:\Projects\OM0RX-xNexus`. The reviewed
external Job had a 12 GiB cap, 20 GiB physical/commit admission and 6 GiB runtime
floors, plus a separately tested five-second natural descendant drain. The
canonical guard/application sources were unchanged. Actual peak was 9.38 GiB;
root exit 0, zero active descendants and no forced live cleanup were confirmed.
No automatic retry or cap increase was permitted.

Actual CLI process/loaded-module evidence was captured. Static auditing passed
against this version/source and process receipt; standalone and NSIS-embedded
hashes remain separate, with only the supported three-byte bundle-marker change.
All 60 retained files passed copy/hash checks and older `.2` evidence stayed
unchanged. The two NSIS `WixMode` warnings passed exact-toolchain inert branch
equivalence checks; obsolete source wrappers remain a cleanup follow-up.
Follow [installer acceptance](WINDOWS_INSTALLER_ACCEPTANCE_PLAN.md) and
[private profile acceptance](PRIVATE_PROFILE_ACCEPTANCE.md) as separate gates.
No real application launch, installation, signing, release tag or public upload
is implied by the completed build or audit.
