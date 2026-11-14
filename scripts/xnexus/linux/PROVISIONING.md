# Linux preparation receipt — 14 November 2026

Image provisioning and locked dependency fetch both exited **0**. The first
offline contract run passed **58 SDK tests and eight fork-policy tests**, then
aborted during the full UI TypeScript check when Node's default approximately
2 GiB V8 heap was exhausted. That run is not a completed contract pass, and
reported no TypeScript source diagnostic before the heap failure. That initial
run did not reach UI tests or desktop compilation. No application launch or package is
claimed. The original failed log/receipt is retained without alteration.

A tooling follow-up sets the V8 heap to 4 GiB inside the unchanged 6 GiB container
limit. It requires 10 GiB free host RAM to start and checks host headroom every
10 seconds, stopping only the positively identified owned container below
2 GiB. Six mocked isolation/memory-guard tests pass, including positive refusal
controls for unrelated IDs, images, commands and volumes. This follow-up changes
neither runtime source nor test thresholds. The later exact-candidate result is
recorded separately below.

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

## Exact c2 contract pass

The exact `1.24.1-xlog.2` candidate
`c2c917b6f748366fbf6b3960876d9f98ba1854c9` passed the scoped Linux contract phase
using hardened harness `ae7c906e34359c8aee7c68c31818d4cc3b23a207` and the same
immutable image above. A fresh candidate-specific cache fetched locked Cargo
dependencies and 289 npm packages; neither source nor caches were copied from
the earlier e3 job. Archive SHA256:
`ae23b0195ff1e9385d3e05e7889ebf7558c42e988e24829cfa542fea15eac519`.

Both fetch and contract streamed verification against the exact committed Git
archive. All **5,445 archive-tracked entries** were checked inside the container
before and after each phase. The contract passed **eight fork-policy tests,
58 native client tests, `tsc -b`, the Vite production build, and 40 xLog UI tests
in seven files**. Vite reported its existing large-chunk warning; there was no
source-check failure. This is not the complete upstream suite or a native
desktop build, package, GUI or physical-radio qualification.

Execution used network-none, two CPUs, 6,442,450,944 bytes RAM and the same total
RAM-plus-swap cap, 256 PIDs, dropped capabilities, no-new-privileges and the 4 GiB
Node heap. Admission observed 10.15 GiB free host RAM; the guard's minimum during
execution was 10,858,905,600 bytes. No interruption or guard error occurred.
The command and effective runner exits were both zero. The container exited;
full desktop validation remains held pending resource coordination.

New raw evidence is retained under
`C:/Projects/OM0RX-xLog/.build/xnexus-linux-evidence-c2/evidence/`:

- `1794676839837347800-fetch.json` and `.log`: exact-candidate dependency fetch.
- `1794677148652152500-contract.json`: successful guarded receipt, SHA256
  `2a9b8b0b0d15a5361f0d735e65b7b835f92c8b93854c4a7f9a1573ce7562a977`.
- `1794677148652152500-contract.log`: full output, SHA256
  `25c59f31085816c7530b7a5e1c6f4367dd15be0238dc9ffbdf1d84f20ec062e7`.
- `1794677148652152500-contract-host-config.json`: actual Docker inspection of
  the exact running container, including limits, network and mounts.

The original e3 failure remains unchanged, with its separate source/cache and
receipt. The new volume is `xnexus-linux-c2c917b6f748366f-1b047bd836fc`.
