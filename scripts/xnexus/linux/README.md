# Isolated Linux qualification

This is a **scoped validation harness**. Image provisioning, dependency fetch and
the exact c2 SDK/UI contract phase have passed; native desktop compilation has
not yet been attempted. See
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
verification. Native desktop compilation, GUI, package and physical-radio
validation remain unexecuted until their own receipts establish otherwise.

Provenance-hardening verification used the existing immutable image
`sha256:1b047bd836fc8bae9c97bd0a6973630561b1a98604550a1921ef3be0322853aa`
with no network, one CPU, 256 MiB/no extra swap, no capabilities and a read-only
tool mount. All **11 tooling tests passed**, including Linux symlink targets and
the actual phase wrapper with stub compilers. The phase fixture requires
`XNEXUS_TOOLING_FIXTURE=disposable-container`; never enable it on a host or a
retained validation volume. Its mutations are restricted to that disposable
container's scratch filesystem. These are tooling tests, not application
contract/desktop qualification. Resource thresholds are unchanged.
