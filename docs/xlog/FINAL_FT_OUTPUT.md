# Final FT output for owner transport

Native source slice based on kernel commit
`5a5ec7e0cabce66d0fe6bf423f0bf5d99ffbac4a`. It introduces no output device,
PTT, owner capability, transport, application registration or version change.
The existing 12 kHz preparation API and prepared-period v1 are unchanged.

`xlog_ft_output::prepare(&Prepared, gain, deadline)` accepts only the actual
private original encoder result. It returns an immutable `FinalPeriod` with
private PCM and exact JSON manifest getters. FT8 is 720,000 final frames /
2,880,000 PCM bytes; FT4 is 360,000 / 1,440,000. Both are 48 kHz mono f32le.
The SDK embeds that manifest into its operation descriptor and sends a four-byte
little-endian descriptor length, exact descriptor JSON, then exact PCM. The
whole operation SHA-256 binds all those transmitted bytes, including whitespace.

The schema-2 manifest matches the owner contract reviewed at SHA-256
`1208BDA0E4B86AB724E76C279C2B02E1258830ED0EE32336F7549E0BE898EC17`:
`schema_version`, `mode`, `message_form` (`wsjt77`), exact `message`, `format`,
`offset_hz_f32_bits`, `frames`, `waveform_frames`, `pcm_byte_count`,
`period_duration_us`, `pcm_sha256`, `content_sha256`, `filter_id`,
`coefficients_sha256`, `group_delay_frames`, `support_start_frame`,
`gain_f32_bits`. No message packing, truncation or case conversion is added.

Content SHA-256 preimage is `xnexus-final-period-v2` plus NUL, one mode byte
(8 or 4), rate u32LE, channels u16LE, `f32le` plus NUL, offset bits u32LE,
message UTF-8 byte length u32LE and message, PCM byte length u64LE and PCM.
The complete operation hash additionally binds transform/support/gain metadata.
Neither hash alone proves RF transmission, owner commissioning or message truth.

## Exact transform and provenance

Only the FIR kernel and generated coefficient bits are reused from parked commit
`81aaf90c718a0f1ce14902717739a76ea3dea8d0`. Its other crate/application changes
are not dependencies or promoted by this slice. Kernel changes are only its
module-local Error import and generated-table path. The original GPL-3.0-only
SPDX notice remains; this code stays in Nexus, not the xLog owner.

- Filter: `kaiser-513-fc6000-beta10-x4-candidate1`.
- Exact 513 f64 coefficient bytes SHA-256:
  `B3B266E2346DA0272CDB1910943A6BA8020D17A56E55A7B0EB0F12CD2C73C764`.
- Generated Rust table, byte-exact SHA-256:
  `6DCE3DBA8F5395BA8C394FC7C3D4CC6A577FC45FE91E2F034B023FC4B3CFA625`.
- Prior numerical analysis receipt `C56D54B4176CE32895BE62D566F4A163B435D675E56E2BB46C3B7F8034513DF4`
  measured ripple bound 0.00008545582051819531 dB through 5.2 kHz and stopband
  attenuation bound 100.60422513238589 dB from 6.8 kHz. This is numerical
  coefficient evidence, not a cross-platform floating-point certificate.

Explicit finite gain greater than zero and at most 0.8 is applied to each source
f32 sample before the FIR. There is no default RF power, normalization or clamp.
Ascending source-index f64 multiply/add then one f32 rounding is retained; build
flags must not permit fast-math/reassociation/FMA. Every final sample must be
finite with absolute value at most 0.95, otherwise preparation fails.

The filter is causal with offset zero and group delay 256 final frames (16/3 ms).
There is no delay compensation or trimming. Support is derived from actual
nonzero gained samples: first*4 through last*4+513 exclusive. `waveform_frames`
is that complete end-exclusive support, not a multiplication of declared input
waveform length. A tail beyond the exact full period is refused. Samples outside
support are positive zero. The owner retains the full lead/delay/tail and only
maps mono into explicitly commissioned DAX stereo channels; it never resamples.

## Focused qualification

Final source passed 35 named tests: six new final-output cases plus all 21
coordinator/encoder and eight decoder cases, zero failures or ignored tests.
The output cases cover actual FT8/FT4 at 100, 1,000 and 4,500 Hz, free text and
compound/hash forms, exact period/support/positive-zero geometry and wire hashes,
full independent source-driven FT4 convolution byte equality, a 10 Hz coefficient
spectral grid with corrupted-filter negative control, explicit gain, clipping,
NaN/infinity, empty input, full-tail refusal and deadline rejection. The existing
1 Hz numerical coefficient certificate remains separate from this regression.

Evidence: `owner-ft-runtime/target/output-attempt-ae4f132e64fb4cd1a17031003c531713`.
Receipt SHA-256 `9A0D6B7730F6A3E89B529C0E0034914719677C4B2A76B96AC0A19743CAABEE54`;
test log `69C613DDE08923E6A6581AD2AC092462CF8C07D30A1B0D0328FBEF7DB54ECA5D`.
GNU 1.93.1 compiled fresh actual modules with `target-feature=-fma`; 1,746 retained
dependency pins and all source/tool pins were unchanged before/after. The
2 GiB/300-second job completed naturally, root/final processes zero, no resource
flags or force, peak 647,077,888 bytes, test execution 67.22 seconds. No Cargo or
Clippy qualification is inferred from this direct native module gate.

This slice does not claim SDK/owner acceptance, cross-platform byte identity,
real output, native UI or RF testing. The modules remain application-unregistered
until the separately reviewed native integration.
