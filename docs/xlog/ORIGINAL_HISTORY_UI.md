# Original Nexus history panes backed by xLog

Source development on `7e36f9e7a6e792f8f845ed1e2ea4aef09635e38f`. This
checkpoint is not a packaged, installed or live-qualified application. Focused
actual-pane browser checks and integrated TypeScript passed; complete-window
qualification is still pending.

`ownerHistoryReader.ts` holds one history answer and one recent-contact answer
for a native connection. The host must share this single reader between the
panes: the native history transport's revision fence is also shared. There is
one outstanding native read, no queue, browser HTTP, local log or automatic
retry. Reads use the native connection token, local session generation, owner
identity and exact query. Selecting A, B and A again cannot revive A's original
in-flight request. Raw ASCII validation precedes case conversion; supported
owner aliases do not merge FT8 and FT4.

The reader measures its deadline from IPC request start and preserves the
earlier deadline when producer timestamps repeat. Session changes, newer log
revisions, failures and expiry remove held facts. The host must use
`inspectHistory`, `inspectRecent` or `isCurrent` on its render/timer path; a
previously returned object is not a live lease. Unsupported or incomplete
summary responses are unavailable, not an empty log.

`ownerHistoryPresentation.ts` maps current owner payloads into only the fields
the original panes display. It never constructs `LoggedQso`, supplies a guessed
frequency, or invents per-contact confirmation flags. Full summary counts remain
distinct from the returned page. Confirmation labels explicitly name xLog's
paper-QSL-or-LoTW policy. A partial contact has no editor action.

The actual `RecallPanel` accepts an optional `externalHistory` state. Its
existing card, contact rows, badges, comment controls and layout classes remain.
Explicit loading/unavailable states suppress native history, stale private notes
and old badges. Row navigation retains exact owner contact IDs; view-only rows
do not enter the editor's keyboard navigation. Native callers retain their
existing history path. The original recent pane uses its existing external-read
seam through the same shared reader.

## Focused checks

All evidence paths below are under the isolated `original-owner-history-reader`
worktree. Jobs used the reviewed 1 GiB aggregate limit, 512 MiB Node heap,
300-second deadline, 8/6 GiB physical/commit admission, 2/2 GiB floors and one
worker. They exited naturally with zero owned processes; no application or
hardware was opened.

- `target/owner-recall-check`: 84 tests passed: 35 reader boundary cases,
  four projection cases, seven external original-card cases and 38 existing
  Nexus card/keyboard/comment regressions. Receipt SHA256
  `E14FF75FD04DD427BC1487F18ECEF87C24E401E9E0B2C3C98632B4851F42DB34`;
  guard `C91A866DB37BE8E772DCB613B2A6CA4062CB9810C3CB194EC25AFFB51B16799F`.
  Peak aggregate memory was 484,569,088 bytes. All 2,560 source pins stayed
  unchanged.
- `target/owner-history-producer-check`: eight tests replayed seven actual
  synthetic Swift projector payloads and both retired-workspace refusals.
  Receipt SHA256
  `7DDC4B079A2B8D111F58FA5248B2128AA33F02CA8C2B8843D568D1D5B45DB3E6`;
  guard `D805D56F277F5D1D2630712FA7981CCCCD0036C4A4713B0F6C4CBC7AC9A9AF13`.
  Peak aggregate memory was 261,332,992 bytes. Producer bytes and timestamps
  were unchanged. Only an explicitly synthetic public identity/IPC wrapper and
  monotonic test lease were added. This historical contract replay is not a
  current native session or end-to-end live HTTP result.

The preserved initial 30-case reader pass preceded review of ASCII query
handling. The corrected 35-case suite adds Unicode refusal and supported
hyphen-call regressions; no expiry or identity policy was relaxed.

## Original-pane browser checks

The combined original-host source passed `tsc -b` and the existing-entry Vite
build with the additive history/station-identity SDK types and spectrum reader.
Evidence is under the `original-xlog-host` worktree at
`target/host-types-vite-28e94c378e0041df91f1cad0abf0755c`; receipt
`5679771FE25F3965136C987516ECD32212C74E1867FBEC623AC1B3CCB4B05A55`.
All 2,589 source/tool pins stayed unchanged; the 4 GiB job exited normally with
zero descendants. This type check includes the new host, but the production
entry had not yet been switched to it. It is not an installed-window check.

The real `RecallPanel`, original stylesheet, viewport hook and owner projection
passed 12 owner-state cases and three distinct native baselines in each of two
configurations. Native records and owner records used different callsigns, modes
and contact IDs. Pointer checks covered exact editor IDs, view-only records,
comment expansion, visible dates/modes and restoration of native geometry.
Native IPC and external network access were trapped; no production data or
running application was used.

- Single remaining pane: `target/recall-browser-preparation/attempts/attempt-e69423f83d984efc853ef2dc23298003`,
  receipt `DA4EDE3D15A9556AD35C12CA007948C964A383B44463903DC38650FD0B12796F`.
- Default three-column Classic layout:
  `target/recall-classic-browser-preparation/attempts/attempt-d1bc61543b754b6598119068172714c3`,
  receipt `5CBAE6BBD396664F1448C27E1AE3AC577FCE3141E3D2FAB930CC20A4F21CD429`;
  results `18B8143759681E3542ACF483F1D0101912D71FC44BB45BBC22B4D070BEFC2746`.
  The original grid produced a 338.547 px laptop rail and a 480 px UHD rail.
  At the scaled floor, the original stylesheet stacked the columns; its scroller
  reached the card and the card's own scroller reached its records. Initial
  top-of-card and post-interaction screenshots are both retained.

Both jobs used 2 GiB aggregate memory, a 1,024 MiB Node heap, a 600-second
deadline, 8/6 GiB admission, 2/2 GiB floors and a three-second natural drain.
Browser and supervisor exited normally with zero remaining owned processes.
Peak aggregate memory in the Classic check was 1,247,629,312 bytes. Product
source hashes stayed unchanged. The other grid occupants were inert fixture
elements: these checks qualify the card's geometry, not the complete cockpit.

## Actual producer fixtures

The checked-in JSON files are unchanged copies of the synthetic exports from
xLog's `CompanionHistoryProjectionTests` and `CompanionContactTests` (12 tests
passed), evidence directory
`C:/Projects/OM0RX-xLog/.build/xnexus-history-tests/swift-tests-20261115-125509`.

| Fixture under `ui/src/xlog/fixtures/` | SHA256 |
| --- | --- |
| `owner-history-main.json` | `D783191924D1728429C6A2BB68DBC244F88CEAB5D77A2B2D349FC640B9596116` |
| `owner-history-lifecycle.json` | `46D7D1FD8E803018418B4A1F82FB930BFB0B91F1C4F82AB0B890861131BF3545` |

They include a complete 30-contact summary with a 25-contact page, recent total
31, a UTF-8 comment whose incomplete details forbid editing, empty/band-only
reads, edits, deletion and retired-workspace refusals. No production log was
copied. The additive SDK DTO overlay used for the focused checks was frozen at
SHA256 `7B349A8735059BACB1FCB897343B3697EBD30089565AAFAD3307D16EB042FBFF`;
it predates canonical station identity additions and must be merged additively,
never replace the canonical API file wholesale.
