# Original Nexus operating host — source qualification

Recorded 2026-11-15. The normal xLog-mode frontend entry now mounts
`OriginalHost`, using the real Nexus TopBar, ModeNav, OperateCockpit, Classic
decode panes, Roster, Waterfall, Recall and Recent components. It imports the
original visual stylesheet followed by the original cockpit structural sheet.
There is no replacement cockpit layout or custom dashboard stylesheet in this
entry. The previous diagnostic dashboard source remains in the tree but is not
reachable from the production xLog entry's static bundle graph.

## Authority and lifecycle

The host does not mount the upstream App or construct its station engine. Its
component-scoped services use xLog's native connection and receive worker.
Mounting observes status; it does not pair, start audio, tune, transmit, open an
independent log, or start upstream station feeds. RX Start and Stop are explicit.
FT8/FT4 decoder choice is local and does not change the owner's radio mode.

An external operating presentation type carries nullable owner readbacks instead
of fabricating an engine AppSnapshot. Saved station call/grid are displayed only
as current owner settings; absent identity stays unknown and never authorizes
TX. Owner dial, PTT, rotator, timing and signal facts remain unavailable when
unknown or stale. Existing native AppSnapshot consumers keep their upstream
behavior. Unsupported controls remain visibly disabled.

Decoded entries retain immutable native IDs and exact host-period timestamps.
Owner Needed/Worked/Unknown facts and score/reasons remain distinct from decoder
SNR, DT, audio frequency and message. Classification expires without renewal by
rerendering. The original waterfall receives real native spectrum bins through
the bounded reader and clears on the reader's context/lease reset; it does not
invent quiet frames. Start intent keeps spectrum admission suspended until a
subsequent successful worker observation. Stop suspends it synchronously.

Recall and Recent share one owner history reader. Scope/query changes retire
old answers; an in-flight request coalesces only the latest desired query.
Successful visible reads refresh without overlap. A failed cycle waits for a
new selection/session or explicit refresh. Hidden pages do not dispatch the
next history/recent read. Owner summary completeness and the paper-QSL/LoTW
confirmation policy remain explicit; thin contact rows are not fake LoggedQso
objects and receive no editor authority.

Ordinary controls synchronously consume their observation lease and reject a
second pending dispatch. Polls started before or during a command cannot restore
control admission; a poll started after completion is required. Rotor Stop stays
independent. Actual command receipt status/reason is displayed without promoting
an accepted/applied receipt to physical completion or retrying the command.

## Verification

Source was qualified in the isolated `original-xlog-host` worktree at baseline
`ff59ec067be461b9f6746eff99715e192abaa0ff`, with separately owned history and
spectrum overlays. All 15 borrowed files match canonical
`e7599669335c0eb2ebad37c4fc9c09118dc9b7a6` normalized Git blobs and are excluded
from this host change. Source inventories and before/after hashes are retained
with the local receipts below. These are individual gates, not a sum of unique
tests: some focused regression groups overlap.

| Gate | Result | Receipt SHA-256 |
| --- | --- | --- |
| Owner model/decode/connection/spectrum/history hooks | 34/34 | `0A5601D1AAC587C6AAB3DF9C09B30CF48276374F11CF57A7CF48DF1C27E9867D` |
| Connection fences, native/owner tuning, RadioTuneNote, wheel and catalog guards | 449/449: 5 + 20 + 17 + 31 + 376 | `ECED484B505063586928D5E8EAF3C55FE30A0D302206C56FD8DA616CEB2205D6` |
| Full integrated `tsc -b` and Vite build | Passed; 2,589 input pins unchanged | `5679771FE25F3965136C987516ECD32212C74E1867FBEC623AC1B3CCB4B05A55` |
| Actual original-host private browser | Three viewports, six Classic/Roster layouts passed | `4288A784ED95E48F459F3EADCB684A5508262FA9CC27B753AED87ADF5D7FDE8E` |
| Receiver lifecycle | 5/5 | `A4ABB83766ED9C6B508B5E1288BD69976068535379652B1E8A24DB7CE129C93C` |
| Final production entry Vite build and module-graph proof | Passed; 2,591 input pins unchanged | `EB956A8B9EDB78E53E682C74F2E15161441FAA127C91F3CD35BB102AFBEE7EDB` |

The browser used the repository's LAPTOP, FLOOR and UHD_200 sizes, the actual
components/styles/useViewport, isolated storage and synthetic service fixtures.
It verified no automatic RX start or native fallback, explicit RX Start and
latched Stop, a Rust-serialized spectrum frame and its original lease expiry,
Needed-to-Unknown expiry, the original decode-field and Stations-selection
gestures, read-only Recall, and Recent enabled through the actual Panels control.
External traffic and mutation calls were refused. Screenshots are preserved in
`target/host-browser-call-roster/attempts/attempt-2409c37a196449dca5456554b2a66b7c`.
No desktop-control helper or global mouse/cursor API was used.

Four earlier browser failures remain preserved: an incorrect assumption that
Recent was initially visible; an incorrect assumption that decode single-click
opens Recall; a pre-admission canvas baseline race; and a Roster selector error.
Corrections were confined to the probe, preserving original gestures, the strict
400 ms image-change check, and the frame's 499 ms lease. The two earlier
TypeScript diagnostic failures also remain preserved; subsequent source fixes
passed the full integrated check.

The final build inspects the real configured `xlog.html` Rollup graph using a
read-only audit plugin. `assets/xlog-OlJNxAuf.js` statically reaches OriginalHost,
OperateCockpit, TopBar and the owner receiver/spectrum/history adapters, while
`ui/src/xlog/App.tsx` is absent. Graph proof SHA-256:
`88C629F9CC61EEE4B4FABF53FB0CCFC024C9D26584C3623FAD4955F77CCF8BA7`.
The resulting file-manifest SHA-256 is
`FC824E767E39BE8D2286285FE9FDA8F87CD26BC920A63FB5442646140D7BD678`.

All completed guarded jobs report natural root exit, zero remaining processes,
no forced cleanup and no resource-limit notifications. Focused tests used a
1 GiB aggregate cap; the private browser used 2 GiB; full frontend compilation
used 4 GiB with a 2,560 MiB V8 heap. Existing bundle-size warnings remain.

## Remaining qualification and software scope

This is frontend source/browser qualification, not a packaged or installed
candidate and not physical radio/audio acceptance. It changes no product
version. A subsequent native candidate must preserve the reviewed startup and
private-profile boundaries and qualify the actual default entry and paired
workflow before installation. The full upstream CI suite was not rerun or
claimed by this proportional frontend validation.

The original host currently exposes RX, owner classifications, bounded passive
history/recent contacts, explicit tuning where authorized, rotator observations
and independent Stop. It does not yet provide the original QSO editor through
the durable xLog outbox, rotor Point/reconciliation workflow, all other Nexus
views/propagation/cluster operating surfaces, or an owner-authorized TX path.
Those are remaining software integration tasks, not merely physical acceptance.
The disabled original controls do not imply those paths are implemented.
