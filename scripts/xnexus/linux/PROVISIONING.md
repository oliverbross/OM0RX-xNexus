# Linux preparation receipt — 14 November 2026

Image provisioning and locked dependency fetch both exited **0**. The first
offline contract run passed **58 SDK tests and eight fork-policy tests**, then
aborted during the full UI TypeScript check when Node's default approximately
2 GiB V8 heap was exhausted. That run is not a completed contract pass, and
reported no TypeScript source diagnostic before the heap failure. UI tests and
desktop compilation remain unexecuted. No application launch or package is
claimed. The original failed log/receipt is retained without alteration.

A tooling follow-up sets the V8 heap to 4 GiB inside the unchanged 6 GiB container
limit. It requires 10 GiB free host RAM to start and checks host headroom every
10 seconds, stopping only the positively identified owned container below
2 GiB. Six mocked isolation/memory-guard tests pass, including positive refusal
controls for unrelated IDs, images, commands and volumes. This follow-up changes
neither runtime source nor test thresholds; its scoped retry awaits headroom.

| Input/result | Recorded value |
|---|---|
| Runtime source | `e3ae1534ade5c34dd4e2d3a81bfbe3a72be15d5c` |
| Harness source | `ce57a45242ab1ee3928be8e117a64d03e2ddd327` |
| Source archive bytes | 160,133,120 |
| Source archive SHA-256 | `2654d29a5b3415644f6459d093fcf722e08cba86e324eff0583fa305de877048` |
| Immutable built image | `sha256:1b047bd836fc8bae9c97bd0a6973630561b1a98604550a1921ef3be0322853aa` |
| Image configuration digest | `sha256:cfc614556476749fb247d3faccad7a187b1780894c97cd8302bace453226f1a1` |
| Docker server | Linux amd64, Engine 28.4.0, Docker Desktop 4.47.0 |
| Rust / Cargo | 1.93.1 / 1.93.1 |
| Node / npm | 24.22.3 / 11.19.1 |
| GCC/GFortran / CMake | 13.3.0 / 3.28.3 |
| FFTW / GTK / WebKitGTK | 3.3.10 / 3.24.41 / 2.52.6 |
| System package inventory | 554 entries |
| Package inventory SHA-256 | `60a58ed48c34d2970b3ec6b4a4adc070e9b15677e243a1b2908192f3f20bd929` |
| npm dependency fetch | 289 packages, install scripts disabled |

The exact image input manifests and verified rustup checksum are in the
Dockerfile. Ubuntu apt fetched approximately 308 MB and reported 1.36 GB
additional installed space. After preparation, Docker reported 5.918 GB total
images (including pre-existing images), a new source/cache volume of about
954 MB, and 330 MB build cache. These Docker accounting categories can share
storage; do not add them as an exact physical disk allocation.

The fetch container was limited to two CPUs and 6 GB RAM with no additional
swap, no capabilities and no devices/profile/socket mounts. It used download
networking; subsequent checks require `--network none`. At completion the fetch
container had exited, confirmed by Docker inventory with the pre-existing
PostgreSQL container as a positive control. No unrelated container or host
installation was changed.

Raw local evidence is retained under
`C:/Projects/OM0RX-xLog/.build/xnexus-linux-evidence/evidence/`:

- `1794665673217996400-image.json` and `.log`: exact build command, helper hashes,
  Docker identity, exit status and immutable image ID.
- `1794666327544973900-fetch.json` and `.log`: exact source/archive hash, image
  inspection, container constraints, dependency fetch and exit status.
- `1794666848972691600-contract.json` and `.log`: original default-heap attempt,
  exit 134 after SDK/policy passes and V8 heap exhaustion. The actual container
  network/resource settings are retained in `contract-host-config.json`.
- `packages.tsv` and `toolchain.txt`: actual installed environment inventory.

No source archive or package/cache data is committed. Preserve these local
receipts and named volume until the wider qualification is complete. A later
successful scoped test must be recorded separately; preparation alone is not
Linux application acceptance.
