# Candidate 1.24.1-xlog.4 — xLog-owned decoded station needs

This reserves a private development candidate, not a release or installed build.
Exact pre-version source is
`7f08478c564382f4cc9eed775846faa9a088ba1e`.
Its production application bytes match tested integrated source
`0d01248b0504ae77f6f122fdada1b21e4a61806b`; later changes are documentation,
Linux validation tooling and the generated FT4 integration test. This version
commit changes only the five version manifests and candidate documentation.
The upstream pin remains `c88740a94172b3e78ae587d62bd0a54394d1e85f`.
The separate upstream merge/serial-alias review is not included.

The paired owner is canonical xLog 1.6.27 source
`16cac86739274073ab2f8000a3916b4e37bc57a5`, whose pre-version implementation is
`8bd486abd2e62aa079e8b4248126e1e186561452`. It is also source-prepared only.
Installed xLog 1.6.26 remains unchanged. Neither source preparation establishes
that the new owner endpoint is installed or that live pairing works.

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
checks were held for host memory. Updated Mac checking was held before any
frontend/compiler child. Earlier `.3` platform evidence does not qualify this
new source. Windows native lifecycle, installer and paired acceptance remain
open. FLEX slice A is confirmed; its actual DAX RX channel must still be selected
and physically qualified. Digital transmit is not implemented or enabled.

## Exact build provenance

After this commit is pushed and fast-forwarded into canonical `main`, record
its full SHA as `C`. Only clean `C`, equal to freshly fetched `origin/main`, may
be packaged. Do not substitute the pre-version SHA. A build receipt must bind
`C`, version `1.24.1-xlog.4`, upstream pin, actual tool paths/versions/hashes,
lockfiles, configuration, native/model/license inputs and artifact hashes.
Do not reuse this version for a different packaged source.

Windows command from canonical `C:\Projects\OM0RX-xNexus`:

```powershell
pwsh -NoProfile -File scripts/xnexus/run-windows-guarded.ps1 `
  -ExpectedCommit <C> -EvidenceRoot <fresh-private-evidence-directory>
```

The helper invokes pinned GNU Rust 1.93.1 and Tauri CLI 2.11.5 with one
Cargo/CMake job, a 2,560 MiB Node heap, unchanged optimized release profile and
unsigned NSIS packaging. The outer private Job has a 4 GiB aggregate limit,
8 GiB physical/6 GiB commit admission and 2 GiB runtime floors. Failure or
insufficient admission preserves evidence; no automatic retry or cap increase.
Successful compilation does not prove final optimization fits the memory cap.

Record the actual loaded official CLI module during the build. After clean
completion, run `scripts/xnexus/audit-windows.ps1` with this exact version,
commit and CLI process receipt. Preserve standalone and installer-embedded
executable hashes separately, including the reviewed NSIS marker transformation.
Independently retain the artifacts before any later build can replace them.
Follow [installer acceptance](WINDOWS_INSTALLER_ACCEPTANCE_PLAN.md) and
[private profile acceptance](PRIVATE_PROFILE_ACCEPTANCE.md) as separate gates.
No app launch, installation, signing, release tag or public upload is implied
by this reservation or build command.
