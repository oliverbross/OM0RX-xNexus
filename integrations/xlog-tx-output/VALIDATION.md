# Offline final-output validation — completed offline candidate gates

This record describes completed offline candidate gates. It does not qualify owner acceptance or real output.

Source branch: `codex/offline-final-output`, based on
`4b9867e8dc65517b269948afa7e9bbc207b71047`. The new isolated crate is not an
application dependency. Existing 12 kHz mono v1 artifacts and the retained
fourteen-case corpus remain unchanged. No device, stream, backend, clock, PTT,
owner endpoint, journal, installer or user-visible version is introduced.

## Completed: numerical coefficient candidate

One generated 513-tap symmetric Kaiser candidate (48 kHz, cutoff6 kHz, beta10,
gain-four sum, causal delay256 output frames) passed the independent direct-DTFT
numerical gate on15 November2026. This is not cross-platform transcendental-error
certification or final waveform/owner qualification. The original generator
manifest still says `qualified:false` and was not rewritten after the result.

- Passband ripple bound through5.2 kHz:0.00008545582051819531 dB, target<=0.01.
- Stopband bound from6.8–24 kHz:100.60422513238589 dB, target>=80.
- Direct grid:5,201 passband and17,201 stopband points at1 Hz.
- Conservative interpolation allowance:0.000005116501185357802;
  explicit numerical allowance:1e-11.
- Unity-impulse image-leakage and asymmetric-coefficient controls were rejected.

Retained attempt: `target/coefficients-attempt-662a45fcebdd47f49d75f019b84eab0f`.

| Record | SHA-256 |
|---|---|
| Receipt | C56D54B4176CE32895BE62D566F4A163B435D675E56E2BB46C3B7F8034513DF4 |
| Guard | 887F335D2A0E31622769F543CF4E57482B9500EACB8638D8B49100EA362737C8 |
| Analysis | FF080FE5118C76271D14C17F8F1DB18A59C9A84A537A1F7086EFDC52374ACD15 |
| Exact4,104-byte f64 coefficient table | B3B266E2346DA0272CDB1910943A6BA8020D17A56E55A7B0EB0F12CD2C73C764 |
| Generated Rust bits | 6DCE3DBA8F5395BA8C394FC7C3D4CC6A577FC45FE91E2F034B023FC4B3CFA625 |
| Generator manifest | 8177B56AA31FA2A1AA96A594D783EF298836FEED6C5D522F3F55011B178D0FD0 |

The1GiB/300s job used corrected85A7/1390 guards,8/6GiB admission,2/2GiB floors,
BelowNormal and five-second natural drain. Root/effective0, final processes0,
no forced cleanup or abnormal/memory flags; natural retirement18ms; peak52,793,344
bytes. All1,895 source/tool pins remained unchanged. No compiler executed.

## Completed: twelve actual Rust module tests

Fresh GNU1.93.1 direct compilation of actual preparation library and output test
module passed12 named tests, zero failures/ignored/filtered, in14.34s. No Cargo
build, Clippy, app or full fourteen-case/owner claim is attached to this result.

Coverage includes actual fixed-filter FT8/FT4 output, exact frames/bytes and silent
channel, strict raw-manifest/actual-private-PreparedPeriod correspondence, source
whitespace affecting final identity, all-phase scatter comparison, actual-nonzero
full-tail support, finite/empty input, coefficient identity, explicit duplication,
gain/clipping/capacity refusal and accepted/rejected tone-centre passband edges.

Retained attempt: `target/output-rust-attempt-ece75199f22d4a84846c5f17f0d63bf4`.

| Record | SHA-256 |
|---|---|
| Receipt | 731A01D282D95A016389D62CCCD3ECFEDAE64AFA85106BE4B948A9C303B44159 |
| Guard | 4C7B9832BCD20C30A4A86EEA9BFB1BFFE189BBFDBBB89B2260051209D1261A8A |
| Checks | C42A57F3B56A52E11F008FAAAADEFB9D11AF7F8A1061244E796DC0EE635B85B8 |
| Test log | 8EBD8AD72D42AB261F887D9548ED7DAADEF93CABB60E1E6AD82C0D83DCFF26C6 |

The2GiB/300s/j1/BelowNormal job used unchanged85A7/1390 guards and five-second
natural drain. Extra concurrent-job admission14/14GiB passed first. Root/effective0,
final0, no force/abnormal/memory flags; natural25ms; peak576,622,592bytes. All17
source,50tool,1,746 immutable dependency and8provenance pins were unchanged.

Qualified current runtime hashes: lib1B821274BB638F6CA7608A810108410D223BB36B052BD7ABEBE69BA1D1496377;
fir40F50E909FF60BCC9F682CD4AA2E163466D2C3E125E0FA220835A54FDF0DA623;
testsA5FBF1145EBB8495D90DBA1C8657FE208AD09F0F0E5D8B005CFF6E4BB05ED1B7.

## Completed separately: locked offline graph only

Isolated lock seeded from the existing paired crate retains 68 registry identities/checksums, all present unchanged in that seed. Metadata resolved 81 packages (66 registry and 15 local). Locked offline
GNU metadata passed; this does not qualify a Cargo build or Clippy.

- Lock:4151461F1BD44660D29DDE51C41362F398DEC0AFA0BBC3ED24517AFEB41C80DA.
- Metadata:943BAB00A50238D2A7DD5DE4696BE07CB3EF04C91AA243ACFE55144D08AAFFF0.
- Receipt:7E31C0A9FB20C85D549ACDD82388F4EE6EBB97EE02E4846E097D423F63B04978.

## Fourteen-case gate: completed corrected attempt; first failure preserved

The first exact proposal below failed before any corpus/reference case. The separately reviewed environment correction subsequently passed all fourteen cases and eleven controls. Both attempts and the unexecuted intermediate proposal are retained.

Frozen proposal in `target/output-corpus-prep`:

- Inputs10FBD67A2CEF49FE6756FFE2CE86C644FD25F19E42F2CFC8B67765C005C825B5.
- RunnerD18424E65DE3A4E263C53FA01BA69950FF20EE782AC45F9EB0F60A10ED840B2E.
- Child9AB8D089D1C670FC5A99D1CA39B53597751FEAC906B8EC8FEA337B7EC606D9DD.
- Exporter405E3348A2003DAD96A58D81E83C60C2B8AAF66518F6BE19AF648EEF93861415.
- Independent reference4B67DD1A5B2381318C0473B7235A73436711AF2C454E91C1F3835419E2E8DE0B.

Original proposal used separate2GiB/300s,8/6GiB admission,2/2floors,j1,BelowNormal,five-second
natural drain. No automatic retries, specification changes or cap widening. Preserve failed
or partial results. Before/after pins cover48source,20,777 installed tool/header/
library,1,746 immutable dependency and13provenance files. The compiler's actual
C++ dependency file must contain only pinned inputs.

Fourteen1kHz v1 cases were regenerated through actual preparation and compared
to every original manifest/PCM/body pin. Explicit fixture policy is channel0 source
gain0.5, channel1 silence, no duplication and a noncommissioned route description.
Required exact stereo sizes are5,760,000 FT8 and2,880,000 FT4 bytes.

Independent C++ uses source-by-tap scatter convolution and full tail beyond the
period, then a separate radix-2 FFT of complete rounded output samples, with no
Rust/polyphase/DSP imports or taper. Required measurements include exact final
byte equality, conservative support/geometry, expected1kHz tone peak interval and
sampled image-band peak rejection>=80dB from6.8–24kHz relative to0–5.2kHz. Finite-grid
FFT measurements are not a continuous/cross-platform certificate.

Mandatory controls:2 missing/wrong corpus refusals;7 source-NaN/tail/oversize/
coefficient-NaN/gain-NaN/clipping/image refusals;2 wrong-channel/one-bit comparison
controls. Counters and named fourteen cases must be nonzero and exact.

### Preserved first failure and reviewed environment correction

Attempt `target/output-corpus-attempt-ba7dd3c1b2bb4937ab894eaee9491e47` compiled the
actual Rust preparation/output libraries and exporter, then failed during C++
preparation. No reference executable or dependency file was produced and its
compile log was empty; no corpus case ran. The initial harness did not retain
the compiler's immediate native exit code, so the exact loader/process outcome
cannot be recovered from that log. Guard root/effective1, final0, natural21ms,
no force/abnormal/memory flags, peak611,926,016bytes. Every input pin was unchanged.

- Failed receipt:E1E46CB7C314F0146B0EA475CB01E78343A70313F6E4D7063815633B61D5D514.
- Failed guard:FE0494363A43EC9314C1DA1F5259F5E0F52B0E5977083AFD4CF24062AA0749D5.

Read-only PE inspection found that actual cc1plus imports `clock_gettime64` from
libwinpthread; the Rust-directory version lacks that export while the ucrt64
version has it. The first runner had Rust first in PATH. DLL shadowing is a
plausible static explanation, not observed loaded-module attribution.

Separately executed proposal `target/output-corpus-ucrt-14g-prep` changed only
ucrt64-first PATH, GCC verbose/native exit/stage/error recording, necessary DLL
pins, and a stricter fresh14/14GiB pre-dispatch overlap admission. Standard guard
and2GiB/300s/8-6/2-2/five-second limits remain unchanged. Actual Rust/C++ algorithms,
coefficient bytes, cases and assertions are unchanged.

- Inputs:5FF39A26CB0D35611238094C1710517E21F7FAA4B4C6853A77A8D1AC522E23E9.
- Runner:7F73489E7D210640F8EA756CA6CE0699E1395D6A10F4D9162574915C8612FE12.
- Child:F5464B4F4D6F806205C845349D125E1E8E1819549C1C6DCDFB7C95B725D2AADA.
- Static inspection:3AD51B779F8467A3B30C5EA79C5807CCA7C997E9C8D0BFDA3DC78BA5D50F358F.

## Actual corrected fourteen-case result

Attempt `target/output-corpus-attempt-f9999844ac5f47bb84975850d6da1cc6`
passed on 15 November 2026. Fresh actual preparation/output Rust libraries and the
exporter regenerated all fourteen retained v1 cases. A separately compiled C++
scatter-convolution reference matched every final PCM byte for all fourteen cases.
Both corpus refusal controls, all seven reference refusal controls, and both
comparison controls passed. No case was skipped.

| Record | SHA-256 |
|---|---|
| Receipt | 613BEB9845C9BB9FFB1990D1A677C69C706007284BE138955F12AC2D3651D875 |
| Guard | 7256C93498227D5B9E71FD7D5FB36E65FBD04C3B2446802B7C967DF1F7A04E59 |
| Checks and fourteen per-case reference hashes | 1AABA371DACD1FF0B16ABA5362A8F45F46CB10F7C31AF4C05ECF38471F6EB876 |
| Export inventory with source/final identities | 894D2AF190A08E1CD3865B01D1090096C5676914B28721CA1892C64635CD7579 |
| C++ process record | 5DFB623E945A81ADA2549FBB2CD4D08667C45A9704E0D5F95FA9E08FE9E343BD |

C++ compilation exited 0 (`0x00000000`) with its exact command, timestamps and
verbose tool output retained. The corrected environment worked; this does not
prove which DLL caused the original failure.

The stricter fresh 14/14 GiB admission passed with 20,498,493,440 bytes physical
and 19,297,107,968 bytes available commit. The unchanged 2 GiB/300 s guard completed
with root/effective exit 0, final active processes 0, cleanup confirmed, no force
or memory/abnormal flags, and 25 ms natural retirement. Peak aggregate commitment
was 610,476,032 bytes; minimum host physical/commit were 20,122,361,856 and
18,898,145,280 bytes. Child work retired at 20:43:58 UTC; final before/after
verification completed at 20:44:30 UTC. All 48 source, 20,780 tool/header/library,
1,746 immutable dependency and 19 provenance pins were unchanged.

For the explicitly declared 1 kHz corpus and channel-zero gain 0.5/channel-one
silence policy, sampled FFT image rejection ranged from 125.24268713838973 to
127.65982295204795 dB. FT8 support was [24004,631229) within 720,000 frames;
FT4 support was [14404,256829) within 360,000 frames. These conservative support
bounds include the full coefficient tail. This is finite-grid waveform evidence,
not qualification of every offset, cross-platform bytes or physical output.

## Remaining gates after the fourteen-case pass

The corpus uses1kHz only. Additional offset-range and modem decoding acceptance,
cross-platform exact arithmetic/FP behavior, independent owner reproduction and
final-body validation, actual owner Body/Permit/fake-effect lifecycle integration
remain unqualified. Owner reproduction must validate the actual retained complete
artifact, not merely a claimed digest/span. No owner API or real output follows
automatically from these offline results.

Actual stream format, physical channel interpretation, presentation-clock mapping,
independent release/watchdog, route/power policy, supervised PTT and RF acceptance
remain future gates. RX route mapping is not TX authority.

## Maintained Cargo target metadata

After waveform qualification, documentation was finalized and `export-corpus`
was declared as an example targeting the unchanged `tools/export_corpus.rs`.
One locked offline GNU metadata invocation passed; no Cargo build or Clippy ran.
The package identity set is identical to the earlier metadata: 81 packages,
66 registry and 15 local. Cargo.lock still contains 68 registry identities and
has the same 4151461F… checksum. The earlier receipt's shorthand combined these
two counts; it is preserved, not rewritten. All tested runtime, exporter,
reference and coefficient bytes remained unchanged.

- Metadata: `2A83F372848D1B29853AF6A3392C214CA3E00418D51F60FB2286C7002FDD068C`.
- Verification: `467A92580AFCC32CB7045D098D368F5AF11784CD6C347246C7CCF3E209C3F1CE`.
- Evidence: `target/output-example-graph-7733cfeaa9124ef4a5ebdaf77fc8fbe4`.

See [README.md](README.md#maintained-reproduction-entry-points) for commands.

## Generator metadata byte-preservation follow-up

Initial source commit `d5336b15e861b41f53569a133c6d8810182d2554` applied the
repository's text normalization to the generator metadata only: its 13 CRLF
sequences became LF in the Git blob, SHA-256
`341A5EBE345010AE42574BBA6CFAD2B426EF4C4D0C5802C3D0FB35B9A721844E`.
The tested/retained original and worktree file remained unchanged at
`8177B56AA31FA2A1AA96A594D783EF298836FEED6C5D522F3F55011B178D0FD0`.

A crate-local attribute applies `-text` only to `coefficients/manifest.json` and
restores the exact retained bytes, including all 13 CRLF sequences, in Git.
This is artifact preservation, not a regenerated manifest or waveform change.
The tested Rust implementation/tests, exporter, independent C++ reference,
B3B266 coefficient binary and 6DCE generated Rust table remain byte-identical.
No tests were rerun for this byte-preservation correction; earlier receipts and
the initial commit remain intact.
