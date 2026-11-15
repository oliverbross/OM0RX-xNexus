# Corresponding-source and notice evidence — incomplete delivery

This follow-up starts at `773a77c65c155a0c1c54db57120e68f2b03ffe32`, preserving
the earlier uncommitted draft and the `.4`/`.5` artifacts. It does not qualify
those artifacts for public distribution or establish complete source delivery.

## Next-candidate notice checks

On 15 November 2026, offline generation and the subsequent unchanged-file check
passed against the locked Windows GNU, macOS ARM64, Linux x86-64 and Linux ARM64
graphs, resolved by Cargo 1.93.1 on Windows. The regenerated inventory contains
531 normal-edge crates, including reqwest 0.13.5, base64 0.23.1, the serde family
1.0.229 and serde_json 1.0.154. This is metadata/license validation, not four
platform builds. The existing normal-edge scope excludes build-only and dev
dependencies; the generated header now states that boundary explicitly.

The output is 1,048,714 bytes, SHA-256
`292dc4e78161ef541e5bd1fcd367b0522a8c09cdda24549c046adaec13584b63`.
The Windows build wrapper accepts optional `-NoticeCargoHome`, passed only as
the generator's offline-only `--cargo-home` argument. Only its Cargo metadata
and tree subprocesses receive that override. The compilation environment and
existing compiler cache remain unchanged. The build receipt records the explicit
notice cache override and generator/output hashes.

The future Windows package preflight uses `--check --offline`. The existing
distribution preflight records each mapped license/notice/provenance text's
source, destination, length and SHA-256. The existing artifact audit checks the
same source inventory and exact extracted destination bytes. This includes
same-basename notices in distinct resource directories. Old build receipts
without this inventory cannot be retroactively declared covered.

Thirteen Node tests passed (eight mapped-resource and five distribution-overlay
controls): exact paths, changed/missing bytes,
misplaced duplicates, source drift, case-insensitive and binary-over-notice
collisions, unsafe mappings and absent inventories. Nine Python tests passed
for offline/locked resolution, four desktop target closures and subprocess-only
cache isolation, including missing-cache and conflicting-source failures.
The original unfiltered metadata run stopped on uncached SPIRV-only `crunchy`;
target-filtered metadata then correctly stopped on required Unix `nix 0.26.4`.
Both failed generator receipts remain preserved; neither produced a notice file.
Another generator failure identified an obsolete `objc2-osa-kit 0.3.2` fallback.
The exact key was absent from the desktop lock and all four resolved package
sets; only that fallback entry was removed. Its shared texts and hashes remain
for the twelve other Objective-C fallback entries. Missing and unused fallback
checks remain enforced. The proof SHA-256 is
`12d66f93de4bc7c3c2ac33930fa2bb2755a71911ac54b0a308369f48a6043e9f`.

Final validation receipt SHA-256:
`5434782031b30523f130cec40cdd5a20f7996c7d062c00b516b434caf7fb07da`;
guard SHA-256:
`1ccf8b3485b279f9d2847e9b4f210611c416f5aef8482678c9681f76daa79a64`.
All 2,221 pinned source/license/tool inputs remained unchanged. The 1 GiB job
peaked at 211,050,496 bytes, exited zero, drained naturally in 16 ms and left no
owned processes. No Rust build, installer or application ran.

## Isolated acquisition provenance

The global Windows Cargo cache lacked required Unix archives; the retained
Linux-cache manifest was available, but Docker was off and was left off. A new
task-owned Cargo home was seeded from 664 checksum-verified locked archives and
the pinned public crates.io sparse index, without copying credentials or global
configuration. Two cold metadata attempts timed out after 30 and 180 seconds;
their failed receipts remain preserved. The reviewed continuation pinned all
19,533 existing cache files and preserved the 425 already-complete package trees.

Only these five archives were requested by actual offline target-metadata
failures and fetched from the registry's declared
`https://static.crates.io/crates/<name>/<name>-<version>.crate` URL:

| Archive | Cargo.lock SHA-256 |
|---|---|
| nix 0.26.4 | `598beaf3cc6fdd9a5dfb1630c2800c7acd31df7aaf0f565796fba2b53ca1af1b` |
| xattr 1.6.1 | `32e45ad4206f6d2479085147f02bc2ef834ac85886624a23575ae137c8aa8156` |
| libudev 0.3.0 | `78b324152da65df7bb95acfcaab55e3097ceaab02fb19b228a9eb74d55f135e0` |
| libudev-sys 0.1.4 | `3c8469b4a23b962c1396b9b451dda50ef5b283e8dd309d69033475fa9b334324` |
| unescaper 0.1.8 | `4064ed685c487dbc25bd3f0e9548f2e34bab9d18cefc700f9ec2dba74ba1138e` |

Each download was checksum-verified before publication, with no redirects or
download retries. Bounds were 16 MiB compressed, 128 MiB expanded and 20,000 tar
members per archive; links, special entries and unsafe paths were rejected.
The four metadata probes remained locked/offline. No excluded-target crate was
hardcoded away; `crunchy` and the two other unrequested allowlisted archives were
not fetched. Global cache/index donors and completed package contents remained
unchanged. Acquisition receipt SHA-256:
`d24479417d598ce5aab7c95a91b8db15534c248055b5d1a063535c8b01756dd0`;
guard SHA-256:
`f389ac18f99388caef361ac959dd35b4fa856d5ec4a2b97586c50d2e1c593d64`
(exit zero, 21 ms natural drain, no owned processes or forced cleanup).

The private cache/evidence is retained beneath the `notice-qualified` worktree's
`target/notice-validation`; neither cache contents nor private acceptance logs
are Git deliverables. A future build must bind this cache provenance explicitly
or prepare another verified complete cache, not silently fetch during packaging.

## Historical resource-map compatibility

The helper also verified all 24 mapped notices in each retained `.4` and `.5`
extracted payload against their actual distribution maps. Tracked resources
were reconstructed from each exact historical Git commit; ignored inputs were
explicitly identified as payload-derived, not independent source evidence.
All 60 `.4` and 59 `.5` retained artifact/provenance files were hash-checked before
and after. This compatibility check does not retrofit a pre-build inventory into
an old receipt. Its receipt SHA-256 is
`80da6284c02d3cf152179c06aee8a3a3a601189ec45d5d0f415d7a17e263c3d7`;
guard SHA-256 is
`918aac19b9407ae3bef8db485675427d6d991d3972da42d6343bf9beabbf5ae0`
(exit zero, no retained processes or forced cleanup). The initial disposable
fixture import-path failure is preserved separately.

No new installer audit, package or platform run is claimed by these checks.

## Material inventory still required

| Material | Existing evidence | Missing delivery/provenance evidence |
|---|---|---|
| Fork and vendored WSJT-X | Git source/history, COPYING, NOTICE, vendored modem source and provenance | Exact future candidate source archive/hash tied to completed artifact and audit receipts; Linux validation archives alone are not that deliverable |
| Rust/UI dependencies | Lockfiles, verified normal-edge Rust notice generation and exact bundled notice maps | Exact source/material delivery inventory beyond notice text, including build-only dependencies; UI source/material inventory |
| DeepCW | Pinned distributed ONNX and metadata hashes; AGPL text; transformation example | Exact original upstream revision/model hash and pinned onnx/onnxsim transformation versions; source-to-distributed-model derivation evidence |
| Hamlib Windows package and auxiliary DLLs | Pinned 4.7.1 archive fetch recipe, staged binary hashes and bundled license files | Bind actual staged members to retained archive provenance and identify corresponding source/build material for each delivered component |
| libvpx | Build receipt records static archive and source hash; bundled stream notice | Bind retained source/configuration/build recipe to final candidate inventory |
| FFTW, compiler runtimes and other native inputs | Build recipe/import audit and selected tool versions | Exact native package/source/configuration inventory for linked inputs; absence of runtime DLL imports does not establish source coverage |

The smallest later source-delivery step is an offline manifest plus exact-commit
`git archive`, with a sorted tracked-file inventory and explicit external-input
records. Exclude profiles, credentials, build caches and private acceptance logs.
Record each external item as retained source, binary-only evidence, or unresolved;
missing inputs keep the result **incomplete**, never a fabricated complete bundle.
This draft neither downloads those inputs nor reaches a legal conclusion.

Corresponding-source inventory/material preparation remains separate from this
notice correction. Changed notice bytes require a future source/version candidate,
not edits to retained `.4`/`.5` artifacts. The next original-interface candidate
remains private qualification until its remaining distribution gates are met.
