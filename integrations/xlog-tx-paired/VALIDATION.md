# Maintained paired sequence regression

The fixture and 18 assertions were first qualified as an isolated, ignored
direct-rustc harness against Nexus source
`8376996993e0cb9ef987c643761b484fdb3f5c0f` on 15 November 2026.
Moving them into this crate changes only generated owner imports and the crate
name. The maintained-source rerun also passed all 18 cases and five input-rejection
controls. Neither direct-rustc result claims a full Cargo/Clippy or application
build.

## Retained initial combined pass

Evidence under the isolated sequence worktree:
`target/combined-owner-attempt-e2c82525f050401d9290a71ad1983cb5`.

| Evidence | SHA256 |
|---|---|
| Final receipt | `7CABE4DF8A24A1495693E68A4530FCA283D223025317610A896436D80304D7C5` |
| Guard | `6CA15E611E23AD276C4FE63B766A0C97966FD044697FD989E4B74C8491B78604` |
| Exact checks | `83CC9C8652F8E5F701CE51226F3D7E75D2D8DF7D08084CC0061AC6A7D3AC1E8B` |
| Eighteen-test log | `F9E774FCC9EC32CCF78561A7B477E5B7DA9E663D04A822BE1A0D37BE58B2D97A` |
| Original fixture | `8EA8A9DBB59DD2ABE5286B5CD0D3D917CF5AE63499976D029F28565FE0966E3B` |
| Original test source | `5A3E735634178671660ABD73A30FE548716C1C192EAE4CD601F7A082CA3E0DC7` |

All 18 tests passed: fourteen corpus cases and four negative controls, with no
ignored or filtered tests. The guarded root and descendants exited naturally,
exit 0/final active 0, no forced cleanup or memory/abnormal-exit flags, 19 ms
drain. Peak job memory was 581554176 bytes. Minimum available physical memory
was 20258828288 bytes and commit 18763706368 bytes. The unchanged policy was
2 GiB aggregate cap, 300 seconds, fresh 8/6 GiB physical/commit admission,
2/2 GiB runtime floors, BelowNormal, sequential compiler calls and five-second
natural drain. All 21 source, 50 tool, 1746 retained dependency pins and 29 corpus
files were unchanged before/after. Existing preparer/sequence sources were
freshly compiled; retained qualified GNU 1.93.1 dependency rlibs were read-only.

## Maintained-source rerun

Evidence under the same isolated worktree:
`target/maintained-paired-attempt-fda93ed0ebeb4b31aaadb23751651053`.
The new crate was tested as an uncommitted additive change over the same exact
`8376996993e0cb9ef987c643761b484fdb3f5c0f` base. Actual source hashes:

| Input/evidence | SHA256 |
|---|---|
| Maintained fixture library | `633C3BAEDE93C48BE6C9761EF1F95E264A0C3C5D7F33B82D807EB4F922637BA7` |
| Eighteen integration tests | `2D05AAC59537307EC6A1273BE092435AF04F4F7DB29FF50724B06BADB3F62D71` |
| Actual build script | `C8D357411A1DDA183A5B881013E635FA0A14D5FA0689C65C9174C8EAEE577E18` |
| Reviewed inputs | `32EF8F6CEFF0F713F4A9B7FA76293710C891ACCCEBFFBC5FC2BB7BD945D9843A` |
| Final receipt | `69252C6DAE98779C1142830955A5FFB62DE4912421ED10B404B155FDE35A3D82` |
| Guard | `77D26AB8B9280D679ED959FCD82C283123576C0870626D7897D3A388C1FE6126` |
| Exact checks | `C4BEB3099FCCB580E2FBB1F3FC0FEF59BC24B1447A200F474387ECA3BB19A21D` |
| Eighteen-test log | `93186D6D6C24244427EDA202DC51FEDC851BBB336143E19C6F38F75788A0E870` |

The actual maintained build script was freshly compiled and executed. Missing
owner root, relative root and changed source were each rejected with the expected
specific error and no generated glue. Valid input generated exactly four imports
and the normal fixture library compiled successfully. Missing corpus and changed
inventory were rejected by the expected input assertions. The actual integration
test list contained exactly the reviewed 18 names; all passed, with no ignored or
filtered cases. The original fixture body and all assertions remain unchanged,
with only the import preamble/crate name and text line endings adapted.

The same bounded policy and immutable dependencies were used. Guarded root and
all descendants retired naturally, exit 0/final active 0, 23 ms drain, no forced
cleanup or memory/abnormal-exit flags. Peak job memory was 581898240 bytes;
minimum available physical memory 19387920384 bytes and commit 17983291392 bytes.
All 26 source, 50 tool and 1746 dependency pins were unchanged before/after. The
old preparations and initial pass were not overwritten. Only evidence prose was
updated afterwards; build script, fixture, assertions and lock remained unchanged.

This proves the maintained ordinary-library/integration-test source path and its
actual build glue using direct rustc with qualified cached dependencies. The
locked Cargo graph was checked separately without a build; no full Cargo or
Clippy qualification is asserted.

## Owner source and corpus provenance

The four actual owner module hashes are enforced by [build.rs](build.rs).
They were qualified independently in the owner's 42-test run at
`adapter-20261115-190743`, with source receipt
`D9A2ACBDEC35DB32946F3C17A67BFE9637023A17BFEE79F17772C5366067B05B`
and natural-pass guard
`CAB7A53CAC7AEDF46C41DDB886474634B99944C741D9DF3BF5207796D045B419`.
That owner run recorded changed source over base
`9d7555d5d23f8f4dc7953e40b60198b7af19a751`; the exact file hashes, not an
assertion of a clean baseline, bind the result. All four remained byte-identical
during the combined run. The owner subsequently committed these same four bytes
as `6369fc47e4bd381d7eebcc63157990811f78c8ef`; that later Git identity does not
rewrite the original dirty-at-test provenance. No owner implementation is
vendored here.

The retained corpus inventory hash is pinned in the fixture. It records source
baseline `984fbaadd75cc5f9d53cb3d6af9ce44fe5ae285d`, actual exporter SHA256
`2BFD727DCEA7374D00503C618208EBFC40967BBC14803938687075E3134418DA`
and producer SHA256
`D7F2B5D325A958455FBEF86D5EAB262C0370E2AB1F616D40DCBD2DD406B6F53D`.
The original three ignored fixture preparations and prior 6-golden/22-sequence
pass remain preserved, including the older unrun/unbound proposals.

## Package graph

Locked offline GNU-target metadata passed without compiling: 120 packages,
102 registry identities/checksums all present in the existing shell lock,
one workspace member, one integration test target and no library unit-test or
doctest target. The isolated lock SHA256 is
`04C139ABD6CDA62A3907C899CA3170154E1B00F52F4B0E570A398ED900540DBF`;
metadata SHA256
`3538A04E8852EF3171ADCBE5EBF2932405E8DEE671DD145AFE3AB856AB33C766`.
This validates package resolution, not compilation of that graph.

No application, version, installer, live clock, backend, hardware or RF action
is part of this acceptance crate. M6 remains unfinished operating software.
