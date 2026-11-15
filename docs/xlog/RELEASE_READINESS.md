# OM0RX-xNexus release readiness

Windows [1.24.1-xlog.4](CANDIDATE_1.24.1-xlog.4.md) is built, statically
audited and independently retained at clean canonical source
`ae35fc6435f861e38029049299833afe57de4885`. It includes the reviewed profile
repair, installer hardening and completed SDK/native/UI integration.
[The exact build evidence](WINDOWS_BUILD_1.24.1-xlog.4.md) records artifacts,
actual tools, resource limits, natural process cleanup and the NSIS warning
qualification. The [real disposable install/uninstall](WINDOWS_INSTALLER_1.24.1-xlog.4.md)
passed, followed by native A→B→A profile/lifecycle checks and permanent current-user
installation. The installed application was left closed. Paired UI and physical
station acceptance remain open. The candidate is unsigned; package/runtime
source remains exact `ae35`, regardless of later documentation/tooling commits.
Source tests, an artifact audit, native lifecycle checks and physical station
qualification remain separate gates; see the maintained [execution ledger](STATUS.md).

The `.2` Windows and Mac builds completed at
`c2c917b6f748366fbf6b3960876d9f98ba1854c9`. Their artifacts passed static audits,
but Mac qualification exposed shared WebView storage across private profiles.
Neither `.1` nor `.2` is qualified for isolated-profile use. Preserve the
[incident evidence](PROFILE_ISOLATION_2026-11-14.md); do not launch or distribute
those candidates as repaired applications.

The repaired [`.3` candidate](CANDIDATE_1.24.1-xlog.3.md), source
`d206b880216543af2142e600ea6b8e5ac6bdd1b2`, passed Mac artifact, A→B→A profile
isolation and core paired synthetic checks. Its Windows package was deferred
in favor of the next reviewed candidate. Those Mac checks do not qualify later
decoded-needs changes, active DAX reception, Windows installation or distribution.

The previous Windows `.1` build at `5e56be94c7ede2099d5b8ed6d1a4d0d1ad12134b`
was deliberately stopped before the application build to include completed RX
work. It produced no Windows application or installer; its aborted receipt and
log remain preserved. The Mac core `.1` application compiled at that original
source; its archive passed the supplemental native symbol audit and initial
draft/lifecycle checks, which did not catch the later isolation failure. Its
historical CLI executable path remains unproven;
do not claim the downloaded official CLI was used. It is not the RX candidate.
Do not reuse `.1` for changed source.

The current Windows recipe uses Rust 1.93.1 GNU, Tauri CLI 2.11.5, pinned native
and model assets, **one Cargo/CMake build job**, the unchanged release profile and
unsigned NSIS packaging. The completed `.4` attempt used a separately reviewed
external supervisor with a 12 GiB aggregate Job cap, 20 GiB physical/commit
admission, 6 GiB runtime reserves and a five-second natural descendant drain.
Canonical supervisor sources remain unchanged; their earlier 4 GiB default is
not a qualified fat-LTO allowance. The measured `.4` peak was 9.38 GiB, with
natural root exit, zero active descendants and no forced live cleanup.
The Windows helper pins and records a 2,560 MiB V8 heap allowance. The complete
decoded-needs frontend checks passed with this setting under a 4 GiB job
(2,585,292,800-byte peak); smaller 1,280/1,536 MiB heaps exhausted before useful
type diagnostics. This measures the frontend, not the final native optimizer.
The frontend result does not imply final fat LTO fits 4 GiB; the completed
optimized build's separate memory record is authoritative for `.4`.
Only clean canonical `main` may be packaged. Build receipts bind the exact source,
upstream pin, configuration, tools, dependencies and artifacts. xLog packaging and
installed acceptance remain owned by the “xlog work” chat; an SDK/owner source
test does not establish that those changes are installed. The owner reports
1.6.27 portable/installer packages, installation and installed qualification
passed. Current owner documentation is `7789c569` over application `16cac867`;
ordinary owner handoff and live pairing remain pending. Changing packaged
source requires a new visible version, including failed qualification candidates.

The fork retains KD9TAW attribution, COPYING, NOTICE, and third-party notices.
Its product/binary is OM0RX-xNexus, app identifier `sk.om0rx.xnexus`, default
legacy-mode profile `om0rx-xnexus`, profile override `XNEXUS_PROFILE`, and shared
data override `XNEXUS_DATA_DIR`. Legacy keyring entries use fork-owned namespaces.
Connected mode initializes its own restricted command set before legacy station
startup; it does not initialize upstream account, remote, logbook, cluster or
hardware services. Explicit `--independent-mode` remains an upstream-compatible
station mode and is outside the connected-mode no-hardware guarantee.

Stable and beta self-update are disabled at the plugin/dependency, command and
manifest levels. Manual release navigation uses the fork repository. A future
update channel requires fork-owned signing keys and qualification; it must never
reuse upstream updater manifests or signing configuration.

## Windows build

The native DSP still compiles into the desktop binary even when the connected
mode does not initialize it. Required tools are MSYS2 UCRT64 GCC/GFortran, FFTW,
Boost, CMake, Ninja, pkg-config, make, tar, curl, unzip, nasm, Node, Rust
`1.93.1-x86_64-pc-windows-gnu`, and Tauri CLI `2.11.5`. Use explicit toolchain
arguments; do not change the user's default MSVC Rust toolchain.

Missing native dependencies can be prepared with
`bash scripts/xnexus/prepare-windows-deps.sh` in MSYS2 UCRT64. It builds pinned
libvpx 1.17.0 and stages pinned Hamlib 4.7.1; the system libvpx package is not a
substitute. Do not rerun dependency preparation merely because a new candidate
uses the existing verified cache. Schedule compilation separately from the
xLog owner's acceptance/resource window.

### Verified retained artifacts and inputs

The `.4` retention record binds all 60 independently copied files and verifies
older `.2` evidence unchanged; see [the current artifact table](WINDOWS_BUILD_1.24.1-xlog.4.md#artifacts-audit-and-retention).
The following earlier preflight and rejected-candidate evidence remain historical.

Read-only verification on 14 November 2026 used clean canonical source
`1b4ddbcbdcf763e791e2062e3092d650d65f0c15`. No compiler, Node process, application,
installer or dependency preparation ran during that check; no cache was changed.
All 13 entries in the `.2` retention receipt matched their recorded lengths and
SHA-256 hashes. The retained application and installer each have only their own
hardlink path, independent of the reusable canonical build outputs.

The retained root is
`C:\Projects\OM0RX-xLog\.build\xnexus-worktrees\release\target\retained-candidates\1.24.1-xlog.2`.
Its `retention-receipt.json` binds source
`c2c917b6f748366fbf6b3960876d9f98ba1854c9` and status
`retained-rejected-isolation-candidate`. It includes the standalone application,
installer, WebView loader, build/static-audit receipts, initial failed audit,
actual CLI process evidence and logs. Generated installer source is also retained.
Do not remove that worktree or its evidence as temporary cleanup without first
preserving and verifying this unique record.

| Retained artifact | Bytes | SHA-256 |
|---|---:|---|
| Standalone `OM0RX-xNexus.exe` | 114630669 | `CB44BA10D28FC819290165135B059788196AC4DF6DA3A8F4E1E00A94228B3BF7` |
| `.2` x64 NSIS installer | 268932507 | `3ACF27223D19C6B6E135BDD4DFE5D354076F1E9CEA6E573196A41391A04DF4D2` |

Both DeepCW model files, all eight Hamlib executables/DLLs and the libvpx archive
match the dependency hashes in the retained `.2` build receipt. All 11 base
resource mappings resolve to files, and the installer MIT notice is present for
the reviewed 12-entry distribution merge. GNU 1.93.1, UCRT64 GCC/G++/GFortran,
FFTW, CMake, Ninja, objdump, Windows SDK `mt.exe`, 7-Zip and cached NSIS tools
are present. These presence/hash checks do not replace final-source compilation.

The cached Tauri CLI 2.11.5 native module has SHA-256
`CF1426421FAFFEE1460CDE828A97D4DDEC387F276549280CC8530F7EA04124F8`.
The cached 215116512-byte Microsoft offline WebView2 installer has SHA-256
`D3CBE8FEDB104282D745FFF106993EFEAD51AE94F5595A4E75804446A974F9C3`
and a valid Microsoft Corporation Authenticode signature. Recheck the actual
inputs selected for the next package; a cache entry is not proof it was used.
The release dependency cache remains available, but new source/version require
a new optimized application build. `.2` has no reliable peak-memory series;
the separate `.4` supervisor measured 9.38 GiB. Neither permits unreviewed
resource widening or automatic retry. `npm ci` and pinned CLI installation require
registry availability; this is not an offline reproducibility claim.

### Candidate preparation and qualification sequence

1. Complete and review SDK/native/UI integration and owner lifecycle gates.
   Record exact pre-version Nexus commit `P`, paired xLog owner commit `O`,
   test receipts and remaining acceptance boundaries. Schedule final-source
   desktop checks and focused native tests within the agreed resource limits.
2. Check both platform ledgers, version receipts and release tags for unique
   next version `V`. Only then update the five version files:
   `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tauri.conf.json`,
   `ui/package.json` and `ui/package-lock.json`. Record `P` and `O` in candidate
   documentation, commit and push final source `C`. The build receipt records
   `C`; do not attempt to embed a commit's own hash in its contents.
3. Fetch and fast-forward canonical `C:\Projects\OM0RX-xNexus` only. Verify
   branch `main`, clean tracked/untracked source and freshly verified
   `origin/main` equal to full SHA `C`. Confirm all five version files use `V`
   and no prior receipt claims `V` for different source. Freeze through packaging
   and static audit. The wrapper's exact-HEAD/path checks do not replace explicit
   branch/remote verification.
4. Recheck retained `.2` evidence and native inputs. Run applicable fork,
   NSIS-template, installer-policy, distribution-merge and documentation gates
   after resource-slot release. Preflight must preserve every base resource and
   add only the installer license. Inherited `TAURI_CONFIG`,
   `NEXUS_ALLOW_MISSING_AICW` and unreviewed platform config files are rejected.
   Never delete cache/source/receipt evidence to make a preflight pass.
5. After explicit package clearance, use the supervisor reviewed for exact `C`.
   The canonical default wrapper retains its historical 4 GiB Job, 8/6 GiB
   admission and 2 GiB floors; the actual `.4` optimizer exceeded that cap.
   The [completed `.4` invocation](CANDIDATE_1.24.1-xlog.4.md#exact-build-provenance)
   used a hash-pinned external wrapper/copy of the tested drain guard with a
   12 GiB Job, 20/20 GiB admission and 6/6 GiB runtime reserves. It still called
   the unchanged canonical build helper with one Cargo/CMake job, BelowNormal
   priority, assignment before resume and a 12-hour timeout. Do not repurpose
   that exact-source proposal or rerun the completed candidate automatically.
   Preserve failed attempts and require a reviewed decision before retry or
   widening; unconfirmed termination is not resource release. Release
   optimization remains fat LTO, one codegen unit and `strip = "debuginfo"`.
6. Capture actual owned CLI process/module path and hash during the build.
   Require root termination and zero active descendants. The supervisor preserves
   before/after copies of `V.json`, `V.distribution.json` and
   `V.distribution.config.json`; the inner receipt binds configuration/template/
   guard/license hashes, assets, source and outputs. Audit unchanged canonical `C`:

   ```powershell
   pwsh -NoProfile -File scripts/xnexus/audit-windows.ps1 -Version <V> -ExpectedCommit <C-full-sha> -CliProcessReceipt <fresh-process-receipt>
   ```

   Preserve standalone and installer hashes separately: the supported Tauri
   bundle-marker transformation is not byte equality. The audit checks version,
   Common-Controls v6, imports, embedded assets/licenses, distribution policy and
   signed Microsoft WebView2 input. Retain the new artifacts and receipts
   independently before later target reuse.

The [installer hardening record](WINDOWS_DISTRIBUTION_HARDENING.md) describes
tested ownership, payload-completion, ordinary-close and data-retention guards.
Its inert executable NSIS tests do not qualify a real install, upgrade or uninstall.
Actual merged-config bundling and static audit passed for `.4`; isolated
installer/application acceptance remains required. The obsolete `WixMode`
shortcut wrappers emitted warnings, but exact-toolchain inert tests proved all
four update/no-shortcut combinations equivalent to the intended branch.
Source cleanup remains a later-version follow-up, not an artifact alteration.
The [silent installer acceptance plan](WINDOWS_INSTALLER_ACCEPTANCE_PLAN.md)
defines fresh current-user namespace checks, reviewed flags, process evidence
and synthetic file/profile retention checks without launching the application.

The script does not install or start the candidate. An unsigned installer is not
platform signing approval or proof of operating-system trust prompts. Native
startup, cross-platform installers, all radio models, rotators and on-air behavior
need separately recorded acceptance evidence. A successful check only proves the
compiled shape; no production device is opened by these checks.

Run `scripts/xnexus/test-windows-native.ps1 -ExpectedCommit <full-sha>` for the
focused hardware-free native tests. Tauri 2.6.2's build helper links its Windows
resources only into binary targets, so Cargo's library-test executable otherwise
loads Common-Controls 5.82 and fails before its harness starts: the native dialog
dependency imports `TaskDialogIndirect`, which requires Common-Controls 6. The
runner compiles with `--no-run`, preserves the generated executable, and embeds
the standard v6 dependency manifest into a **copy of the test harness only** with
Windows SDK `mt.exe`. Its receipt records the source, toolchain, manifest and SDK
tool hashes, executable hashes before/after preparation, WebView loader hash and
test result under `target/xnexus-native-tests/<source>/`. The runner never patches
an application or changes system activation settings. Passing these tests does
not establish application startup, GUI, installer or on-air acceptance.

## GitHub Actions audit

All 14 inherited workflows have an exact `github.repository == 'kd9taw/Nexus'`
condition on **every job**, including jobs with pre-existing conditions. Thus
workflow dispatch, schedule, release and push events cannot execute their jobs
in the fork. Preserve those guards when merging upstream; the fork-policy test
fails on a new unguarded inherited job.

| Inherited workflows | Reviewed effect; blocked in fork |
| --- | --- |
| ci-main, ci | Large upstream CI and dispatch of other workflows |
| release, publish | Build/sign/publish releases; upstream site/SourceForge mirrors |
| site-deploy | Upstream site deployment |
| remote-production, remote-staging | Hosted Remote/Cloudflare deployment |
| cty, fcc-states, tles | Scheduled rolling data releases with write permissions |
| fd-rules | Rules validation and publication |
| street-maps | Hosted map bucket copy/retire operations |
| release-endpoints, ship-state | Scheduled upstream endpoint/state probes |

`xnexus-ci.yml` is the only fork-executable workflow. It has read-only contents
permission, no repository secrets, no deploy/release/tag/signing/upload steps and
no schedule. It checks isolated client tests and UI on Windows/Linux/macOS, and
compiles all desktop targets on the same platforms. Compilation is not GUI,
installer, signing or hardware qualification. It installs dependencies in hosted
ephemeral runners and uses public checksum-pinned Hamlib/libvpx assets on Windows.

Repository Actions were enabled with operator approval after review. Inherited
publishing workflows remain individually disabled in addition to their repository
guards. The reviewed validation run was blocked before execution by billing;
no billing settings were changed. Local evidence does not imply hosted success.

The fork's macOS minimum is 14.0. WKWebView ignores `data_directory`; connected
mode therefore uses a persistent custom store identified by a domain-separated
hash of its canonical companion root. Different roots and independent mode do
not share drafts. The native macOS version check fails before WebView creation
on older systems rather than falling back to the default shared store. Moving
the root to a different path selects a different WebView store; keep the original
root available when recovering an unqueued draft. Native queue files remain in
the explicitly selected root. Mac build validation is separate from signed
bundle qualification and does not establish support for older macOS releases.

### Local macOS candidate route

`scripts/xnexus/build-macos.sh <full-reviewed-SHA> <candidate-version>` builds
only clean `main` matching the authorized fork's freshly fetched `origin/main`.
For a matched release, the expected SHA and version must be the same reviewed
pair used for Windows. An older in-progress platform candidate retains its
original pair and receipt; it does not qualify the newer Windows source.
`--preflight-only` checks prerequisites without building or packaging. A version
claim prevents a later source from reusing an existing candidate version.

Stage the two checksum-pinned DeepCW assets from `deepcw-model-v1` first; the
script refuses missing files and any model bypass. Prepare Hamlib from its
pinned Hamlib/libusb sources with
`NEXUS_BUILD_JOBS=2 MACOSX_DEPLOYMENT_TARGET=14.0 bash scripts/fetch-hamlib-unix.sh`.
Keep its preparation log and source revision. All four tools and their licenses
must be present, and both staged tools and the final app pass the existing
Mach-O dependency audit. No `/opt/homebrew` runtime dependency is accepted.

The first local candidate may use Homebrew FFTW, Boost and GCC. Their exact
versions, static-library architectures and hashes are recorded; that is not
qualification for every macOS 14 machine. Rust 1.93.1's actual binaries are first
on the child process PATH, and Tauri CLI 2.11.5 is downloaded by its pinned
archive hash and invoked directly by its verified absolute executable path.
Earlier helpers used Cargo subcommand lookup, which prioritizes an installed
Cargo-home CLI over PATH. The historical `.2` Mac build's actual locally installed
Tauri 2.11.5 path/hash and install metadata are recorded in a supplemental
provenance correction; its application compiler is verified as Rust 1.93.1.
Release debug information is retained explicitly for compatibility
with this Mac's linker; the receipt records this larger candidate profile.

The output is an unsigned (possibly ad-hoc signed), **not notarized** `.app.tar.gz`
archive with an artifact SHA-256 and provenance under
`target/xnexus-provenance/macos/`. There is no DMG, app launch, installation,
Developer ID signing, notarization or public upload in this route. A failed build
retains its receipt and logs. Existing `.build` validation evidence must be
inventoried before using a local Git exclusion or relocating it; never delete
it to satisfy the clean-source guard. Run `bash scripts/xnexus/build-macos.test.sh`
to exercise source/version/provenance guards without compiling a candidate.
The receipt records the actual `CFBundleVersion` and `CFBundleShortVersionString`
from the packaged app instead of assuming they preserve the prerelease string.
The built companion entry must contain its full candidate version and source
stamp; native window acceptance must still verify their visible presentation.
