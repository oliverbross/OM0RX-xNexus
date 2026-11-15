# Decoded-call classification verification — 14 November 2026

This adds xLog-owned classification to native FT8/FT4 receive rows. It does not
introduce a second logbook, country resolver, scoring engine or station owner.
The [wire contract](NEEDS_QUERY_CONTRACT.md) remains the source of field and
authority rules. These are development checks, not installed or physical RF
acceptance.

## Owner and client evidence

The xLog owner implementation is isolated commit
`8bd486abd2e62aa079e8b4248126e1e186561452`, based on canonical `72c4f009`.
Its authenticated, read-only batch endpoint uses a single Session mailbox,
carried only by an already scheduled ordinary cluster poll. HTTP requests do
not create writers, workers, station connections or additional polling.
Warm retained logbook and country indexes provide call/DXCC statuses, score
and reasons. A mutation fence rejects facts that span a log change.

The production-source Rust harness passed 102 tests. Swift passed 44 tests,
including publication and mutation races, with a second export-enabled relink
also passing all 44. The full native host passed 1,173 tests with two existing
exclusions. Its initial build/test job required cleanup of a remaining
descendant; a separate run of the exact compiled test binary passed all 1,173
tests and exited cleanly with zero owned processes. The initial discrepancy
and earlier resource/compiler failures remain preserved, with no inferred
cause. The owner's review record contains exact receipts and hashes.

The SDK is commit `a6b9b4a5bbd458219aeb6d0ef25f63e2c9e033df`, including
the preceding `b2f7596` and `e5299ed` changes. Eighty tests and all-targets
Clippy passed. One explicit replay test separately accepted all 40 unchanged
Swift request/reply pairs: 21 ready, six refreshing, one stale and twelve
unavailable, covering FT8 and FT4. The manifest hash is
`bcf943b4684f701eeabd5f7a0348596398746878722f5696981e061ea7b6f0b0`.
The replay adds a labeled synthetic HTTP identity wrapper, checks each payload
at its original generated time, and then requires rejection against the current
clock. It is cross-language evidence, not live transport evidence. Missing
directory, wrong manifest and wrong count controls fail before running Cargo.

Optional lookup failures revoke classifications without stopping otherwise
valid audio. Authentication, identity and protocol failures revoke the shared
connection. Original monotonic leases cannot be renewed by cached snapshots.
Absent optional metadata supports older xLog versions with Unknown results.

## Native attachment

Native commit `d82c0d58e514dba1b0f5005e430ba937de0866d1` passed 19 focused
tests against that SDK. Each decoded row has an immutable identity and complete
worker, server, workspace, audio route, frequency and decoder-mode context.
Only an unambiguous literal sender parsed by the existing message parser is
queried; free text, reports and hashed senders remain Unknown.

A separate bounded lane sends at most 32 unique queries per batch and one batch
per second. Fair scheduling serves unattempted calls before revisiting earlier
ones, within the 200-row bound. A coalesced notification follows accepted owner
metadata; no extra owner poll or lease extension is added. The existing owner's
one-second wait occurs after each completed cycle, so a one-second HTTP deadline
does not guarantee success. Missing a cycle yields Unknown or a bounded error.

Durable Pending/NeedsReceipt operations for the current workspace block new
classifications until settled, including across reconnect/restart. Enqueue and
delivery revoke outstanding facts before and after the operation. Trimming,
context changes, expiry and revocation discard late attachments.

The tests include a real synthetic HTTP/SDK lane held while PCM and independent
Stop continue, actual worker Stop/restart fencing, durable queue reconstruction,
fairness across 80/200 keys, timing boundaries and strict parsing. The guarded
2 GiB job peaked at 846,770,176 bytes and confirmed root exit and zero owned
processes. Evidence directory:
`.build/xnexus-native-needs-evidence/attempt-8a9a8c5fb2b44f5caaac74f459440a07`.
Compile receipt SHA-256:
`32ba14e94214e8f2e11f63a1c9bc460cc2e50f462c5fd61c515a30d2cf70882e`.
Guard receipt SHA-256:
`434a07aa76c80ed97a1617c6d13c3cf768dab8afe0d5761309fe87eb05977f54`.
The exact Rust-serialized fixture supplied to UI tests has SHA-256
`00a4efc0828fbc78cd88aa8743ce5ec5ddeea316b726a8d504685e66320fb3dd`.

## Frontend evidence

UI commit `09b31a62feab0ee9b100179c42ac8058cf1803ce` displays xLog's status,
score and optional evidence independently from the decoded signal measurements.
It matches immutable row identity, current owner/route and decoder mode. Its
monotonic deadline starts before the native status request, so IPC delay cannot
renew evidence. Expiry works even while IPC stalls. Pending Start/Stop actions,
capability loss and context changes revoke displayed facts; delayed responses
cannot restore a cancelled action's classification.

All 55 companion UI tests passed, including 15 new needs cases and the exact
Rust-serialized fixture. Full TypeScript and production Vite checks passed in
the same guarded job: 7m11s total, 2,585,292,800-byte peak under a 4 GiB aggregate
cap, with a 2,560 MiB V8 heap. Root exit and zero owned processes were confirmed.
The final receipt is under the UI worktree's
`target/needs-ui-check/build4-55a2b84656804bd590970bf4fe4ed96b`, SHA-256
`23023cb3583e03c43cef4c6046ff3e7abe6b83637ec645dd1d1e70e0803998df`.
Earlier heap exhaustion, three test-only Promise type errors, and one history
timeout remain preserved; the final run passed without a test-timeout override.

The actual production bundle passed 14 private-browser viewport/theme cases,
expiry/capability-loss/no-auto-command checks and a negative geometry control.
Expanded evidence was visually inspected at 1366 pixels and at 1024 pixels with
175% scaling in both themes. The data area scrolls while the connection bar and
rotator Stop remain visible. Source and bundle hashes were unchanged, and both
browser jobs confirmed cleanup. Main browser receipt SHA-256:
`c21804987c1e14794c87c9eed5f1d6b9bbb9a4d76f98207608e4bbcde76c38e6`;
focused visual receipt:
`fa99f1a2e79a8d6e3c8c8f389489819a3f9c8a4f7973b5045370933bbe3e8f85`.
These use a synthetic browser-side bridge and do not establish native IPC or
installed xLog behavior.

## Integrated Windows compilation

Exact integrated application source
`0d01248b0504ae77f6f122fdada1b21e4a61806b` passed a fresh frontend build and
the full pinned GNU Rust 1.93.1 `cargo check --locked --offline --features radio
--all-targets -j1`. Cargo reported both application and test `tempo_lib` targets
with `fresh=false`; checking took 6m07s. Source, frontend asset inventories,
prior receipts and baseline library hashes remained unchanged.

The 4 GiB aggregate job peaked at 3,671,310,336 bytes; minimum available
physical memory was 5,214,150,656 bytes and commit headroom 6,223,089,664 bytes.
The root and all descendants exited naturally. This is compile evidence, not
linking, packaging or GUI evidence. An earlier admission refusal is preserved.
Evidence directory:
`.build/xnexus-integrated-native-check/attempt-1e0c23b1f2f4412e8401f263b19984cf`.
Receipt SHA-256:
`4acaf46bdba3d5b06972b4e6d479b97d9d4892c813c29129059be81a9f8282be`;
guard SHA-256:
`6d55d60ac7e2ca9925e99f7dd3ac2c4db33c8af12a1d3ef82b21524f8f4f85dc`.

## Generated FT4 through the actual decoder

Test-only commit `82f08245ca6b425e933ff1f26863a3e29e99a8b8` preserves the
integrated production bytes. Its paced generated FT4 signal crosses the actual
48 kHz owner buffer, HTTP endpoint and needs validator, SDK, worker poll,
assembler and decoder. The decoded K1ABC row receives score 100 from an
explicitly synthetic classifier. A durable pending operation clears facts;
settlement permits new evidence; original authority expiry clears facts while
PCM remains fresh. Stop clears rows and permits exclusive restart.

The test passed in 16.85 seconds under a 2 GiB job cap, peaking at 890,462,208
bytes, with natural root exit and zero owned processes. Guard evidence:
`.build/xnexus-needs-chain-evidence/attempt-22da3308c6794946b9f2380f437a74fb`,
SHA-256 `058706f2716a1062844d362482d6969d357f91f90798f667f1d8a99e04d63727`.
Maintained runner receipt under the test worktree's
`target/xnexus-rx-needs-chain/acfee72a16f841dfb49820d7062c3141/receipt.json`,
SHA-256 `a0b0860422e1cf73e89b19de37ad3a7b8218de0bde84c514c4e9f889ada0f069`.
This closes the decoder-to-lookup test gap; it does not run Swift classification,
a live database, native GUI or physical audio hardware. The actual Swift wire
replay above remains separate evidence.

The [Windows `.4` build and artifact audit](WINDOWS_BUILD_1.24.1-xlog.4.md)
subsequently passed at exact source `ae35fc6435f861e38029049299833afe57de4885`;
paired native acceptance and physical DAX reception remain separate gates.
Canonical xLog 1.6.27 at `16cac86739274073ab2f8000a3916b4e37bc57a5` contains
the reviewed owner source. Its owner reports packages, installation and installed
qualification passed; current documentation is `7789c569`. Normal installed
handoff passed and left xLog connected, but Settings automation stopped before
pairing or capture. The installed endpoint still requires
paired UI and physical station acceptance.
