# Windows internal candidate 1.24.1-xlog.5

The unsigned Windows package built, passed the static artifact audit and was
independently retained on 15 November 2026. **Internal testing only: it was not
launched, installed or qualified as the finished product.** The user explicitly
requires the original Nexus operating interface. The companion diagnostic
dashboard is not the intended replacement UI, and this artifact will not be
upgraded into the installed application. TX remains required and unimplemented.

## Exact source and build

Canonical source was clean `75a59f786ee1f437e7ab14a147bb3ca0ceed1c49`, version
`1.24.1-xlog.5`, throughout build, audit and retention. Pre-version source was
`371474c900fd4d18fa18f831b5005f5a0fb1fe2b`; application runtime was
`9383d286c480fd94820cf63a6651ef6d6f717076`. The upstream base remains
`c88740a94172b3e78ae587d62bd0a54394d1e85f`. Later documentation does not change
which source produced these binaries.

The one actual attempt ran from 08:19:54.0879168 to 10:02:20.2208985 UTC.
Rust 1.93.1 GNU targeted `x86_64-pc-windows-gnu`, with
`radio,custom-protocol`, the unchanged optimized release/fat-LTO profile, one
Cargo/CMake job and BelowNormal priority. The frontend used Node 20.19.5,
npm 10.8.2 and `--max-old-space-size=2560`; TypeScript and Vite passed during
this build. Native release compilation finished in 93m40s; NSIS then completed.

Earlier 12 GiB/20–20 and 12 GiB/19–19 proposals were preserved. After explicit
root and owner review, the selected serial supervisor used an **11 GiB aggregate
Job cap, 18 GiB physical AND available-commit admission, and 6 GiB physical AND
commit runtime reserves**. It retained the five-second natural drain, owned-only
cleanup, deadline and fail-closed abnormal-exit8/process-memory9/job-memory10
classification. No automatic retry or cap enlargement occurred.

| Observation | Bytes / outcome |
|---|---|
| Actual admission physical / available commit | 21,205,086,208 / 19,724,648,448 |
| Verified kernel aggregate limit | 11,811,160,064 |
| Peak aggregate Job memory | 10,202,296,320 |
| Minimum physical / available commit | 13,093,904,384 / 9,702,023,168 |
| Root / effective exit | 0 / 0 |
| Final active processes | 0; root termination and cleanup confirmed |
| Natural drain | 36 ms; no forced live cleanup |
| Abnormal or memory notification | None |

The executed proposal SHA-256 is
`DF2C5A2A154066A90F479CCB48D79E016FF1D06B8256D0C2207B4F440C4CF026`;
root-review receipt is
`0EC589D80BF929C6400CFA7CF1CD8722A57A6A9EC6CB3212762EC4A4A907D8ED`.
The guard receipt is
`6F9AF406134C4BF6660AFBF15310A710D0E5F2DD5629F23B8AB348E0D5662EC8`.

## Artifact audit and retention

| Artifact | SHA-256 |
|---|---|
| Standalone executable | `FE4E8857F26EDF0543E4F0A248CEAAA2A3449AE098E80159FEBD21D0D95FC572` |
| Installer-embedded executable | `7C07CBFB4090D6E6B9C197FD0A027D3490E0364DADC2B7715E1197C4673BC601` |
| NSIS installer, 269,015,987 bytes | `003FF5E69FD513C9AEA2780708B8F5FF787F7B767F086986680917FCE324E1CF` |
| Build receipt | `7ED087BBCBD33AD11CB712461B9806ADE7801D83586BB53E003797D662BF847F` |
| Static audit receipt | `6A92A8604DD5252FB9009E372E3FA28954DCD7E4675845067D6E51C1D5545862` |
| Independent retention receipt | `613E6093434ADABC34898851835E09A7BCAE63E153A5429B654BC10556DFF81E` |

The audit verified exact product/version resources, Common Controls v6 activation,
PE imports without builder-only GNU runtime dependencies, model/Hamlib/loader
resources, COPYING/NOTICE and the installer-template license. The executable
pair differs only at the three supported `UNK` → `NSS` bundle-marker bytes;
all other bytes are identical. Executable and installer are unsigned. This is
not a complete dependency-notice or redistribution qualification: four current
direct SDK dependency notices and comprehensive notice-resource hash coverage
were identified for a separate follow-up; frozen artifacts were not altered.

The embedded offline WebView2 installer has a valid Microsoft Corporation
signature and SHA-256
`D3CBE8FEDB104282D745FFF106993EFEAD51AE94F5595A4E75804446A974F9C3`.
Its presence was inspected, not executed.

Actual live process evidence identifies the npm Tauri 2.11.5 node shim and loaded
native addon, not an assumed `cargo-tauri` executable. The addon SHA-256 is
`CF1426421FAFFEE1460CDE828A97D4DDEC387F276549280CC8530F7EA04124F8`.
Original process receipt
`746724D267110A0963F1AA75FCB157CBA39C4DBADA42B8F2A1F10806E6FC0D81`
was preserved. A reviewed lossless audit-schema projection adds top-level aliases
for the unchanged command/identity fields; its SHA-256 is
`93F12E03A70427B58C3F0C623DCAEA7F2727382D97BC4D1B70007A307C5C2102`.

The two NSIS `WixMode` warnings match the previously tested `.4` inert shortcut
branches. Both six-line generated bodies, fork template/generator and pinned
makensis/LogicLib hashes were unchanged. The carry-forward comparison receipt is
`E8BF1F915FC54C64C0830C4671F0E2617A89CD434B1ACB89BD4DDA653867C61A`.
This reuses exact prior branch-equivalence evidence, not a new installer run;
warning cleanup remains future source work.

Private evidence is under xLog's ignored
`.build/xnexus-windows-xlog5-evidence/20261115T0819538318478Z-56121e74eed940adbc3aed345f270d17`.
The independent copy is under
`.build/xnexus-worktrees/release/target/retained-candidates/1.24.1-xlog.5`:
59 files, 540,887,017 bytes, copied without overwriting and rehashed. Build/audit
processes retired and evidence was checked stable before copying. Previous `.4`
retention was fully reverified unchanged before and after; older failures remain.

## Qualification boundary

The [candidate source checks](CANDIDATE_1.24.1-xlog.5.md) include 23 focused
native tests, the actual owner cache/SDK/generated-FT4 worker chain, 59 UI tests,
full TypeScript/Vite and 28 browser viewport/theme cases. These source checks
are separate from this package audit. No `.5` native GUI, installer, upgrade,
real log mutation, paired live RX/needs, physical control or RF test was run.
Historical `.4` profile/lifecycle evidence remains tied to its own source.
The original Nexus interface, owner-mediated TX implementation and supervised
station qualification, current platform packages and distribution gates remain
unfinished; this build does not close the project.
