# Isolated Linux qualification

This is a **scoped validation harness**. Image provisioning, dependency fetch and
the exact c2 and d206 SDK/UI contract phases have passed. Exact d206 `.3`
native desktop all-targets compilation also passed with the reviewed reduced
resource profile. See
[the retained preparation record](PROVISIONING.md). It does not package, install or launch xNexus.
Keep the fork's canonical package guards and inherited publishing restrictions.

The recipe follows `.github/workflows/xnexus-ci.yml`: Ubuntu 24.04, Rust 1.93.1,
Node 24, the contract/UI checks and desktop `--features radio --all-targets`
check. `scripts/gates --list` records the upstream gate inventory. These selected
checks do **not** constitute the complete upstream CI matrix. No Hamlib package
is installed; hardware-dependent upstream tests are not claimed. AI-CW weights
are absent; the debug compile can warn about them, but there is no release build
or missing-model override. No xLog application is built in this container.

## Pinned inputs and retained receipts

The Dockerfile pins official Linux amd64 image manifests, resolved on
2026-11-14, and the exact rustup installer checksum from `release.yml`.
Signed Ubuntu apt packages still resolve from rolling repositories. Each
container writes its complete package and toolchain inventory; retain the
built immutable image ID for an exact reuse of that environment. This is a
reproducible procedure with recorded resolved inputs, not a claim that a later
apt resolution produces identical bytes.

The host runner accepts only this fork's origin and a full committed source SHA.
`git archive` exports tracked bytes, preserving the source snapshot without
mounting Git credentials, Windows build outputs, station profiles or a Docker
socket. Every phase records the source/archive hash, helper hashes, image ID,
Docker version, exact command, full log and exit code in the chosen job folder.
The named cache volume is specific to both source and immutable image ID.
Nothing is automatically pruned.

An explicitly selected `seed` phase can initialize a **new source/image volume**
from a retained qualified donor. It never reuses the donor volume as the new
source. See the bounded procedure below; old source and qualification remain
independent evidence.

The source label is not accepted as proof: before every container phase, the
host streams a fresh `git archive` for the exact commit and compares its SHA256
with the retained archive. A mismatch is refused without replacing either
archive or cache. Inside the container, `verify_source.py` checks every
archive-tracked regular file's bytes and executable bits, each symlink's exact
target, and real parent directories before the phase and again on exit.
Generated/untracked build outputs are allowed. A tracked edit made by a build
command prevents a successful phase even if that command returned zero; the
changed source remains available for diagnosis. An interrupted process cannot
claim the missing post-phase proof.

## Resource budget and execution

Inspection on the available Windows Docker Desktop Linux engine 28.4.0 found
approximately 110 GB host free, 448 MB existing images and 3.55 GB existing
volume data. No builder image was installed. These are point-in-time inventory
observations, not a space reservation. Expected initial downloads are roughly
1–3 GB and peak image/source/dependency/target storage 10–25 GB; these are
estimates until actual preparation. Allow at least 30 GB free and inspect Docker
VM capacity separately. Compilation can take tens of minutes to multiple hours
on two CPUs. Coordinate the timing with native release work before execution.

Default invocation only prints a plan and reads Git identity:

```powershell
$linuxRepo = 'C:/path/to/isolated/xNexus'
$linuxSource = 'e3ae1534ade5c34dd4e2d3a81bfbe3a72be15d5c'
$linuxJob = 'C:/path/to/disposable-linux-evidence'
python scripts/xnexus/linux/validate.py --repo $linuxRepo --source $linuxSource --job $linuxJob
```

After the coordinated resource window opens, run each phase separately, inspect
its exit code/receipt, and stop on failure:

```powershell
python scripts/xnexus/linux/validate.py --repo $linuxRepo --source $linuxSource --job $linuxJob --phase image --execute
python scripts/xnexus/linux/validate.py --repo $linuxRepo --source $linuxSource --job $linuxJob --phase fetch --execute
python scripts/xnexus/linux/validate.py --repo $linuxRepo --source $linuxSource --job $linuxJob --phase contract --execute
python scripts/xnexus/linux/validate.py --repo $linuxRepo --source $linuxSource --job $linuxJob --phase desktop --execute
```

`image` installs tools **inside a new image**, without host installation or
daemon changes. It runs apt/rustup provisioning, not application compilation.
`fetch` downloads locked Cargo/npm dependencies without npm install scripts;
it has ordinary container networking but no ports, devices, credentials or
profile mounts. `contract` and `desktop` run with `--network none`, two CPUs,
6 GB RAM/no extra swap, 256 PIDs, no Linux capabilities and no privilege gain.
The runner requires at least 10 GiB host available RAM before starting either
check phase. Every 10 seconds it checks host memory; below 2 GiB, it verifies
the exact owned container ID, image, command and cache volume before requesting
ordinary Docker stop. Such a run is recorded as interrupted, not a source
failure or pass. It never pauses a container to address memory pressure, because
pausing retains its memory. No unrelated container is stopped.
Any memory-guard error, uncompleted guard, or recorded interruption makes the
runner's effective exit nonzero even if Docker returned zero. Receipts retain
the actual `process_exit_code` separately from the effective `exit_code` and
`result`; a guard failure is not reported as a source-test failure or pass.

The separately selected `--resource-profile desktop-reduced` is valid **only**
for `--phase desktop`. It uses 4 GiB RAM with the same RAM-plus-swap cap,
one Cargo/CMake build job and the unchanged two-CPU maximum. Admission requires
both at least 8 GiB physical RAM and 6 GiB available Windows system commit;
unknown commit availability refuses admission. Its ten-second guard stops only
the verified owned container if either available physical RAM or commit falls
below 2 GiB. Standard fetch/contract policy and prior receipts stay unchanged.
A reduced-profile OOM is preserved as failure; there is no automatic widening,
retry, cache dropping, Docker/WSL reconfiguration or unrelated process action.
Receipts record the profile, admission readings, both running minima, helper
hashes, harness commit and dirty status. Preserve actual Docker inspection too.
The reduced profile requires separate resource-window coordination before use.

## Offline cache seed and reduced contract

The additional `contract-reduced` profile supports only `seed` and `contract`.
It uses 4 GiB RAM/no additional swap, one Cargo/CMake/test job, one UI worker,
two CPUs, a 2,560 MiB Node heap, no network, and the same 8 GiB physical/6 GiB
commit admission plus 2 GiB running floors as `desktop-reduced`. Existing
standard-profile behavior and retained results are unchanged. A reduced Linux
frontend pass is not inferred from Windows heap measurements.

Prepare a fresh job outside source; copy only the retained immutable `image.id`
into it after checking the actual image ID. No image rebuild or fetch is needed
when the required dependencies are already cached. The exact source commit and
source archive are independently verified by the runner, as in other phases.

```powershell
$linuxRepo = 'C:/Projects/OM0RX-xLog/.build/xnexus-worktrees/linux-validation'
$linuxSource = '0d01248b0504ae77f6f122fdada1b21e4a61806b'
$linuxJob = 'C:/Projects/OM0RX-xLog/.build/xnexus-linux-evidence-0d01248b'
$donorJob = 'C:/Projects/OM0RX-xLog/.build/xnexus-linux-evidence-d206'
$donorReceipt = "$donorJob/evidence/1794679901010293000-desktop.json"
# After coordinated release, fresh admission and reviewed image.id preparation:
python scripts/xnexus/linux/validate.py --repo $linuxRepo --source $linuxSource --job $linuxJob --phase seed --resource-profile contract-reduced --donor-job $donorJob --donor-receipt $donorReceipt --execute
python scripts/xnexus/linux/validate.py --repo $linuxRepo --source $linuxSource --job $linuxJob --phase contract --resource-profile contract-reduced --execute
python scripts/xnexus/linux/validate.py --repo $linuxRepo --source $linuxSource --job $linuxJob --phase desktop --resource-profile desktop-reduced --execute
```

Run separately and inspect each receipt before continuing. The seed requires an
exact successful guarded desktop receipt for the same immutable image. It
verifies the donor Git archive and all four lockfiles plus application/path-crate
manifests against the new archive; unequal inputs refuse reuse. Both archives,
the donor receipt and dependency hashes are recorded. The donor volume and input
archive are mounted **read-only**. No source, prior `ui/dist`, node_modules,
TypeScript build state, credentials or qualification receipt is copied as new
evidence. Only `cargo/registry`, optional `cargo/git`, `target` and the npm
content cache are copied. File bytes/modes and link targets are compared before
and after copying and retained in manifests; the old donor remains intact.
Copying compiled dependencies saves work but does not establish new-source
correctness. Allow disk capacity for an independent complete cache copy.

The destination must be empty. A failed partial copy remains marked and unusable;
there is no automatic overwrite, recursive cleanup or retry. Contract extracts
fresh source and installs locked npm dependencies offline from the copied cache,
then rebuilds/tests the new frontend. Missing cache content fails without enabling
network. Cargo retains its ordinary dependency fingerprint checks. A seeded
desktop check additionally records Cargo JSON and requires a successful
`fresh: false` compiler artifact for the actual
`/work/source/src-tauri/src/lib.rs` library; an all-fresh cache result cannot
claim the updated app was compiled. Keep that JSON and its verification receipt,
alongside before/after tracked-source checks and actual container inspection.

This tooling change has host Python fixture coverage. The disposable Linux shell
fixture, real seed and updated-source checks still require their own execution
receipts; creating a seed is not a contract or desktop pass.

`NODE_OPTIONS=--max-old-space-size=4096` gives TypeScript a 4 GiB V8 heap inside
the unchanged 6 GiB container budget. The initial default-heap run passed all
58 SDK and eight policy tests, then exhausted Node's approximately 2 GiB heap
during `tsc -b`. Preserve that failed receipt; raising the per-process heap
within the existing container bound does not change source or test thresholds.
Only loopback within the container is available. The UI's dependency rebuild
runs offline. A missing native download fails; do not reconnect the test phase
to the network to conceal it. Add a separately pinned fetch if necessary.

The image build itself uses Docker's configured builder resources; it has no
application compile workload. Actual checks are explicitly resource-limited.
The scripts were checked with Python unit tests, a dry plan, Bash syntax
validation and the fork policy tests. Image provisioning and locked dependency
fetch subsequently passed. The later exact c2 contract passed SDK/policy tests,
the TypeScript/Vite build and 40 xLog UI tests with before/after source integrity
verification. Exact d206 subsequently passed those checks and the native Linux
`--features radio --all-targets` compile check. GUI, package and physical-radio
validation remain unexecuted by this harness.

Provenance-hardening verification used the existing immutable image
`sha256:1b047bd836fc8bae9c97bd0a6973630561b1a98604550a1921ef3be0322853aa`
with no network, one CPU, 256 MiB/no extra swap, no capabilities and a read-only
tool mount. All **11 tooling tests passed**, including Linux symlink targets and
the actual phase wrapper with stub compilers. The phase fixture requires
`XNEXUS_TOOLING_FIXTURE=disposable-container`; never enable it on a host or a
retained validation volume. Its mutations are restricted to that disposable
container's scratch filesystem. These are tooling tests, not application
contract/desktop qualification. Resource thresholds are unchanged.
