# Qualification checkpoint — 14 November 2026

This checkpoint separates implemented behavior, built artifacts and operational
acceptance. It does not close the project milestones or qualify live station use.

## Sources and artifacts

| Component | Exact source | Evidence at this checkpoint |
| --- | --- | --- |
| Windows xNexus `1.24.1-xlog.2` | `c2c917b6f748366fbf6b3960876d9f98ba1854c9` | Canonical source frozen; optimized release packaging running. No completed installer claim. |
| RX runtime before version preparation | `e3ae1534ade5c34dd4e2d3a81bfbe3a72be15d5c` | Windows GNU Rust 1.93.1 all-targets check passed. |
| Mac xNexus `1.24.1-xlog.1` | `5e56be94c7ede2099d5b8ed6d1a4d0d1ad12134b` | Core application archived and initial private native UI observed. This older candidate does not contain the integrated RX runtime. |
| Mac newer RX candidate | `c2c917b6f748366fbf6b3960876d9f98ba1854c9` | Full native all-targets check passed with Rust 1.93.1 in 3m56s. No matching Mac package claimed here. |
| Installed xLog `1.6.26` | `df8d81c9d8b9c760fe7a3eee0f25990cb78db732` | Accepted and installed by the xLog work chat; acceptance documentation at `a22a1db38dde6c791ab7a45f66d39c159dc2b3c6`. |

The Mac core archive contains 56,583,180 bytes and has SHA-256
`ab8fb543ba702bdf3ca5fc75673945d49a62b6d9fa7a391dab317d42a509b2af`.
Its original failed system-symbol allowlist receipt is preserved. A supplemental
audit reviewed 58 additional Hamlib imports against Apple library and SDK
evidence, verified all seven Mach-O files and deployment floor 14.0, and proved
the bundle bytes unchanged before archiving. The main executable is ad-hoc
signed; there is no Developer ID signature or notarization. The archive remains
on the Mac, with no public upload or installation claimed.

## xLog acceptance handoff

The owner integration at `5efffc1a23af68157e3c564eb2aa345256bea90a` is included
in installed 1.6.26. The xLog chat reports 1,155 Rust tests, 60 reference/Chaser
tests and 34 FLEX tests, plus focused UI checks. Its quiet 365-second installed
check recorded 1,571 station replies, no transport failures or worker retirements,
stable station connections and unchanged profiles. These are xLog acceptance
results, not proof of the two applications paired together.

The preceding 1.6.25 candidate failed the existing Settings bundle-size gate
before installation. Lazy loading corrected the issue without raising the limit.
Earlier compression-loaded acceptance saw three passive-read timeouts; that
evidence is retained alongside successful partial-load and quiet rechecks.

## Private native UI checks

The Mac core app displayed its full `.1` version and source `5e56be94c` in a
process-verified private WebView store. It showed no pairing, disconnected xLog,
an empty queue and unavailable station controls. Initial dark 100% and light
125% window-only captures were readable. Four explicit private launches ended
with verified ordinary Quits. A synthetic unqueued `K1ABC` draft survived an
ordinary Quit and explicit restart of the same binary/profile. Theme and scale
reset because their source uses session-only component state; this is separate
from the successfully verified draft storage. No pairing, QSO submission or
hardware action was performed.

A fresh protected Windows xLog fixture used explicit offline private-root
arguments on both host and worker. Exact process identities, parent relationship,
arguments and package hashes were verified before accessibility observation.
The UI showed first-run setup and disabled station I/O. Two disposable `N0CALL`
identity fields were entered. Capture timed out; an accessibility-only recovery
succeeded, but Continue failed with unavailable input geometry. Activation and
fresh capture also timed out. The sequence stopped, retaining its process receipt,
logs and last successful accessibility tree. No workspace or pairing was created,
no station action was performed, and production xLog was untouched. The private
fixture was left idle rather than force-quit.

## Linux and hosted checks

The pinned Linux image and locked dependency provisioning succeeded. An offline
contract run passed 58 SDK tests and eight fork-policy checks before TypeScript
exhausted Node's default 2 GB heap. The failed receipt is preserved. A retry with
an explicit 4 GB Node heap stays inside the same 6 GB container, two CPU limit
and network isolation; it waits for at least 10 GiB host memory. No retry or full
Linux desktop pass is claimed. The exact `.2` source snapshot is prepared.

GitHub Actions validation remains blocked before execution by billing. Inherited
publishing workflows remain disabled and repository guarded. Local evidence does
not represent hosted CI success, and no billing settings were changed.

## Findings that still gate live control

Review found that the xLog companion rotor identity currently names the backend
family, rather than a particular connection. Retained emergency Stop can
therefore match a replacement rotator of the same family. Ordinary motion
revision checks do not solve this, because Stop must survive motion changes on
the same connection. A connection-identity fix is being implemented in an
isolated xLog worktree, with owner review required before canonical integration.
It must preserve native emergency Stop and dispatch to a captured existing worker
generation. Regression coverage must include both backend families, same-route
motion, same-family replacement, worker retirement and delayed dispatch.

Physical FLEX slice A/DAX reception and rotor/radio control remain unqualified.
The operator confirmed DAX and slice A; DAX channel selection must come from
actual metadata and explicit endpoint association. Host endpoint timing is not
proof of UTC synchronization or RF-path delay. Decoded-call batch need lookup
and transmission remain separate unfinished work.

## Upstream maintenance

An isolated merge rehearsal from exact `.2` source to upstream
`3885df0fe0d992a52f51ff5481025f860b2322f5` had no conflicts across 29 changed
files. Eight fork-policy checks passed and all 44 inherited workflow jobs kept
their repository guards. The pending merge and patch evidence are preserved;
upstream runtime changes have not been qualified or promoted. The maintained
upstream base remains `c88740a94172b3e78ae587d62bd0a54394d1e85f`.
