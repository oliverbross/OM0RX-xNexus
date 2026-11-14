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


## Exact d206 `.3` contract pass

Candidate `1.24.1-xlog.3`, source
`d206b880216543af2142e600ea6b8e5ac6bdd1b2`, passed fetch and contract using the
same immutable image and hardened executable harness (documentation head
`2dc9a8180f6a73be866a5eb0596b06cec9fe7dec`). Its fresh named volume is
`xnexus-linux-d206b880216543af-1b047bd836fc`; no c2/e3 source, dependency cache or
frontend assets were copied. Archive SHA256:
`e74906dfeb3118fe10048f3b16ee36e36e7ae154f8596ca6eff8b6a6fbc3b21c`.

Fetch installed 289 npm packages with scripts disabled and locked Cargo
dependencies. Contract passed **eight fork-policy tests, 58 SDK tests,
TypeScript/Vite production frontend build and 40 UI tests in seven files**.
The generated `ui/dist` therefore belongs to this exact candidate. Both phases
verified all **5,472 archive entries** before and after execution; generated
outputs are permitted, tracked source changes are not. Vite reported only its
large-chunk warning. Guarded contract process/effective exit codes are zero,
with no interruption or memory-guard error. Its minimum observed available host
RAM was 9,034,883,072 bytes, above the unchanged 2 GiB running guard floor.

Raw evidence remains in
`C:/Projects/OM0RX-xLog/.build/xnexus-linux-evidence-d206/evidence/`:

- `1794678356636813900-fetch.json` and `.log`.
- `1794678512647349100-contract.json`, SHA256
  `5fb7f9f43003b99e434c0fcccfc7074ffaccbccf31c6121b73ca3d676407ddfb`.
- `1794678512647349100-contract.log`, SHA256
  `80a073182b30254d44c286212c4d366bd9f5efb50557d31f1fb35aa8b6a78321`.
- `1794678512647349100-contract-host-config.json`: actual running container
  inspection proving network-none, two CPUs, 6 GiB RAM/no additional swap,
  256 PIDs, capability drop and restricted mounts.

The first desktop invocation was refused before container creation because
available host RAM was 9.14 GiB, below the unchanged 10 GiB admission threshold.
This is an admission hold, not a desktop source-test failure. No native desktop
build, package, GUI or radio acceptance is claimed by these contract receipts.


## Reduced desktop profile preparation

After the `.3` contract exited, observed physical headroom remained below
10 GiB for more than five minutes (8.81–9.99 GiB across discrete observations,
not continuous monitoring). The 17:56:42Z reading showed 8.81 GiB physical and
6.06 GiB Windows available commit. Read-only inspection of the existing
`vmmem` process found 7.92 GiB working set and 14.12 GiB private committed bytes;
the guest reported 14.25 GiB available, including 9.93 GiB file cache. These are
separate host/guest measurements, not proof of additional Windows capacity.
The retained evidence includes `desktop-admission-observations.jsonl` and
`desktop-admission-vm-state.json`; no cache/daemon/process changes were made.

A separate desktop-only profile lowers the workload to a 4 GiB cgroup cap and
one build job, instead of admitting the original 6 GiB workload with less
headroom. It requires 8 GiB physical plus 6 GiB available Windows commit and
monitors both against a 2 GiB running floor. Existing standard-profile evidence
and behavior remain unchanged. All **15 tooling tests passed** in the pinned
image with no network, one CPU and 256 MiB RAM/no extra swap, including exact
admission boundaries, commit-pressure/foreign-container refusal, guard failure,
actual shell one-job selection and source integrity. An initial syntax check
caught Windows shell line endings during preparation; they were corrected
before the successful complete test run. No desktop compiler was launched by
these stub-based tooling tests. Output: `desktop-reduced-tooling-tests.log` in
the d206 evidence directory. Actual desktop execution awaits profile review
and release of the other owner's short focused test window.


## Exact d206 `.3` native desktop pass

The same exact `d206b880216543af2142e600ea6b8e5ac6bdd1b2` candidate and fresh
candidate cache subsequently passed:

```text
cargo check --locked --offline --manifest-path src-tauri/Cargo.toml --features radio --all-targets
```

The reviewed clean harness was
`3cee2ba5da518acc12cb7be37e934f0d5b5426af`. Invocation selected
`--phase desktop --resource-profile desktop-reduced --execute` with the existing
d206 job directory. The single-job dev-profile check finished in **19m 36s**.
All **5,472 archive entries** passed verification before and after the phase.
The exact source-matched frontend assets came from the preceding d206 contract
phase; no source/archive/cache substitution occurred. The expected build warning
states that AI CW model weights are absent. There was no missing-model override.

Actual Docker inspection confirmed network-none, two CPUs maximum, **4 GiB RAM
and the same RAM-plus-swap limit**, 256 PIDs, all capabilities dropped and
no-new-privileges. The explicit desktop-reduced environment selects one
Cargo/CMake job. Admission measured 10,991,316,992 bytes physical free and
8,089,931,776 bytes available Windows commit. Running minima were respectively
7,170,748,416 bytes and 5,177,794,560 bytes, both above the 2 GiB floors.
No memory-guard error or interruption occurred; process and effective exits
were zero with result `passed`.

Retained d206 evidence:

- `1794679901010293000-desktop.json`, SHA256
  `2bc258e797d298c1552b7b259ea045c3a142cbe0df7bfc72cc645862fdafffac`.
- `1794679901010293000-desktop.log`, SHA256
  `70ff67ea70a94684accb79abc34c7e756dcace46ccab5bdb11634805960b5122`.
- `1794679901010293000-desktop-host-config.json`: actual running container ID,
  image, command, profile environment, limits, network and mounts.

The validation container exited; Docker's remaining running-container inventory
contained only the pre-existing healthy `om0rx-postgres` positive control. Host
C: had 79.71 GiB free at completion. This establishes a Linux native compile
check in addition to the scoped SDK/UI contract. It does **not** establish a
Linux package, GUI/runtime acceptance, complete upstream test matrix or any
physical-radio behavior. No application/device was launched and no unrelated
container, VM configuration or production xLog process was changed.

## Exact 0d source: tooling and cache seed passed; contract admission held

The updated application source is
`0d01248b0504ae77f6f122fdada1b21e4a61806b`; validation tooling is
`93a6cabaeccacf5e72b767829daa677ff8db9b8d`. Its independently exported
160,768,000-byte Git archive has SHA256
`3d831e29e37d0f5cea20b1fe3ff7d210e249fb1edb819b0f05351af8344ffad5`.
Fresh streamed Git archive verification passed. The immutable image remains
`sha256:1b047bd836fc8bae9c97bd0a6973630561b1a98604550a1921ef3be0322853aa`.
All **37 dependency inputs** (four lockfiles and 33 application/path-crate
manifests) match the independently verified d206 donor archive.

The original 8 GiB physical admission refused at 7.69 GiB before container
creation. A separately reviewed explicit `contract-low-headroom` profile then
required fresh **7 GiB physical and 8 GiB available commit**, while retaining
4 GiB RAM/no additional swap, two CPUs, one build job, network-none and 2 GiB
physical/commit runtime floors. It is not an automatic fallback or cap increase.
The standard and earlier reduced-profile evidence above remain unchanged.

**Disposable fixture pass:** all 25 Python tooling tests passed in the real
Linux image, including the actual shell wrapper and symlink checks that the
Windows run could not exercise. Admission measured 8,394,575,872 bytes physical
and 10,078,478,336 bytes available commit. Only the tooling directory was mounted,
read-only; no station source/cache/device/socket was mounted. Actual Docker
inspection records the 4 GiB RAM=swap cap, two CPUs, network-none and capability
drop. The fixture exited zero without OOM; its exact exited container was removed
after inspection, with removal exit zero. This was tooling execution, not an app
compiler or application test run.

**Seed pass:** the new volume
`xnexus-linux-0d01248b0504ae77-1b047bd836fc` was initialized with the d206 donor
volume and donor archive mounted read-only. Seed admission measured
8,321,105,920 bytes physical and 9,963,888,640 bytes available commit. The only
copied paths were Cargo registry (29,947 manifest entries), compiled target cache
(9,446 entries) and npm content cache (1,517 entries); the optional Cargo git
cache was absent. No prior application source, UI output, TypeScript state or credentials were
copied. File bytes/modes and link targets matched the donor before and after copy,
including the final donor comparison. Archive/source verification and the donor
receipt comparison also passed. Process and guarded effective exits were zero,
without interruption or guard/donor-integrity error. Running physical/commit
minima were 6,656,573,440 and 9,486,462,976 bytes, above the 2 GiB floors. The
actual mount/resource inspection is retained. The seed container exited normally.

**Separate contract hold:** after the seed exited, the next contract invocation
performed fresh admission and found **6.35 GiB physical free, below 7 GiB**.
It stopped before creating a container. Exact-byte/commit readings were not
printed by that refused call, so the hold receipt does not invent them. No
automatic retry, threshold change, cache dropping or application closing followed.
The newly seeded volume and all evidence are preserved. SDK/UI and desktop checks
for exact 0d have **not run**; no new-source compile, package, GUI or radio
qualification is claimed. A future resumed phase still requires explicit resource
coordination, fresh admission and actual non-fresh application compiler evidence.

Raw evidence directory:
`C:/Projects/OM0RX-xLog/.build/xnexus-linux-evidence-0d01248b/evidence/`.

| Evidence | SHA256 |
|---|---|
| `20261114T210838Z-tooling-fixture.json` | `c43d4bbe7ce1d0f65ad611fa56b5843e5db70aa6a7c4ce20b067920faf433838` |
| `20261114T210838Z-tooling-fixture.log` | `117fb0bff810f151b7768e1342b2a3399102f252c379ee96240795767b433892` |
| Retained local fixture supervisor `run-tooling-fixture.py` | `fe60a4966264d62e6eeaa94c721bb166ba5abf9d3460cd848dfc636aaf409b05` |
| `1794690544229301100-seed.json` | `ea6f4639323fc17d476a4663991622f57ac6b6d8a16b3597b01b49a9a969255d` |
| `1794690544229301100-seed-host-config.json` | `eeb959fa7a9e2573054821c0e3ee66a9b7364a4dc1c1f7468898917ce79e65bd` |
| `seed/seed-copy.json` | `4142ab1365fd647830053fc5c94b9003f8c93bc060e9ffa60b56f1bb7df45552` |
| `contract-admission-hold-20261114T211346Z.json` | `981c18c18fe81b9e8529b0a07393cd07885597170973f28e86be8c7fa39a7a56` |

The seed-copy record binds each complete cache manifest. Host rechecks after
completion confirmed the original d206 archive remains
`e74906dfeb3118fe10048f3b16ee36e36e7ae154f8596ca6eff8b6a6fbc3b21c`, its desktop
receipt remains `2bc258e797d298c1552b7b259ea045c3a142cbe0df7bfc72cc645862fdafffac`,
and its log remains `70ff67ea70a94684accb79abc34c7e756dcace46ccab5bdb11634805960b5122`.
Final read-only running-container inventory contained only the unrelated
`871ae0b9051d`, image `postgres:16-alpine`, name `om0rx-postgres`. It was untouched.
No xNexus validation/compiler/container remains running from these attempts.
