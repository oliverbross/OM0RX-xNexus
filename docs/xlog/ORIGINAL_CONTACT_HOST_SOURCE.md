# Original ContactCard host mount — source qualification

Base `b84610861fb3a02c348f5fbebf2019da11204367`. The original operating host now
opens xLog contacts through its actual Recent and Recall gestures and reuses the
original ContactCard/editor. Source tests, strict TypeScript, the production Vite
bundle and private browser qualification passed as detailed below. No native
application, package, version or production owner data change is included.

## Actual operating gestures and authority

OriginalHost passes the actual Recent/Recall `onOpenLogbook(call,id)` gesture to
a selected-contact owner hook. Event-time admission inspects the shared history
reader's current lease and exact unique UUID membership. Recall passes the
queried call, which can differ from a portable row callsign. Recent uses the
row's call. Neither route searches by call, chooses the first row, or invents an
ID. Compact rows with `details_complete=false` can open by canonical UUID when
the independent contact-read capability is available; only the full by-ID
answer determines editability. Existing default/native projections are unchanged.

`isCurrentReadConnection` additionally requires the same API, live accepted
connection and current capability set. Poll rejection synchronously retires
that accepted read state before React publication. Existing broader connection
identity and emergency rotor Stop semantics are preserved. Station snapshot
expiry does not revoke independent passive log reads. Old captured gestures
cannot retarget a new connection even when its rows reuse an ID.

## Read refresh and draft lifetime

The host retains one contact reader and one caller-owned editor above the actual
ContactCard's presentation body. Selection gets a stable key; loading, lease
expiry and generated timestamps do not cause renewal remounts or focus loss.
Successful reads schedule another nonoverlapping read after two seconds while
visible/open. Hidden state cancels timers; failures pause until explicit Refresh.
Only the latest explicit selection/refresh intent is retained while one read is
pending. There is no mutation queue, failed-read retry or automatic write.

Reader authority still expires at its original pre-invoke deadline and retires
while a refresh is pending. The opt-in host local-typing mode seeds a complete
base only after a fresh authenticated full read/write grant. It is memory-only
until the first actual edit; restored drafts need that admission again. Within
the same live connection/write grant, a withheld/expired read does not drop
keystrokes. Queue, Submit and Retry keep strict current read/base/capability
checks on every intent. Clock faults, capability loss, connection replacement
and known foreign workspace block local typing. A fresh contradictory base,
revision, tombstone or read-only answer also retires typing, preserves saved
fields and requires explicit resolution. No stale confirmation/award facts are
presented as current; retained core fields are the local edit base/draft.

The original ConfirmHost is mounted once. Direct close, keyboard close and
replacement respect busy/dirty/frozen policy. No cleanup effect discards data.
An explicit Resume saved correction control reads the same private draft key;
it performs no HTTP on startup. Only a valid record for the current owner
workspace/capability may open its exact validated UUID, even when that old
contact is outside Recent/history. Lookup spelling is canonicalized without
changing retained base/payload bytes or request ID. Foreign, invalid and unknown
records are never overwritten or imported. This is distinct from row admission.

## Qualification and limits

The owner-only **xLog connection** footer action opens the existing connection
dialog above the retained card. Reconnect is explicit; it does not discard,
replace or submit a dirty draft. The operator closes that dialog and explicitly
refreshes the selected contact to obtain new read/write authority. The footer
action is disabled while the editor is busy and absent from default/native cards.

The following evidence is retained under the isolated worktree's `target/`.
Every completed job verified its source/tool pins, or has a separate preserved
after-failure pin check. All jobs retired naturally with zero active descendants;
there was no forced cleanup or memory-limit/abnormal-process notification.

| Gate | Actual result | Receipt SHA-256 |
| --- | --- | --- |
| Initial proportional 15 suites | 213 passed; one actual Recall gesture timed out. Original test and failure retained. | `110FB2FDF56C98F0799F77A76DF710E1C85DD6059BEB8DF7255CFE53B94E4C91` |
| Controlled gesture follow-up | Both actual Recent/Recall cases passed. Fixture timeout/interval scheduling became deterministic; the original five-second test budget and product source stayed unchanged. | `007208D5F826E5E6F9DCDF9577FACBC8DAD160A996CF606602D3AC6FE808E6C7` |
| Connection-footer regressions | 68/68 across five suites: editor 10, read-only card 14, original native Logbook/card 28, selected-contact hook 13 and actual host 3. | `A5770E7B9DF5E3F80F7F0FABED7CE7D8258C0CBFDD272D848A926F1E2532D314` |
| Final strict TypeScript and production Vite | `tsc -b` and `vite build` passed; 41 bundle files including the production xLog entry. | `03FD87A46E3FC52FBE35F6345869D491B616004E88214783D505AB338DBFF196` |
| Parallel catalog diagnostic | 435 passed; an unchanged RadioSetupWizard extraction exceeded its five-second test budget while TypeScript ran. | `54284BC86B3A942257D69E0BE69E55C8B585C538461ECA8C9EC75AB666B5C3B8` |
| Exclusive serial catalog confirmation | Same source/tests/limits, 436/436 passed: hardcoded 378, placeholders 30, invariants 8, i18n 20. Previously timed case completed in 15.69 ms. | `B05434FC548F988D549D1301616FB9C5282F4EBD727AAE1127ACAF61B320FAF8` |
| Actual original-host browser | Three viewport cases and six Classic/Roster layouts passed, with every contact and RX display assertion. | `6ED2E21D421BDACAC700F150EC388A3C3C10F15F3C9541D2CF465FAA7883767C` |

The focused tests cover exact current-row UUID admission, portable Recall query
identity, synchronous capability/API/session retirement, late A→B→A reads,
hidden/failure polling, dirty/frozen/busy close handling, local typing through
refresh/expiry, known revision/tombstone/read-only conflicts, and same-ID Resume
outside the current Recent page. The 68-case follow-up also proves a nested
connection dialog preserves the actual dirty field, request and storage bytes,
with no automatic contact read, enqueue or delivery on reconnect.

The browser used actual OriginalHost, original panes, original card markup/CSS
and current hooks, with declared synthetic transports. It tested 1366×768,
1366×768 at 175%, and 3840×2160 at 200%. Each case opened exact Recent/Recall IDs,
kept the same multiline textarea through a delayed read beyond its original
five-second deadline, refused writes until an explicit fresh read, retired
write capability, hid private read errors, and reconnected above a dirty card.
It then queued once, explicitly submitted an uncertain request, closed/remounted
the host without automatically reading that contact, resumed the exact saved ID
outside Recent, inspected its queue and reconciled that same request. Only the
matching applied receipt produced the applied status. Existing RX Start/Stop,
needs expiry and genuine serializer-shaped spectrum/render/reset assertions
also passed. No independent native API invocation, external request, page error
or console error occurred. The private browser and its guard both exited zero.
Representative captures include `1-draft-expired-refresh.png`,
`1-resumed-applied.png` and `0-recall-card.png` in
`target/contact-host-browser/screenshots/`; the original card and sticky footer
remain readable and inside the tested windows.

Browser source review caught and corrected three fixture assumptions before any
browser run: comment text can activate a now-clickable history row, a delayed
answer cannot renew its original request-start deadline, and a failed connection
requires explicit reconnect. The modal reconnect usability gap was corrected in
product source and tested before the first browser launch. Fixture syntax was
checked before browser startup. No test deadline, hit-test requirement, geometry
assertion or resource cap was weakened. The initial strict TypeScript diagnostic
(`08A8D2AFCAC1996A0350D66869BC3B5C13C600EAF4A565B5E86467BF99A76262`)
identified two unreachable duplicate status branches; their removal preserved
reachable behavior. All earlier receipts and generated build metadata remain.

The standard guard was `85A7D3DFAEE539E415FA4EA11765F4B922D13ED5D5AABB2C8945876D5128775F`
with NativeJob `139080E0E80FE5822B6618208684F6C5F60EA37507108AF9EDFFB808CC41652E`.
Tests used 2 GiB/300 s/Node 1024 MiB/one worker; full TypeScript/Vite used 4 GiB/600 s/
Node 2560 MiB; browser used 2 GiB/600 s/Node 1024 MiB. All required fresh 8 GiB physical/
6 GiB commit admission, 2 GiB runtime floors and a five-second natural drain.
The final production bundle manifest is
`8330AB3B6FD8594FA5C79868FAFAA45B1A4CC309CEAFA64A640E3A14C57F8897`.

These are frontend/source integration gates, not a full upstream CI claim.
Synthetic browser queue/receipt handling and a React host remount do not prove
native outbox durability, native application restart, actual owner SQLite edits,
installed-app interaction or physical RF operation. Those remain separate
acceptance gates. This slice does not add paged Logbook navigation, contact
creation, received-decode logging, transmission or an independent database path.