# Passive bearing body codec — qualification

This slice passed focused Rust tests and SDK Clippy on 2026-11-15. It exports no
capability, makes no HTTP request, starts no owner or device, and adds no UI.
The validated `Bearing` wrapper contains passive data only, not a freshness
lease, a motion grant or transmission authority. The future nine-key host
envelope and station-identity/session validation are separate work.

## Actual producer fixture

`bearing-swift-v1.json` is copied byte-for-byte from xLog's
`scripts/companion-bridge-tests/fixtures/bearing-swift-v1.json` at producer commit
`9d7555d5d23f8f4dc7953e40b60198b7af19a751`:

`307BD860A42BC5A3191FA0F421BFCA32AB0C27CC44DF0843840AE0F57B43FE8D`.

It contains four actual Swift encoder results: historical grid, CTY entity
approximation, coincident geometry and unavailable reference. These came from
the owner's thirteen-test Windows run, not from a hand-authored client fixture.
Owner receipt `.build/xnexus-bearing-tests/swift-20261115-175429/guard.json`:
`15BBFE2AAFE781F895D02E4A384FB146FB2AD63682EDFF1AC3129FC66D18D0FF`.
The source qualification is documented in xLog's
`docs/planning/COMPANION_BEARING_QUALIFICATION.md`.

The qualified `Sources/XLogCore/CompanionBearing.swift` git-blob bytes at that
commit hash to `BA00F2732B6ACAC5B493C9C282C2E92714D791D35E60C6215773DDF9A00003B3`.
The subsequently observed endpoint-worktree HEAD `48a645f7449522ebbb6f37bc81b8ecce641a1c5d`
adds a public grid-normalization accessor and an early length guard; it is not
claimed as qualified by the earlier producer run. The fixture is unchanged.

The Rust replay extracts each fixture object through `serde_json::Value` and
serializes that object for decoding, then compares every output field to the
actual producer object. Thus the retained **corpus** is byte-identical; extracted
body escaping/formatting is reserialized, not represented as original wire bytes.
Comparison normalizes only the seven schema-declared floating-point coordinate
and geometry paths: Swift may emit an integral Double as `1`, while Rust emits
`1.0`. All other JSON values/types remain exact, including integer timestamps and
revisions. A changed-coordinate negative control must still compare unequal.
Mutation tests are synthetic negative/boundary controls, explicitly separate
from the actual producer's four exported states.

## Codec scope

All 12 body keys and nested keys are mandatory, including explicit JSON nulls;
unknown and duplicate keys are refused. The exact requested ASCII operating call
must match the caller's supplied request, and only ASCII uppercase normalization
is accepted. No trimming, Unicode folding, base-call substitution or resolver is
used. Origin/grid/target-source precision and nullable historical/CTY evidence
remain distinct. Signed historical timestamps retain their original values;
they never establish freshness. Nullable matched-reference/rule strings remain
raw and optional, with the producer's byte bounds. No local distance, grid-center
or reciprocal-bearing calculation validates or replaces owner geometry.

The qualified focused test set has 12 Rust tests, including the four-state actual
producer replay and exhaustive required-key removal across its populated object
shapes. It covers call binding, authority, source/evidence shape, finite geometry,
safe signed timestamps/revisions, status/reason consistency, metadata byte limits
and the exact 16 KiB body limit. No native Tauri/DSP build, HTTP endpoint, host
envelope, UI, movement or transmission is qualified by these checks.

## Client validation evidence

The source baseline was `5b94295003fea92029df6b7da85469edf5da150b`, plus this
isolated codec contribution. Production codec SHA256:
`D92AC7EC1F0B10B1A3A455EBBD67DCEC73359913162A13D952AD0A95FACCE3D1`.
Final test source SHA256:
`379E9B204288B5920C607F802F22A9B0C7B1C257488AD6028882466E3CDEC791`.

The first attempt is preserved under the bearing-codec worktree's
`target/bearing-codec-preparation/attempt-538118e1e849424f942d8159667bd695`.
Its receipt `25C39FF3C4BC7A1E3D9932963F0243070953486A3385D4B6086CD717FEC1C859`
remains failed: eleven tests passed, while JSON `1` versus `1.0` representation
made the actual-corpus comparison fail. The codec accepted that producer body;
the correction changed only comparison of declared floating-point fields and
added a changed-coordinate negative control. Original test and runtime bytes,
logs and receipt remain retained. Clippy was not reached in that first attempt.

The separately authorized corrected attempt is retained under
`target/bearing-codec-numeric-preparation/attempt-3b3513c679234141bd22081b81efb86c`:

- Guard receipt: `BDF723495350690E04683A0ADE79E419828480F7B4100AC32954DD056FEA3CCC`.
- Checks: `D9E4205EA45C54E23578FF08E3A82E7ED06C1309A7F227D6B1009498728E39C4`.
- Source before/after, identical: `B24DB25E19E7D752E1F43BD64C6D185D53F7F840824A68B1D33E513D9907F422`.
- Fresh SDK test executable: `1301BC1775AD8EEE399825A83DE9DCEF8287F81DED31C02522F427D371A541A1`.
- Twelve-test log: `20B28A8A30B80DC1AA454FDC9E884B4B0984D87B1B56F32434F17EB99A57A081`.
- Clippy log: `E6B5A9B5982F9BB6E32BA2CDD666F122980D4346446C3D8DE09C4CF86726C4A3`.

Exact GNU Rust 1.93.1, offline/locked, one job; changed-file rustfmt check and
SDK all-targets Clippy with warnings denied passed. All twelve selected tests
passed, none ignored; 137 unrelated SDK tests were filtered, not claimed rerun.
The fresh target copied only qualified third-party cache inputs; all 1,702 donor
inventory files were rechecked unchanged. No dependency or lockfile changed.
The private Job was capped at 2 GiB and 300 seconds, with 8/6 GiB host physical/
commit admission, 2/2 GiB runtime floors and three-second natural drain. Root
PID 18696 exited 0; natural active count and final count were both zero, cleanup
confirmed, no memory/abnormal-exit notification, peak job bytes 677,142,528.
This qualification paragraph was added after the source-pinned run; executable
source, tests and actual corpus remain the tested bytes.
