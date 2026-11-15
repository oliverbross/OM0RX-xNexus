# Actual owner cache → native decoded-needs acceptance

The opt-in `owner-needs-cache` fixture feature adds `needs_delivery:
"scheduled-cache"` to a marked generated-FT4 fixture. Existing direct fixtures
remain the default and do not compile the newer owner's `QueryCache` API.

This acceptance path uses the actual xLog HTTP bridge, receive buffer, needs
validator, `QueryCache` and `Mailbox`. An independent ordinary poll runs every
1,738 ms; HTTP requests never wake it. Its synthetic classifier produces the
correlated `needs_reply` envelope consumed by the real mailbox. Classification
runs only on that poll, never on cache reads. The classifier is **not Swift or a
real logbook**. Audio is a generated `CQ K1ABC FN42` FT4 period, not hardware PCM.

The actual SDK, assembler, decoder and native Worker must produce a literal row,
receive a real pending HTTP 503, retry its immutable ID and attach Known facts.
The test then makes needs unavailable, verifies Unknown while PCM advances,
restores fresh Known facts, stops the Worker and closes the fixture normally.
Each successful cache result must exactly equal a recorded poll result, retaining
the original generation timestamp and 2,000 ms TTL. Both known generations must
have earlier same-ID pending requests. The SDK timeout remains 1,000 ms.

## Exact-source execution on 15 November 2026

- xNexus integrated application source: `9383d286c480fd94820cf63a6651ef6d6f717076`
  (includes pending-group runtime and reviewed UI corrections).
- Owner source: clean detached `979b7ff748992c73ee99571eabb65b0a5e68a20e`,
  under `.build/xnexus-worktrees/needs-cache-owner-979b` in xLog.
- Rust: pinned 1.93.1 GNU; source/retained rlib hashes checked before and after.
  `scripts/xnexus/test-needs-cache-chain.ps1` compiles the acceptance path only;
  invoke it inside the reviewed private Job guard, never unguarded.
- The whole-repository no-product-diff comparison includes UI sources; only
  this acceptance subtree and its specific runner are excluded.

The first BelowNormal attempt `attempt-d0ab1ca0671c43aca283e54e237c7ad7`
compiled successfully and passed the callback-wrap test, but failed before
classification: synthetic `first_frame=20160`, `late_us=598069` exceeded the
unchanged 250 ms pacing guard. A later snapshot request returned Transport.
Its failure remains preserved; no exact scheduling cause is claimed.

One separately reviewed **Normal-priority execution-only replay** reused those
exact compiled binaries, rlibs and waveform without source changes or compilation.
The xLog 1.6.31 Host build was concurrently active under its own 4 GiB BelowNormal
guard; this was not a quiet-machine run. The replay retained a 2 GiB Job cap,
8 GiB physical/6 GiB commit admission, 2/2 GiB runtime floors, five-minute limit,
three-second natural drain and the original 250 ms pacing bound. Captured Job
member identities and actual PriorityClass verified headless PID 21580 and
generated owner PID 33432 as Normal.

Replay `normal-headless/attempt-d9a712e7ec0d4c40be459cc41353045a` **passed**:

| Event | Elapsed from receive start |
| --- | ---: |
| Actual FT4 row classified Known after pending retry | 12,494 ms |
| Unknown with continued PCM | 13,520 ms |
| Fresh Known after renewed owner evidence | 16,605 ms |

There were four queries and two poll classifications. Two distinct generation
timestamps proved same-ID pending-to-ready recovery. Root exit was 0; initial
and final descendant counts were zero, natural drain/cleanup confirmed, no
termination or resource/abnormal-exit notification. Peak Job memory was
182,161,408 bytes. All source, dependency and artifact hashes were rechecked.

Evidence root (not committed, contains private disposable pairing material):
`C:\Projects\OM0RX-xLog\.build\xnexus-needs-cache-chain-evidence`.

| Evidence | SHA256 |
| --- | --- |
| Original failed guard | `B268BB5DB5ECB8CAF96B007E953CA3F03E08A123F4F9B9C2F3B12301CCEC1728` |
| Original compilation receipt | `B0CF9B4525B20B0B9FD8ED69554B5B2C4649E006652D0F38768B165CAE5279B7` |
| Reused artifact inventory | `F3A1E26E1578A34380275C2C3FFCB66D63CA231CCDD25C7942D7F7E5ED9ED731` |
| Passing Normal guard | `03DFD4DC8FEFDF041F9622EBDA73DECF509F5AE82BE9371A1AAD1EB1DAA6326C` |
| Passing execution receipt | `B5329F4795C5B4394500E71305199428996864F37DFDB7A18BDFE7B25A232997` |
| Passing headless lifecycle | `9082EA9D52336513D889B04BBABD1583D2B0150121A750909CA1DFF71D61016D` |
| Cache-chain assertions | `38C4D0AB12DF0754119816082537674B6E84A29BBB357B19C9107A608B2807B0` |
| Compiled owner | `58566ADDFC823E5A655DAC5F71794F940BA68D706E52BA7F4166B7BEE68456D1` |
| Compiled SDK | `4F2C6BE9EE7AD57525769E3A8B31DEA191855FE8ABBBD0CF83D4520C0E1FD6C5` |
| Compiled native headless Worker | `3FE183EB5D021D32743F9EA37D9CDC213B28E692C6D49E9B88B79BF6036CFF79` |
| Generated FT4 waveform | `CBD61186133FB9B8CAB0F16F550E993588246D1F4241F4BC47DFDDFB6B2196A7` |

This proves the cache/mailbox transport-to-native attachment path with synthetic
classification. It does not replace actual Swift classifier tests, full native
application compilation, UI acceptance or live DAX/radio qualification.
