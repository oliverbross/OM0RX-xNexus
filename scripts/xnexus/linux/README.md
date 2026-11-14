# Isolated Linux qualification

This is a **prepared, scoped validation harness**, not evidence that Linux
compilation has already passed. It does not package, install or launch xNexus.
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
Only loopback within the container is available. The UI's dependency rebuild
runs offline. A missing native download fails; do not reconnect the test phase
to the network to conceal it. Add a separately pinned fetch if necessary.

The image build itself uses Docker's configured builder resources; it has no
application compile workload. Actual checks are explicitly resource-limited.
The source-only scripts were checked with Python unit tests, a dry plan, Bash
syntax validation and the fork policy tests. Actual container dependency
installation, Linux compilation, GUI, package and physical-radio validation
remain unexecuted until receipts establish otherwise.
