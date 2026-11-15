# Original manual Add QSO — source qualification

Base: `3487e4e2e7148398c9085e19ed17f5e04e314653`. Isolated branch:
`codex/original-manual-create`. Focused, catalog, TypeScript/Vite and actual-browser results are recorded below. The presentation-only follow-up is separately qualified below.
No version, package, native launch, production log write or hardware action.

## Scope

The original `Logbook.tsx` form body and draft shape are extracted into
`LogbookForm.tsx`. Native callers retain the same callbacks, IOTA child slots,
field order, classes, UTC controls, one-line comment normalization and Enter
submission. The owner dialog renders that actual surface without mounting the
native Logbook's data queries, provider lookups, import/export or upload actions.
Unsupported original fields remain disabled with an explicit explanation.

`OriginalHost` adds an explicit Add QSO action. Its existing operating Log button
remains disabled: a received decode/selection is not evidence of a completed QSO.
The create draft coexists with the separate retained correction record. The connection
button opens the existing connection dialog; it does not reconnect or send.

## Exact owner input

`manualCreateFields.ts` implements the actual CompanionQSO limits: uppercase
ASCII call 3–32, ASCII band 1–16, uppercase mode 1–16, integer UTC milliseconds
0–4102444800000, nullable integer Hz 1–1000000000000, reports up to 16 UTF-8 bytes,
empty or Maidenhead 4/6/8 grid, comment up to 4096 UTF-8 bytes with CR/LF/tab only
among controls. Isolated UTF-16 surrogates fail closed. Neither stored nor frozen
input is trimmed, uppercased or retargeted on retry.

The original frequency label is MHz. Owner mode admits six decimal places,
converted with checked integer arithmetic; fractional Hz/exponents/zero are
refused and blank remains null. Native step remains 0.0001; owner step is
0.000001. Owner UTC text retains milliseconds, with calendar/time roundtrip
checks; native maximum length and default handling remain unchanged. New form
UTC defaults are local user input, not certified receive timing. Band/mode/call
start blank rather than borrowing radio DIGU as contact mode.

## Draft and operation ownership

Only the narrowly namespaced `om0rx-xnexus.manual-create.v1` key is used. Existing
correction v1/v2 bytes remain untouched. Record schema:

- version 1, kind `owner_manual_create`, immutable request/contact UUIDs;
- original workspace ID and bounded raw form fields;
- nullable frozen complete QsoDraft, exactly equal to the fields projection.

Open/view does not write or dispatch. First explicit edit binds a current owner
workspace and persists raw typing. Unknown/corrupt/changed records are preserved.
Storage uses synchronous compare-before-set, not a claimed cross-process atomic
CAS. The private native profile's existing single-writer lock remains the outer
owner. No credentials/connection tokens are stored.

A synchronous busy gate rejects duplicate intent before React rerenders. Exact
payload persistence must succeed before enqueue; failed persistence keeps the
same in-memory immutable request for explicit local save recovery. Enqueue is
local only, and Submit is separate. Restart does not inspect/send automatically.
Exact queued payload/workspace/request/contact must match before delivery or
receipt recovery. Uncertain operations retain the same ID and require explicit
receipt lookup; no automatic HTTP retry/new request identity exists.

Only a verified matching Applied receipt enables explicit Done acknowledgement
that removes this local form. It never removes the native outbox record or xLog
contact. Durable/restored/possibly attempted frozen operations cannot be discarded on
uncertainty, rejection or capability loss. A freeze whose local persistence
failed before this controller ever called enqueue can be explicitly discarded
through confirmation, after verifying exact older unfrozen/empty storage. A
changed/unknown/stored-frozen record is preserved. This distinction does not
claim an operation absent merely because an outbox inspection was empty.
Unqueued discard uses the existing confirmation dialog and compares exact prior
bytes. Never-persisted typing can be explicitly discarded only after verifying
the key remains empty; changed/unknown storage is preserved. Closing retains a
durable draft; busy or unsaved-storage-failure forms cannot close silently.
Queue inspection retires prior Pending presence before reading; empty/error
responses cannot leave Submit enabled. Previously verified receipts and frozen
identity remain retained without claiming a current queue entry.

## Qualification checkpoint

The first focused attempt retained all 96 results: 59 new tests passed and 37
native regressions failed before the form event because six old fixtures omitted
an existing `openQrzPage` export. Receipt `564E4DF2EA672DD283E03CF298782EBD64AE1F07E63838A44F35F44D41563603`
and all 2,806 source pins remain preserved. Static baseline comparison proves
the original fixtures, `qrzPageOf`, and row rendering unchanged from the base;
that is source comparison, not a claim that baseline tests were executed.

The five new suites passed: field validation 20, store 6, controller 24, actual
form 6, and hook 3. Only the six native fixtures then gained explicit throwing
`openQrzPage` spies and assertions proving zero calls. The separate follow-up
passed 37/37: comment 3, UTC date/time 4, end-time 8, fields 5, IOTA 3 and
keyboard 14. Receipt `33F6A66CB75AB6E7654BE8DB7C7006CA71CA80170397621C046331816C56DF93`
binds unchanged 2,807 pins, natural root 0/final active 0 and 738,721,792-byte
peak. This is a composite 59 + 37 qualification, not a retroactive green result
for the first attempt. All original native assertions remain intact.

Four actual translation CSVs (Italian, Russian, Simplified and Traditional
Chinese) contain exactly the 13 new labels with exact English and translated
values. Italian/Russian additions preserve every pre-existing byte. Scoped
proof `F2CEC6957A2D62E2820C1CD303C50973CCF364E8CAD9BE615D01AC7C1B2BCA81`
is not a full converter regeneration/check claim.

The four catalog suites passed 439/439: hardcoded strings 381, placeholders 30,
i18n invariants 8 and i18n behavior 20. Receipt
`F73BB287CF0FFE5C71D52D6E987A09822F9B79BCA9B9C7BE3A4A993523744D80`
binds all 2,807 unchanged pins, natural root 0/final active 0 and
887,328,768-byte peak. No ignored or empty suite was accepted.

Full strict `tsc -b` followed by the production Vite build passed under the
4 GiB/600 s guard. Receipt
`5DBFD80EE1F99F8ED4A86C8BD4DA933B7B0ACDF225A110D969253D5E2584B57D`
binds all 2,810 unchanged source/tool/evidence pins. TypeScript produced no
diagnostics. The real production entry produced 41 bundle files; manifest
`8926ABA3C2437A9C38142D55796608EF603B9588B2F0FCA7AC911C25D8AC8E7E`
records them. Existing large-chunk warnings remain. Root and all descendants
retired naturally with code 0, peak 2,616,766,464 bytes. No existing tsbuildinfo
was present or deleted before this actual complete project check.

The actual original-host browser passed three viewport cases in six Classic /
Roster layouts: laptop 1366×768 at 100%, floor 1366×768 at 175% (780×438 CSS),
and UHD at 200%. Receipt
`EC9267FD73446FEE9009509A38CB27DCDBBC2CCA7D05A197DDB4646DEE714CA7`
binds all 2,814 unchanged pins; results
`6C1BFCDE06A7BDD43D704334082D81C528B7DAF2BBDCEA50B71F2E8F65252608`
record exact payload/ID and control assertions. Root 20936 and private headless
Edge 25868 retired naturally with code 0, final active 0, peak 1,501,876,224 bytes.
No OS desktop input, native binding, native invocation, external request, page
error or console error occurred. Syntax validation preceded the browser.

The probe uses real OriginalHost, Recent/ContactCard and LogbookForm components:

- A real Recent/Card edit generates a separate retained v2 correction. A
  simulated host restart preserves it; all create actions preserve its bytes.
- Explicit Add writes nothing until editing. Original labels/units, millisecond
  UTC, exact MHz-to-integer-Hz, invalid-calendar refusal, multiline Enter and
  disabled unsupported fields are checked through hit-tested controls.
- Closing/reopening preserves the same draft. Write-capability revocation
  disables admission; return of a fresh grant restores only explicit actions.
- Storage failure exactly at freeze produces no enqueue. Explicit confirmed
  discard of proven local-only intent permits safe close with correction intact.
- Enqueue sees the exact already-persisted immutable payload once; pending close
  is blocked. Queue is visibly not Applied; only explicit Submit is dispatched.
- Uncertain outcome survives simulated host restart. Explicit Inspect validates
  the retained complete queued body, then reconciliation uses the same request
  ID. Only the matching Applied receipt enables explicit Done; native outbox and
  xLog contact are not removed by local draft retirement.

Representative floor images under `target/manual-create-browser/screenshots/`:
`1-manual-empty.png`, `1-manual-draft-classic.png`, `1-manual-uncertain.png` and
`1-manual-applied.png`. Visual inspection found the original form and wrapped
sticky actions readable. Ordinary scrolling exposes fields above that footer;
no forced click or CSS/assertion relaxation was used. Native form behavior is
separately covered by the 37 original regressions and the new shared-form
positive control, not claimed as a native application browser session.

This is source/frontend qualification only. Synthetic in-memory owner transport
and simulated React remount do not prove actual-owner SQLite writes, native
process restart, installed private-profile persistence or live-user acceptance.
No version or package was changed. The earlier failed fixture attempt remains
preserved and is not relabeled as passing.

## Qualified presentation-only follow-up

Root visual review found ready/dirty guidance repeated both beneath the title
and in the sticky footer. The follow-up changes only `OwnerManualCreate.tsx`:
footer status renders while busy or for any state other than ready/dirty.
Guidance remains once beneath the title; all operation/error status, actions,
CSS and draft/controller behavior remain unchanged.

Seven actual component cases passed, including ready → valid dirty single
guidance, pending/queued status and retained storage errors. Receipt
`03F6C1D8F9BE55467BDD3737B4CB6EC76956F8F6709BF086F0296B3ED56768A5`
records unchanged source, natural root 0/final active 0 and 514,428,928-byte peak.
This includes the existing six cases plus one new presentation regression, not
seven additional independent behavior cases.

The same complete three-viewport/six-layout browser flow then passed with two
additional guidance-count assertions. Receipt
`9E6DA7485BAFB46D814ABF0B87B89A7B6CFEB914165D241D92ABCB3974748406`
binds all 2,811 unchanged source/tool pins; results
`6335FB1F17F670040424B4D5BAF0934D8E90767E172CBFC5DB4E0888974002FE`
retain all prior queue, recovery, storage, no-native and geometry assertions.
Root 18172 and private headless Edge 8008 exited naturally with code 0, final
active 0, peak 1,469,591,552 bytes; no external requests/page/console errors.

Final floor images are under `target/manual-create-browser-presentation/screenshots/`:
`1-manual-empty.png` (SHA256 `22025F5F8F1AB361A80740EB09F1DB20EEFED6EDCD9B5F1724E3C94B127BD853`)
and `1-manual-draft-classic.png` (`17807E120E45FD143C158CB5E042ADFA20BC1633147AE64443C9FBC4310B7F59`).
Visual inspection confirms guidance once, more usable form height and intact
sticky actions. Earlier screenshots, failures and receipts remain preserved.

The `5DBF` TypeScript/Vite, `F73B` catalog and `EC9267` first browser receipts bind
their earlier exact source. They are not relabeled as full final-source checks
after the JSX/test follow-up. Combined final-source TypeScript/Vite is a separate
integration gate. No new locale keys, CSS or controller changes were introduced
by the presentation follow-up.

Full Logbook page/locate/filter contracts remain missing. This change does not
fabricate page totals/confirmation states, support arbitrary native rich-field
creation, mount a custom dashboard, or revive the parked terminal-archive UI.
