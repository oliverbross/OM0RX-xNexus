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
  an observed Job memory-limit or abnormal-exit notification, or the twelve-hour watchdog stops
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

## Completion-message classification correction

Windows SDK `10.0.26100.0/um/winnt.h` and Microsoft's
[job-object constants](https://github.com/microsoft/hcsshim/blob/main/internal/winapi/jobobject.go)
define abnormal process exit as message **8**, process-memory limit as **9** and
job-memory limit as **10**. The previous `8 || 9` memory classification mislabeled
an abnormal exit and omitted the aggregate job-limit notification. Existing
failed receipts remain failed and unchanged; a low peak alone never proves that
memory allocation was or was not denied.

The production completion-port drain now uses named constants and a shared,
sticky classifier. `memoryLimitNotificationObserved` remains the combined
9/10 flag; receipts also retain `processMemoryLimitNotificationObserved`,
`jobMemoryLimitNotificationObserved` and `abnormalExitNotificationObserved`.
An observed message 8 independently prevents success with reason
`owned-job-abnormal-exit`, including the final notification drain after root exit.
If both abnormal and memory events occur, both flags survive; memory takes reason
priority. Ordinary root exit-code handling, caps, admission/floors, deadlines
and owned-only cleanup are unchanged. This does not import the separately pinned
external five-second natural-drain policy into the tracked guard.

After the small compiler/test slot is released, run in fresh PowerShell:

```powershell
pwsh -NoProfile -File scripts/xnexus/test-windows-guard-notifications.ps1 -EvidenceRoot <private-directory> -RunProcessFixtures
```

The tests feed literal message IDs through the exact production classifier and
exercise receipt/reason mapping, unrelated notifications, sticky and combined
events. Optional real subprocess cases use normal exit and native `ExitProcess`
with `0xC0000005`; the latter tests an actual abnormal-exit notification without
dereferencing invalid memory or invoking a crash dialog. It is an injected native
exit status, not a reproduced application access violation. Children have a
512 MiB owned Job, real production admission/floors, a 20-second watchdog and
positive exit/cleanup checks. No application, profile, device or package is used.
Completion notifications are best-effort, so a missing expected fixture event
fails this qualification rather than being silently accepted. The original full
guard fixture remains separate coverage for cap and cleanup invariants.

The focused run passed **34 checks**, including both actual child cases. Evidence:
`C:/Projects/OM0RX-xLog/.build/xnexus-guard-notification-evidence/notifications-8fee5f3d48424ffb8eea15e97cd687d9/`.
`tests.json` SHA-256 is
`0B159263D4468E60F8DDEF9572C1C94D4CC7B88E640E0D4C6120BC9DA1A0B670`.
The normal child exited 0 with no failure flags (73,113,600-byte peak). The abnormal
child retained exit 3,221,225,477 (`0xC0000005`), abnormal flag true, both memory
flags false, reason `owned-job-abnormal-exit`, effective failure 125 and peak
73,269,248 bytes. Both receipts confirmed root exit, zero final processes and
cleanup. Existing broad cap/cleanup fixtures were not rerun for this narrow fix.

For a **new** external five-second-drain supervisor copy, retain its reviewed
lifecycle implementation and update all of these seams together:

1. Copy the corrected `NativeJob.cs` and pin its new hash; do not overwrite old
   pinned runner copies. Use fresh PowerShell so an old loaded type cannot survive.
2. Add `Get-XnexusNotificationReason`, `Update-XnexusNotificationReceipt` and the
   three extra receipt fields. Call the receipt updater immediately after each
   notification drain: main monitor, natural-drain loop and final root-exit drain.
3. In both monitoring loops, retain the current headroom check, then use the
   notification reason if it returned none. Preserve deadline precedence. At the
   final drain, reject any returned notification reason before success. Do not
   change limits, timeout, drain duration, retained-child identity/exit checks or
   cleanup policy. Re-pin helper hashes and test the new external copy separately.

Copying only NativeJob would correct memory labels but could silently admit an
observed abnormal descendant exit where the root returns zero; the PowerShell
failure checks are required as well. Historical receipts remain unchanged.
