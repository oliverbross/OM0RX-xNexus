# Bounded Windows package attempt

This wrapper prepares one resource-bounded attempt; no empirical optimized
Windows peak is known. It does not promise that packaging fits in 4 GiB and
never retries, increases limits, changes Docker/WSL, closes unrelated apps, or
installs/launches the result. A candidate version/source must be reserved and
reviewed before a separate explicit execution instruction.

Run only the canonical copy of `scripts/xnexus/run-windows-guarded.ps1`, with
an exact full `-ExpectedCommit` and a private `-EvidenceRoot`. The wrapper checks
canonical path, clean source and the reviewed `build-windows.ps1` Jobs interface.
It invokes only that build script with `-ExpectedCommit`, `-MsysRoot` and
`-Jobs 1`; existing package/version/model/distribution guards remain in force.
The wrapper has no generic command, memory-limit, retry or priority option.

The internal testable guard has these boundaries:

- Windows admission: at least 8 GiB available physical RAM **and** 6 GiB available
  system commit, sampled through `GlobalMemoryStatusEx` immediately before launch.
- Private unnamed Job Object: 4 GiB aggregate committed memory for the root and
  descendants, BelowNormal priority, no breakaway permission, kill on final
  Job-handle close. This is a committed-memory limit, not a working-set estimate.
- `CreateProcessW` creates the known PowerShell executable suspended, with no
  inherited handles and no window. The guard assigns and verifies the Job before
  resuming the initial thread exactly once. Failed assignment terminates only
  the suspended process handle just created; it never falls back to an unguarded
  start. Descendants inherit the Job and cannot inherit its controlling handle.
- Each second, physical RAM and system commit are sampled. Either below 2 GiB,
  an observed Job memory-limit notification, or the twelve-hour watchdog stops
  only that owned Job. Probe/telemetry errors also fail the attempt and dispose
  the owned Job. A root that exits with surviving descendants is a failed attempt.
- Termination is asynchronous. Before closing its Job handle, the guard waits
  up to five seconds for both zero active Job processes and root termination.
  It records `finalActiveProcesses`, `rootTerminationConfirmed` and
  `cleanupConfirmed`. Failure to prove cleanup changes the result to
  `cleanup-unconfirmed`; callers must not announce the resource slot released.
  A constructor failure after launch was attempted also remains unconfirmed,
  since the PowerShell supervisor never received a usable Job handle.
- The Job cap rejects excess memory commitments independently of polling. Windows
  hard-limit completion-port messages are best-effort, so absence of a notification
  is **not** evidence that an allocation was never denied. The child exit status,
  build-script artifact verification and guard status all remain relevant. Any
  observed limit notification prevents success even if the root returns zero.

Microsoft documents [suspended assignment](https://learn.microsoft.com/en-us/windows/win32/api/jobapi2/nf-jobapi2-assignprocesstojobobject),
[aggregate Job memory limits](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_extended_limit_information),
and [completion-port notification guarantees](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-jobobject_associate_completion_port).

Every attempt creates a timestamp/UUID directory. `provenance.json` records exact
source/version and wrapper/build/helper/driver hashes. `guard.json` records the
PID, admission, verified cap, assignment ordering, priority, peak Job commit,
minimum host headroom, observed notifications, actual exit and effective result.
`telemetry.jsonl` retains per-poll observations; `build.log` retains driver/build
output. Startup failure before the PowerShell driver runs can leave that log
absent, but the guard still records the actual process result.

Before launch and after any exit/failure, the wrapper snapshots these exact
version-scoped files into separate `before`/`after` directories with hashes and
explicit absence records:

- `target/xnexus-provenance/<version>.json`
- `target/xnexus-provenance/<version>.distribution.json`
- `target/xnexus-provenance/<version>.distribution.config.json`

Prior directories are never overwritten, and partial artifacts remain in the
existing canonical build target. A later explicitly permitted retry must use a
new run directory and retain prior evidence. A failed/OOM/interrupted attempt
does not qualify an installer or justify silently widening its profile.

## Focused verification

`test-windows-guard.ps1 -EvidenceRoot <private-directory>` runs only tiny
disposable PowerShell children. Tests exercise admission boundaries, physical
and commit floors, priority, assignment before execution, blocked admission,
descendant membership, owned-family cleanup with an unrelated positive control,
OS kill-on-close after a supervisor exits without disposal, single-process and
aggregate parent/child allocation denial, timeout, and
preservation of before/after receipts. Synthetic headroom values are injected
only into this internal fixture seam; the production wrapper always uses the
real Windows counters and fixed 4 GiB limit. Test allocations use smaller Job
caps. These tests are neither package compilation nor optimized peak evidence.

The source-matched Windows fixture run passed **30 assertions**, including
positive post-stop root/descendant exit confirmation before handle closure.
Its retained evidence is `C:/Projects/OM0RX-xLog/.build/xnexus-windows-guard-evidence/fixture-cb6bcb1bf6e94a18bf299753fc25ad2e/`.
`tests.json` contains all four executable/test source hashes; each child case
retains its own guard receipt and telemetry. No actual package attempt was run.
