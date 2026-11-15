# Original Nexus cockpit: first service boundary

Service-boundary draft based on `75a59f786ee1f437e7ab14a147bb3ca0ceed1c49`.
The bounded validation below is complete for this seam; no integrated owner host, native application launch,
package or version change is part of this slice. This is not a claim that an
integrated cockpit can yet mount.

The next user-facing application must use the actual Nexus operating interface,
including its Classic/Roster layout and chrome. The existing companion dashboard
is diagnostic material, not the replacement operating interface.

## This slice

`ui/src/components/operateServices.tsx` introduces a finite, typed, component-scoped
service surface. `OperateCockpit` supplies it to its actual existing descendants.
Without an adapter, each operation invokes the original API. With an explicit
adapter, missing operations reject; they never fall back to the original API.
Write services also require the matching action capability, including shortcut
paths. Callback actions are guarded separately, and receive marker movement does
not grant TX marker movement. Emergency stop has its own capability.

The existing cockpit JSX, CSS, panes and Classic/Roster selection are retained.
The first converted children were `Waterfall`, `OperateDecodes`, `OperateRoster`
and the cockpit's recall card. The descendant boundary below extends that seam.
No global IPC interception or original `App`
mount is introduced. The host must keep its adapter stable within a session and
remount the subtree on session/workspace/route identity change; asynchronous
owner results still need their native/session validity checks before delivery.

Draft tests render the real cockpit in both layouts, exercise F6, Alt+6, Escape
and receive/TX marker callbacks, and compare denied, explicitly provided and
default upstream routes. They also check missing-service rejection and exact
result forwarding. A canvas-only test double avoids pretending jsdom validates
the real waterfall. The initial 19 adapter/descendant cases passed under the bounded frontend runner.
The exact validation checkpoint below records the remaining gates.

## Descendant/chrome boundary

The draft now routes the table below's direct operations through the same typed
boundary: original `TopBar`, rotor, tuning/split, recall/call cards, meter readers,
audio/RF scopes and relevant wheel/geography/privilege hooks. The provider inherits
an outer adapter when no override is supplied; an inner cockpit cannot accidentally
restore native services. TX, mode, source, radio, log and unsupported method controls
gain disabled states in their original markup. Original mode pills retain their
layout but an injected adapter must enumerate actual supported tiers.

`LiveMeters` retains the independent engine's existing bus by default. An injected
reader never subscribes to that bus or adopts its cached readings; it is single
flight, expires a stalled reading, and renders missing data as unavailable. Entity
centroids and callbook results likewise do not consume the independent engine's
cache. Rotor Stop remains reachable even without an azimuth or a fabricated Nexus
settings record, and does not require an unsupported satellite-stop service.

Two additional hidden independent paths were found in the original Classic station
card: selected-station propagation polling and the Best times menu. Both are
explicitly unavailable with an injected adapter until a real propagation boundary
exists. They do not fall back to Nexus's feed engine. `StationCard`'s Work/double
click and QRZ fallback are guarded/routed too. Local decode-table erasure can still
clear the display without pretending to send an unsupported upstream UDP erase.

The descendant tests cover inherited providers, original top-bar disabled states,
separate dial/VFO capabilities, rotor Stop without telemetry or satellite services,
unavailable meters, the supported mode list and source-switch retirement. All
19 new cockpit/descendant cases passed in the focused run.

Groupable source scopes are: initial cockpit/data-service seam; descendant/chrome
service routing; visible capabilities and cache isolation. Keep them reviewable
when validated; no untested source is committed. No stylesheets changed.

## Boundary inventory before any host mount

These actual surfaces now have direct service routing, but their data still needs
the truthful owner adapter below. They must not be replaced with lookalikes:

| Original surface | Remaining owner/service dependency |
| --- | --- |
| `RotorStrip` | `readRotator`, stop, declination, settings and satellite tracking calls. Point/Stop can map to current xLog owner controls; satellite commands are unsupported. |
| `TuningStrip`, `SplitControl` | Frequency/RIT/XIT/VFO/split operations. Current xLog tune is simplex; no assumed split/VFO support. |
| `CallStatsPane`, `RecentQsosPane`, `RecallPanel` | Owner history plus `LogSource` answers; callbook/external links remain separately explicit. |
| `FastGraph`, `RfScopePane` → `PhoneScope` | Real audio power/spectrum and RF scope, with unavailable states. RF scope is not an audio FFT. |
| `TxAlarmBanner`, `OperateQsoStrip`, meters | Alarm/attention/meter service calls; TX controls must be visibly unavailable until an owner TX contract exists, with stop behavior independent. |
| geography hooks and call-card menus | `getDxccEntityLocations`, call statistics/callbook and menu actions. Local geography cannot supply owner award classification. |

`LogSource` already has an intended injection point in
`ui/src/features/logSource.ts`: install an xLog-backed source before mounting
views. It must refuse unsupported questions rather than invoke `askLog` or
materialize another log database. Exact current revision-bound history can feed
supported answers; log writes must retain the durable outbox and immutable IDs.

Capability guards and visible disabled states have focused adapter coverage;
integrated typechecking and the selected original browser geometry gate passed.
An actual owner-bound host still requires its own validation. Actual
buttons, shortcuts and menus must agree before the integrated host is enabled. The host
must never pass normal `App` command handlers to a companion cockpit.

## Truthful operating view-model needed

`OperateCockpit` currently requires `AppSnapshot`. It includes engine-owned
`mode`, `radio`, `link`, QSO/contest state, stations, conversations and other
facts that cannot be invented from the existing companion snapshot. A narrow
cockpit view-model with explicit absent/unknown states is the next required
source change. Do not cast a partial owner object to `AppSnapshot`.

| Fact | Available authority and required treatment |
| --- | --- |
| Saved station call/grid | Proposed owner `station_identity` schema 1: source `saved_station_settings`, state `current`/`incomplete`/`unavailable`, nullable `mycall`/`mygrid`, nullable binding containing workspace/worker generation, settings revision, identity epoch/revision and `operating_module: normal_logging`, `transmit_authority: false`. The separate SDK/owner-producer source validation passed; this owner feature is not installed. Display only when its binding is current; no operator-call or OM0RX fallback. This is not contest/TX identity authority. |
| Radio dial/mode/PTT | Current snapshot `station.frequency_hz`, `mode`, `transmitting` are nullable and freshness-bound. Do not turn null PTT into false, or a missing dial into a default amateur frequency. |
| RX route/mode/period | Existing `RxView` and immutable `RxRow.context` bind server/workspace/worker/route/mode. Local decoder FT8/FT4 mode is distinct from the radio's DIGU mode. Host timestamps have explicitly unknown UTC/RF delay qualification. |
| Decode need/worked | Existing owner `RxNeeds` contains status, nullable score, reasons, authority and remaining lease. Original `DecodeRow.worked: boolean` cannot represent Unknown. Extend the actual row/history/roster data contract, preserving immutable decode IDs and expiration; never encode Unknown as an unworked/new award. |
| Decode SNR/DT/audio offset | Existing native rows provide these measurements directly. They remain separate from owner award score. Do not claim directed-to-me without a valid station identity and parsed addressing. |
| Waterfall | Original `getSpectrumRow(transmitting)` returns real `Spectrum`. Current RX worker exports decoded rows, not spectrum. Add bounded native FFT output from the same xLog-owned PCM, with epoch/freshness reset; no browser PCM, device acquisition, zero-filled fake quiet band or `SharedEngine`. |
| Sequencer/TX/QSO | There is no owner FT8/FT4 TX sequencer contract yet. Null/unsupported is explicit. Do not fabricate `txAllowed`, tune/ATU, auto-sequence, split, own transmitted rows or completed QSOs. Preserve original controls/layout but disable unavailable actions truthfully. |
| Rotator | Current owner azimuth/motion and stable target identity are nullable; Stop must remain independent of normal queued operations. |

## Original application chrome, not a diagnostic header

Reuse `TopBar`, `ModeNav`, `.app`, `.shell`, original `CockpitHeader`, original
panel layout/state and `cockpit-panes.css`. Their existing sources are the visual
contract; no copied/recreated toolbar. `App.tsx` around the `.app`/`.shell` render
is the composition reference, not a safe startup entry point: its effects own
Nexus settings, feeds, mode/source changes and snapshot polling.

`TopBar`'s `appVersion`/`repairClock` and meter reads now use the scoped boundary,
but its radio types still assume engine facts. The nullable view-model must
preserve the original markup. `ModeNav` can keep its existing
enabled-feature contract; unimplemented views cannot silently start original
station services. The full original Logbook/Connect/propagation views need the
same service inventory before enabling their navigation.

## Next executable slices

1. Finish explicit descendant and chrome service boundaries plus visible
   capability states. Run focused default-upstream and denied-action tests.
2. Introduce the truthful nullable operating view-model and owner identity DTO;
   carry existing immutable native decode/need facts through the real tables.
3. Add bounded real spectrum service and xLog `LogSource`/durable write adapter.
4. Mount the original Classic/Roster host at the companion entry point, with
   original chrome and no `App`/station-engine startup. Prove no unhandled native
   commands using an invocation-denying browser fixture, then real private native
   acceptance. Preserve the previously qualified private storage boundary.

The parked history/archive dashboard drafts are not part of this restoration.
No original runtime, canonical candidate or installed application has changed.

## Nullable model/host implementation plan

Keep `AppSnapshot` as the independent engine's truthful upstream API. Do not add
a function that constructs one from xLog with zeros/false/null engine defaults.
Introduce a separate operating input for the original components, with an explicit
`native` or `xlog` source and typed observations. Refactor the actual components
to read that input; the native branch forwards its existing snapshot unchanged.
The xLog branch should contain only:

- Saved station identity and its current binding from the proposed owner DTO.
  Missing call/grid renders unavailable and cannot generate attributed TX text.
- Nullable dial, band/mode, PTT and rotator observations from a fresh paired
  snapshot, carrying the existing server/workspace/revision binding.
- The actual native RX view and immutable rows, plus their separate remaining
  owner-need lifetime. Existing `RxPanel` lease helpers can become shared data
  helpers; its visual panel is not reused as the operating screen.
- Local operating preferences and draft/selection state, explicitly local:
  Classic/Roster, pane arrangement, selected call, receive-marker choice and
  unqueued QSO draft. They are not radio readbacks or completed contacts.
- Explicit unavailable TX/sequencer state until the owner contract exists; no
  invented run, QSO, txAllowed, tune, ATU, synchronized-clock or confirmation facts.

The primary type change is at decode history and the real Classic/Roster tables:
retain native decode IDs/context and an explicit owner-need object whose Unknown
state cannot satisfy unworked/needed filters. Preserve existing upstream boolean
semantics on the native branch. Do not downgrade the current SDK/worker lease
validation into a wall-clock age test, and do not let merged/retained history rows
keep an expired score. A route/session reset clears history and waterfall together.

The host composition is original `TopBar` → `.shell` / `ModeNav` → real
`OperateCockpit`, with original panel-state hooks and styles. The separate
`LogSource` provider is being prepared by the parent; this worktree does not edit
that module. Owner history requires a truthful completeness/confirmation contract
before the full recall/page answers can be supplied. Unsupported log questions
remain unavailable, not empty authoritative histories. Durable QSO submission
continues through the existing outbox; local selection never submits a contact.

Before any native package, render this actual host in the existing browser probe
with source-shaped owner fixtures. Capture original Classic and Roster at normal
operator size and supported floor/high scale for review. The fixture must deny all
unmapped upstream IPC so hidden dependencies fail visibly. This is followed by
private native storage/lifecycle and owner-backed RX/log/control acceptance; a
browser fixture alone is not owner or hardware qualification.

## Validation checkpoint, 2026-11-15

- The first attempt stopped before test collection because sandboxed esbuild
  could not spawn (`EPERM`). The receipt is retained. The authorized identical
  1 GiB attempt with subprocess permission passed all 19 new cases; peak
  552,202,240 bytes, root exit 0, all descendants retired naturally.
- A 161-file upstream selection reached its 600-second bound without a final
  JSON report. It is not a passing test group.
- A smaller 37-file selection reached its 300-second bound after 23 complete
  suites and 290 passing tests, with no reported assertion failure. The other
  14 files were not qualified by that attempt. Its peak was 591,425,536 bytes;
  the guard stopped the owned job at the deadline and confirmed no descendants
  remained. This is partial evidence, not a completed gate.
- All 2,408 recorded UI source files remained byte-identical during those
  attempts. Evidence, raw logs, explicit inventories and cleanup receipts are
  retained under this worktree's `target/cockpit-validation/`.

Source-switch regressions cover queued wheel intent, late wheel replies, rotor
settings/declination, app version, same-call award facts, bearing/entity results
and independent emergency rotor Stop. Held facts are keyed to their source as
well as the question; a pending new owner never displays the old owner's facts.

The repository-derived UI CI gate includes the full Vitest suite and
`tsc -b && vite build`; its browser matrix is broader than the selected original
CallCard geometry check. These selected checks do not claim full CI coverage.
No owner-bound Classic/Roster browser host is mounted yet.

The remaining 14 closest upstream suites subsequently passed all 75 tests in a
completed bounded run (103 seconds, natural root/descendant exit). Thus the 37-file
selection has 365 observed passing tests across the retained partial run and the
completed follow-up; this does not relabel either timed-out attempt as successful.

Integrated `tsc -b` and `vite build` then passed in 320 seconds under the 4 GiB
aggregate bound (peak 2,593,251,328 bytes). Five separately owned files were copied
with exact reviewed hashes for this integrated check: the scoped `LogSource`
module/test, additive station-identity wire types, and the original recent-QSO
pane's external-read module/test. Their baseline copies and hashes are retained;
they are excluded from this service-boundary commit. No canonical source or
TypeScript build-info was changed. The selected original browser qualification subsequently passed as described below.

The next host slice must also separate mutation outcomes: native API methods may
return `AppSnapshot`, while an xLog command returns an owner receipt and the owner
read model refreshes independently. A receipt must never be cast into a snapshot
merely to satisfy the current upstream service signature.

The original CallCard browser matrix passed all 21 cases: eight protected
placements, eight intentional-overlap controls and five Phone-card presentations.
A first attempt failed three initial cases before their trigger existed within
the original ten-second setup wait; all later measurements and normal browser
exit were retained. The reviewed Windows evidence copy then waited up to 120
seconds for a complete document and a visible/topmost trigger, rechecked the hit
target before input, and closed every target in `finally`. No geometry assertion
or product source changed. The overall Job remained bounded at 2 GiB/600 seconds.
The corrected run took 60 seconds, peaked at 1,222,213,632 bytes and retired the
browser and all Job members normally with exit 0. Screenshots 001 and 018 were
visually inspected for Stop separation and 175% text presentation.

Key retained evidence receipts (relative to `target/cockpit-validation/`):

| Scope | Receipt and SHA-256 |
| --- | --- |
| Integrated TypeScript/Vite | `build-prep/receipt.json` — `18811A095CF3A30D7EE73A58C4643A5315C3E8BB3496DFDC2F6A930BB8252B9C` |
| Remaining 14 upstream suites | `upstream-remaining-b2f59b085fbe4428a6985005d817894d/receipt.json` — `FCFF8E5154503FA10D09B704E9E5ED1306781BE2B89AD01CF42264C6BC06960B` |
| Corrected original browser matrix | `browser-ready-0215de699232472a983765277ad77fe9/receipt.json` — `0339E147831B2FD540CC4247E7488AA76FBC2D1E0E5F8D93EF7078CE6E5C3FD1` |

The browser fixture is the upstream default-service surface, not an xLog owner
or a mounted original operating host. In particular its refused `ask_log` and
upstream empty-history fallback do not establish owner history or award facts.
The subsequent owner host must distinguish unavailable history from an empty log.
All failed and interrupted attempts remain preserved; full upstream CI and the
full 161-file selection remain unqualified.
