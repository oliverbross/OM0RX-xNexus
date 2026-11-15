# Offline final-output preparation candidate

**Coefficient numerical analysis, twelve actual Rust module tests and fourteen
fixed-1-kHz corpus/reference cases passed. Owner acceptance remains pending.** No application member, backend,
stream, radio, clock, authorization, journal or version change is introduced.
The existing 12 kHz mono v1 format and retained 14-case corpus are unchanged.

## Boundary and proposed final schema

`BoundSource::bind` requires the actual privately constructed `PreparedPeriod`
and strictly checks every original v1 manifest field against its getters. The
exact raw manifest is retained, including whitespace. Its whole-body identity
uses the existing `xnexus-prepared-corpus-body-v1\0` domain, little-endian u64
manifest byte count, raw manifest, u64 PCM byte count and actual PCM. There is no
arbitrary PCM constructor or copied xLog implementation. This establishes local
producer correspondence, not owner trust, RF validity or message certification.

`CandidateOutput` has private immutable fields and getters and is deliberately
not serializable. It retains exact source manifest/PCM and the exact serialized
offline artifact manifest, output PCM and both whole-body hashes. Its manifest:

- distinct final-output artifact/schema version;
- source whole-body hash and exact raw-manifest/PCM byte lengths;
- transform version, exact coefficient-bits hash, arithmetic order, causal
  offset zero and group delay 256 output frames (16/3 ms);
- 48,000 Hz, F32LE interleaved, exactly two ordered channel entries;
- each channel explicitly `Source(gain_f32_bits)` or `Silence`, with an explicit
  duplication declaration if both carry source; no channel assignment default;
- explicit route-policy identity (description, **not commissioning**) and bounds;
- full-period frames/bytes, conservative actual-source-derived signal support
  including complete FIR tail, and exact output PCM digest.

The offline whole-body digest covers new exact raw manifest and final PCM using
`xnexus-offline-final-output-body-v1\0`, followed by little-endian u64 raw length,
raw bytes, u64 PCM length and PCM. It is not an actual owner Body or approval.
Actual owner Body integration remains separate work. Original raw manifest and source PCM stay alongside the
final artifact so the owner can reproduce it from an admitted producer. A hash
and caller-declared span alone are insufficient.

## Numerical candidate

513 taps, symmetric Kaiser beta 10, cutoff 6 kHz at 48 kHz, sum/gain four.
`generate_coefficients.py` produces fixed f64 bits and hashes in a NEW directory.
Its floating-point generator is an offline tool, never a runtime filter designer.
The only external filter constructor, `fixed_candidate()`, pins the exact
measured generated table; arbitrary bits are crate-private for negative controls.
This establishes coefficient identity, not final waveform/owner trust.

The coefficient-only first attempt (15 November 2026) passed unchanged targets:
ripple bound 0.00008545582051819531 dB and stopband bound 100.60422513238589 dB,
including interpolation allowance 0.000005116501185357802 and numerical allowance
1e-11. Unity-impulse and asymmetry controls were rejected. The generated manifest's
`qualified:false` remains unchanged because wider qualification is incomplete.

- f64 bits: `B3B266E2346DA0272CDB1910943A6BA8020D17A56E55A7B0EB0F12CD2C73C764`
- generated Rust table: `6DCE3DBA8F5395BA8C394FC7C3D4CC6A577FC45FE91E2F034B023FC4B3CFA625`
- generator manifest: `8177B56AA31FA2A1AA96A594D783EF298836FEED6C5D522F3F55011B178D0FD0`
- numerical analysis: `FF080FE5118C76271D14C17F8F1DB18A59C9A84A537A1F7086EFDC52374ACD15`
- attempt receipt: `C56D54B4176CE32895BE62D566F4A163B435D675E56E2BB46C3B7F8034513DF4`
- guard: `887F335D2A0E31622769F543CF4E57482B9500EACB8638D8B49100EA362737C8`

Evidence is retained in the isolated worktree under
`target/coefficients-attempt-662a45fcebdd47f49d75f019b84eab0f`. The 1 GiB/300 s
job retired naturally (root/effective zero, final processes zero, no force/flags),
peak 52,793,344 bytes. All 1,895 source/tool pins were unchanged. No compiler ran.

The polyphase kernel computes `sum x[n] * h[m-4n]` in ascending source-index order,
distinct f64 multiplication/addition, then explicit channel gain in f64 and final
f32 rounding. No `mul_add`, fast-math, reassociation, clamping, normalization,
repetition or linear interpolation. Cross-platform exact bytes are unqualified.

Support comes from the first/last ACTUAL numerically nonzero source samples
(including subnormals), expanded by the entire 513-coefficient extent. A complete
tail outside the fixed period is refused; no waveform_frames trust or threshold
cropping. Source nonfinite/empty samples, output nonfinite/overshoot and insufficient
capacity are refused. FT8 has 720,000 frames/5,760,000 stereo bytes; FT4 has
360,000 frames/2,880,000 bytes. Delay is explicit and not backend-compensated.

The proposed tone-centre passband is through 5.2 kHz. The actual GFSK spectral
envelope is a further gate; passing a tone-centre check is not spectral proof.
Declared corpus fixture gain may be 0.5, never inferred by normalization. Tests
using a unity-impulse filter use gain 0.125 only as a numerical negative-control
fixture; that filter MUST fail independent image-rejection analysis.

## Verification boundaries

1. **Passed coefficient-only:** bounded Python job generated once, independently analyzed once,
   retain outputs/failure, never modify tap count/beta/limits automatically.
   Targets: <=0.01 dB ripple through 5.2 kHz; >=80 dB rejection from 6.8–24 kHz.
   Analyzer does not import generator/Rust. It evaluates centered direct DTFT at
   1 Hz and a global second-derivative bound on interpolation error, with an
   explicit numerical allowance. It must reject a gain-four impulse (image
   leakage) and an asymmetric mutation. Not rigorous libm-error certification.
2. **Passed twelve tests:** fresh actual Rust prepare library/output test module,
   pinned coefficients and bounded compiler runner. Twelve actual unit tests cover
   manifest, raw-byte binding, arithmetic, support, silence, gain, bounds and
   tone-centre refusal. Fixed-filter FT8/FT4 output is exercised, but these are not
   all14 corpus/reference/owner qualification. No preparer-golden rerun was claimed.
3. **Passed fourteen retained cases:** regenerated through the actual preparer,
   matched original v1 source PCM and all manifest fields, then transformed using
   the pinned table and explicit fixture policy: source gain 0.5 on channel zero,
   silence on channel one. Existing corpus files remained unchanged.
4. **Passed independent reference:** separate C++ scatter convolution matched
   every final byte for all fourteen cases. Two missing/wrong-corpus controls,
   seven reference refusals and two comparison controls passed. Exact support,
   full tails, fixed period sizes and silent-channel behavior were checked.
5. **Passed fixed-1-kHz spectral observation:** independent radix-2 FFT of complete
   rounded outputs measured image-band peak rejection 125.2427–127.6598 dB
   (6.8–24 kHz relative to 0–5.2 kHz). Finite-grid measurements are not continuous
   or cross-platform certificates. Additional offset edges/range, waveform
   envelope and actual modem decoding remain separate unrun gates.
6. Owner reproduces and binds final artifact in actual Body/Permit fake-backend
   regression, with revocation/substitution/release-identity controls. Pending.

No stream format, DAX channel meaning, presentation clock, PTT or RF policy is
selected by this crate. Those remain separately reviewed owner commissioning.

### Twelve-test evidence

Isolated worktree evidence: `target/output-rust-attempt-ece75199f22d4a84846c5f17f0d63bf4`.
Twelve passed, zero failed/ignored/filtered, 14.34 s. Fresh actual prepare library
and output test executable; no Cargo build or Clippy. Source17/tool50/dependency1746/
provenance8 pins unchanged before/after. Guard2GiB/300s/j1/BelowNormal/five-second
drain; root/effective0, final0, no force/flags, natural25ms; peak576,622,592bytes.

- receipt: `731A01D282D95A016389D62CCCD3ECFEDAE64AFA85106BE4B948A9C303B44159`
- guard: `4C7B9832BCD20C30A4A86EEA9BFB1BFFE189BBFDBBB89B2260051209D1261A8A`
- checks: `C42A57F3B56A52E11F008FAAAADEFB9D11AF7F8A1061244E796DC0EE635B85B8`
- test log: `8EBD8AD72D42AB261F887D9548ED7DAADEF93CABB60E1E6AD82C0D83DCFF26C6`

Separate locked offline GNU metadata passed: 81 packages (66 registry, 15 local). The unchanged lock retains 68 registry identities/checksums, all present in existing paired-lock seed04C139. No newer dependency
resolution or Cargo build. Lock4151461F1BD44660D29DDE51C41362F398DEC0AFA0BBC3ED24517AFEB41C80DA;
metadata943BAB00A50238D2A7DD5DE4696BE07CB3EF04C91AA243ACFE55144D08AAFFF0;
receipt7E31C0A9FB20C85D549ACDD82388F4EE6EBB97EE02E4846E097D423F63B04978.

## Maintained reproduction entry points

See [VALIDATION.md](VALIDATION.md) for exact source/tool/receipt hashes, preserved
failed preparation, completed counts and remaining gates. These commands are
reproduction instructions, not evidence that a Cargo build or Clippy has run.
Use GNU Rust 1.93.1 and a separate output directory; disable FMA as in the qualified
direct compiler runs. Do not substitute another corpus, coefficient table or
channel policy and claim the same evidence.

```powershell
$env:RUSTFLAGS = '-C target-feature=-fma'
cargo +1.93.1-x86_64-pc-windows-gnu metadata --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-gnu --manifest-path integrations/xlog-tx-output/Cargo.toml
cargo +1.93.1-x86_64-pc-windows-gnu test --locked --offline -j 1 --manifest-path integrations/xlog-tx-output/Cargo.toml --lib
cargo +1.93.1-x86_64-pc-windows-gnu run --locked --offline -j 1 --manifest-path integrations/xlog-tx-output/Cargo.toml --example export-corpus -- ABS_CORPUS ABS_NEW_OUTPUT
```

Replace both `ABS_...` arguments with absolute paths. The exporter requires the
exact retained inventory and refuses an existing output directory. It writes
source/final manifest and PCM files per case plus `export.json`; preserve all of
them. The fixture uses an explicitly noncommissioned route description.

Compile the independent reference with GCC 15.2.0 using the qualified arithmetic
flags (on Windows use the reviewed ucrt64-first PATH). For each exported case:

```text
g++ -std=c++17 -O1 -fno-fast-math -ffp-contract=off -mfpmath=sse -msse2 tools/full_convolution_reference.cpp -o ABS_NEW_REFERENCE_EXE
ABS_NEW_REFERENCE_EXE ABS_CASE/source.f32le coefficients/coefficients.f64le ABS_NEW_REFERENCE_PCM source:3f000000 silence
```

Run those reference commands from this crate directory. Require exit zero, exact
byte equality with that case's `final.f32le`, and the printed frame/support/FFT
measurements. Reproducing the accepted gate also requires all eleven controls
listed in [VALIDATION.md](VALIDATION.md); merely running one example is not that
gate. Owner reproduction, fake-effect lifecycle, platform and physical output
acceptance remain separate.
