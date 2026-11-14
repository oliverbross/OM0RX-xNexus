# OM0RX-xNexus release readiness

Candidate `1.24.1-xlog.1` is reserved for the first integrated fork candidate. The
pre-version source is `9252c72336c6fa824a4e928820ed90171584669e`. This reservation
is not a claim that a binary has been built, signed, installed, or qualified.
The final source SHA must be the clean canonical checkout HEAD recorded by
`scripts/xnexus/build-windows.ps1 -ExpectedCommit <full-sha>` when packaging.
Changing packaged source requires a new visible version, including failed
qualification candidates that were already distributed or installed.

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
Boost, CMake, Ninja, pkg-config, make, tar, curl, nasm, Node, Rust
`1.93.1-x86_64-pc-windows-gnu`, and Tauri CLI `2.11.5`. Use explicit toolchain
arguments; do not change the user's default MSVC Rust toolchain.

1. Install missing MSYS2 UCRT64 packages. Run
   `bash scripts/xnexus/prepare-windows-deps.sh` from that environment. It builds
   checksum-pinned libvpx 1.17.0 and stages checksum-pinned Hamlib 4.7.1. The system
   libvpx package is not a substitute for this pinned build.
2. Run `scripts/xnexus/build-windows.ps1 -CheckOnly` for desktop compilation.
3. Integrate and commit all source into canonical `C:\Projects\OM0RX-xNexus`,
   advance/reserve the version, run proportionate tests, push, and verify clean HEAD.
4. Package with the full expected SHA. The script verifies both DeepCW resources,
   records version/SHA/toolchain/model hashes before building and installer SHA256
   afterward under ignored `target/xnexus-provenance`. Preserve that receipt with
   the release. It refuses version reuse for different recorded source.

The script does not install or start the candidate. An unsigned installer is not
platform signing approval or proof of operating-system trust prompts. Native
startup, cross-platform installers, all radio models, rotators and on-air behavior
need separately recorded acceptance evidence. A successful check only proves the
compiled shape; no production device is opened by these checks.

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

Repository Actions were disabled with user approval pending this review. The
owner may enable Actions after these guards and the dedicated workflow land;
do not enable an inherited publishing workflow or remove its repository guard.
Actual hosted results must be recorded separately, never inferred from this file.

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
The expected SHA and version must be the same reviewed pair used for Windows.
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
archive hash. Release debug information is retained explicitly for compatibility
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
