# Qualification checkpoint — 14 November 2026

This checkpoint separates implemented behavior, built artifacts and operational
acceptance. It does not close the project milestones or qualify live station use.

**Superseding native finding:** `.2` showed a previous test draft in a fresh
profile. The locked Tauri configuration conversion discarded its WebView storage
settings. `.1`/`.2` are retained as failed-isolation candidates; the earlier
same-profile persistence test did not prove cross-profile isolation. See
[the failure record and repair gates](PROFILE_ISOLATION_2026-11-14.md).

## Sources and artifacts

| Component | Exact source | Evidence at this checkpoint |
| --- | --- | --- |
| Windows xNexus `1.24.1-xlog.2` | `c2c917b6f748366fbf6b3960876d9f98ba1854c9` | Installer and static audit complete; not launched/installed, rejected for shared profile-construction defect. |
| RX runtime before version preparation | `e3ae1534ade5c34dd4e2d3a81bfbe3a72be15d5c` | Windows GNU Rust 1.93.1 all-targets check passed. |
| Mac xNexus `1.24.1-xlog.1` | `5e56be94c7ede2099d5b8ed6d1a4d0d1ad12134b` | Core application archived and initial private native UI observed. This older candidate does not contain the integrated RX runtime. |
| Mac newer RX candidate | `c2c917b6f748366fbf6b3960876d9f98ba1854c9` | Full all-targets check and supplemental archive audit passed; native fresh-profile isolation failed before pairing. |
| Installed xLog `1.6.26` | `df8d81c9d8b9c760fe7a3eee0f25990cb78db732` | Accepted and installed by the xLog work chat; acceptance documentation at `a22a1db38dde6c791ab7a45f66d39c159dc2b3c6`. |

The Mac core archive contains 56,583,180 bytes and has SHA-256
`ab8fb543ba702bdf3ca5fc75673945d49a62b6d9fa7a391dab317d42a509b2af`.
Its original failed system-symbol allowlist receipt is preserved. A supplemental
audit reviewed 58 additional Hamlib imports against Apple library and SDK
evidence, verified all seven Mach-O files and deployment floor 14.0, and proved
the bundle bytes unchanged before archiving. The main executable is ad-hoc
signed; there is no Developer ID signature or notarization. The archive remains
on the Mac, with no public upload or installation claimed.

The Windows `.2` NSIS installer contains 268,932,507 bytes, SHA-256
`3acf27223d19c6b6e135bdd4dfe5d354076f1e9cea6e573196a41391a04df4d2`.
The static audit verified product/version, Common Controls v6, imports, 13
model/Hamlib/loader/license resources and a valid Microsoft-signed offline
WebView2 payload. Its standalone executable SHA-256 is
`cb44ba10d28fc819290165135b059788196ac4df6da3a8f4e1e00a94228b3bf7`;
the installer-embedded executable is
`40a36705e7d3f47a64ab24b3bc095bec086ede167592cf6c3857ff3295cab5df`.
Both contain 114,630,669 bytes. The only difference is the documented Tauri
bundle marker `UNK` to `NSS`, three bytes at offsets 86,416,352–86,416,354.
The audit requires that exact transformation and retains the earlier failed
equality check. Neither application nor installer was executed.

The Mac `.2` archive contains 56,650,521 bytes, SHA-256
`aed846e7a7b2910d1bf937f11c6ea8c3e571e8bf26e2d73d867a985bf5744fbc`.
Its executable SHA-256 is
`877ae5c53cc223f95d85174651b75911562dc531bac3887afd86f8450907c041`.
The original old-symbol-policy failure is preserved; supplemental audit proved
unchanged bundle bytes and the same 58 reviewed imports across seven Mach-O
files. The actual invoked CLI was the locally cargo-installed Tauri 2.11.5,
not the downloaded official executable. Its process/path/hash and installation
metadata are bound by the supplemental receipt; application Rust is 1.93.1.
Future tooling invokes the verified downloaded CLI directly. Historical `.1`
actual CLI-path provenance remains unproven. These archives are unsigned and
not notarized, and the `.2` native isolation failure prevents their qualification.

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
the same connection. A connection-identity fix was reviewed and merged into
canonical xLog at `f155bd2d9732bf8fbb32e89a6153804f284d3891`, with application
source `eacfd0c65255023d15ada3b557cb77edac2408bf`. It preserves native emergency
Stop and dispatches to the captured existing worker generation. Ninety-five
focused tests and 1,164 full native tests passed (zero failed, two existing
ignored), including both backend families, same-route motion, replacement,
worker retirement and delayed dispatch. Installed 1.6.26 is unchanged and does
not yet contain this fix. A preceding memory-capped compile failure is retained
separately; it never reached tests.

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
