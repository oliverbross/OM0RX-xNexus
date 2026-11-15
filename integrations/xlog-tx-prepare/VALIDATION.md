# Pure period-preparation validation

This is an unwired content adapter, not transmission, timing, route, radio,
sequencer or release-watchdog qualification.

Final run: `target/tx-prepare-check/attempt-7b73971bed8c43f4b03f5dc06cf53a6e`
in the isolated `codex/tx-period-preparation` worktree. Receipt SHA256
`F93AB9E4A548ADF6593FA5E3653B9E3A07FDE7239DFA8EE79AC6F6689CC1D79E`.
Six tests passed; pinned rustfmt and locked offline no-deps Cargo metadata passed.
Metadata SHA256 `3C23EFC679C50379E94C4EF17984560505F0B774E9F1016B11A433BC80EF6EFE`.

The test executable freshly compiled this crate's actual source with GNU Rust
1.93.1. Its modes/FT8/FT4/SHA2 libraries came from the retained passed Nexus
`43e305f1d65ec27f6f72255a14f2eb0775e9b349` dependency inventory. Before/after
hashes pinned the actual rlibs, compiler, source and lock. A git comparison
confirmed modes, FT8, FT4, tempo-core, tempo-codec, libtempo and root Cargo.lock
were unchanged between that dependency source and this branch base75a59.

The six tests compare pure tone and sample output against the independent
vendored FT8/FT4 reference for CQ/grid/report/acknowledgement/signoff forms,
preserve lead-in/full-period padding, reject unsupported formats and damaged
waveforms, verify exact-byte immutable digests, and demonstrate the actual
packer's lossy free-text behavior that the adapter explicitly excludes.
No `Mode::encode`, fallback state, device or transmit operation is invoked.

Resource policy: 2 GiB job memory, one job, BelowNormal, 300 seconds, admission
8 GiB physical/6 GiB available commit, runtime floors2/2 GiB, natural descendant
drain3 seconds. Final exit0 was natural; root exited, final active processes0,
cleanup confirmed, peak560,852,992 bytes, no memory/abnormal notifications.

Final Rust and lock SHA256:

| File | SHA256 |
|---|---|
| src/lib.rs | D7F2B5D325A958455FBEF86D5EAB262C0370E2AB1F616D40DCBD2DD406B6F53D |
| src/tests.rs | 77411CDF16EF511384B235C9210A6A1AC9111F357B72640F055DD365C5FCA20F |
| Cargo.lock | FB616C5DEB94D3E85AD3A075FE3963CE445E2C4E599EBDCB9BF75579C3FC82F7 |

Earlier evidence is retained rather than overwritten:

- `attempt-d0b04544dc594b1b935c4e3836fe296f`:5/6 tests passed; the unsupported
  character witness incorrectly compared trailing `HELLO@` with `HELLO`.
  The packer right-aligns before substituting unsupported characters. The
  corrected internal `HELLO@WORLD`/`HELLO WORLD` collision passed. Runtime was
  unchanged; the independent 13-character truncation witness already passed.
- `attempt-9d071aa5f5334dbcad53c69d8f89604c`:6/6 passed before formatting.
- `attempt-7b72d6cbee594b4888f71fc9c5879a84`:offline lock generation and metadata
  passed; format check failed, and the runner's unchanged-source assertion also
  noticed its planned new lockfile. Final run used the pre-existing pinned lock,
  without relaxing the source comparison. Old tested source copies remain.

A Cargo dependency build, Clippy, original sequencer integration, owner-approved
output format/route, operation journal, slot admission and independent observed
release remain separate gates. No public release or RF safety claim follows.
