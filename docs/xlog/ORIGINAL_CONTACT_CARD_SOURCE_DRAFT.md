# Original ContactCard owner seam — source draft

Status: read-only component slice with component, producer-replay and browser checks below; not mounted in the operating host. Nexus source baseline
`ceb0ef61bb1360d8979ddb8bb329a06ad5e9c4f4`; owner source inspected read-only at
`5c979984613a91bee9c494c4e8eb84b4eb114f1b`. No application, database or production
profile was opened. The source checks performed after the review are below.

## Validation checkpoint

- First bounded five-suite run: owner/native-comment-policy 14, original native
  ContactCard 28, edit-handoff 5 and pure field-preservation 23 passed. By-ID 13
  failed because its existing API mock omitted the QRZ-page dependency captured
  during row rendering. Receipt `6B3872BDF1000F5FFD815AC27F62A21928BBBDDA2208CF82CF70470EB678EBD4`
  remains in `target/contact-card-check-preparation`.
- The narrow fixture fix exposes a throwing QRZ spy and asserts it is never
  called. Follow-up by-ID 13 passed; hardcoded strings 377, i18n invariant 8 and
  runtime 20 passed. Placeholder suite passed 20 and failed 10: four inherited
  xLog keys missing from translated catalogs, plus two spurious translation-JSX
  sites in the canonical owner-history helper's generic arrow function. Receipt
  `46026A16C858F814B879B569CB99D28DB9095FD6D5F50C754D6C45C140936E73`
  is preserved in `target/contact-card-catalog-preparation`. The computed-key
  threshold was not changed. Root fixed that helper with an equivalent function
  declaration and supplied the four keys in shared commit
  `6b40a539b941b5e3bf0e630b0a21481e83807bef`. The owner-card generic functions also
  use declarations; their earlier attribution as the baseline failure's cause
  was provisional and superseded by the exact extractor diagnosis. The shared
  fix was borrowed unstaged for the final combined catalog check and is excluded
  from this slice's commit.
- Full actual strict `tsc -b` passed with an empty diagnostic log. Receipt
  `CB1333543912FC06B982E397FA27571DC2D3420049F663D09CC7F3961DD2DCB0`
  in `target/contact-card-types-preparation` binds the exact checked source.
  It includes both new runtime files and the new tests, starts without prior
  app/node build-info, and preserves the resulting cache. Peak job memory was
  2,629,046,272 bytes under the reviewed 4 GiB cap. The later equivalent function
  spelling, shared catalog repair and new producer test are covered by the
  subsequent final integrated type checkpoint below, not that original receipt.
- Final actual strict `tsc -b` passed with zero diagnostics on the current
  source, including the producer replay test and shared catalog repair. Receipt
  `A30830D906B159F8812FDCED02F6B5C177891A3721EC8811468CDEAFB6DB21D3`
  in `target/contact-card-final-types` binds 2,602 unchanged inputs. Existing
  build-info bytes were preserved before reuse. The 4 GiB job peaked at
  2,619,092,992 bytes and retired naturally with exit zero and no processes left.
- Final combined catalog gate passed 435/435: hardcoded strings 377,
  placeholders 30, invariant 8 and runtime 20. Receipt
  `AA90D49F18B05C9AB9E060D3DFAB5F8D91E6639AEADC67FA48231A2DD103F100`
  is in `target/contact-card-catalog-shared`; all 2,600 source pins stayed fixed.
- Actual Swift/SQLite projector replay passed 3/3 (found, deleted, not-found),
  receipt `61B1CFA0947FF8683325EE2ACA96B33C3630A933CED4DDD64CE32C5C2794176E`
  in `target/contact-card-producer-check`. It proves the original card retains
  MFSK/SUBMODE FT8, multiline comment, authoritative paper sent-via D and null
  frequency, while remaining read-only even when the producer supplies a full
  edit base. The test-only own-property check was subsequently rewritten to
  its equivalent ES2020 form; exact original passing test bytes are preserved.
- Actual original ContactCard browser checks passed seven cases: native and
  owner bodies at LAPTOP, FLOOR (1366×768 at 175%) and UHD200, plus the native
  static-footer negative control. Receipt
  `0CE07A51851077A41C930F8A9D273FC940C960A96766ED39B9E6CA496C168E9D`
  and result `02DBBEF8B983726D34DA1EC00046CD83E00C48B457C8CC0A5E0614D6D63AFC4D`
  are in `target/contact-card-browser-scroll-fixture`. The same Dialog, fields,
  classes and sticky actions fit without horizontal overflow. Native photo,
  ten recent rows, row double-click and Cancel remain usable. Owner Enter,
  insertion/paste and expanded facts preserve read-only data; unavailable data
  clears without remounting the same selected card. Owner native calls,
  submissions, external requests, page errors and console errors were zero.
  Known Dialog-description warnings are recorded separately.
- Three failed browser attempts remain preserved: the old native fixture lacked
  five optional startup read answers, then its idle completion answer; the
  initial pointer helper's nearest scrolling left Comment beneath the sticky
  footer. Only target-local synthetic answers and ordinary centre scrolling
  changed. Product/CSS, hit-tests, geometry, negative control and limits did not.
  The sole allowed native refusal is the original probe's documented QRZ-page
  fallback from the native row gesture; all others remain failures.
- Every attempt retired naturally with zero remaining job processes and no
  resource or forced-cleanup event; failed receipts/source inventories remain.
  The final browser and guard both exited zero; peak aggregate memory was
  1,331,216,384 bytes under 2 GiB. No native application, owner HTTP, production
  RPC, host mount, package or installed acceptance was performed.
- The nine new messages exist in all eight shipped translation catalogs and
  were appended narrowly to both authoritative Chinese CSVs, preserving all
  original bytes and existing rows. No full catalog regeneration was used.

The owner supplied `CompanionContactDetail.swift` SHA
`A70BE126EDE81C601A711BB1C6DF0E11B11205AB2F4D61D66AE34E811FA343D9`
(reviewed update; the field shape is unchanged from initial `1C43F587`).
The local presentation mirror follows that actual source's fixed categories,
`value`/`empty`/`unavailable` fields, deleted tombstone and exact `editability.base`.
The first serializer corpus remains shape-only evidence. The final corrected
corpus from xLog source `635f754d` is committed unchanged as
`ui/src/xlog/fixtures/contact-detail-producer.json`, SHA
`B53A2BE67F986C41A908D3F2BA802AFA055D747AEF6C62EC67EF1A7E6443043F`.
The outer ID is canonical lowercase; the retained edit-base UUID has uppercase
hexadecimal letters and an explicit null frequency. This is projector proof,
not a tested authenticated transport or live read lease.

Prepared files: the existing `ContactCard.tsx` now has one shared original JSX
surface, its unchanged native service body, and an explicit owner input branch.
`OwnerContactCardBody.tsx` supplies read-only fields/slots without native
subscriptions or service calls. `features/ownerContactDetail.ts` is a local
typed presentation mirror, not a `LoggedQso` adapter or IPC validator. Fourteen
component cases cover unknown/empty, raw references/comment, retirement
without remount, deleted/not-found/unavailable, mismatched contact IDs and no
native calls, inert multiline Enter/paste/change handling, and nonfinite or
out-of-JavaScript-Date-range timestamps, plus a native-default comment-policy
positive control. The shared comment policy defaults to
the unchanged native normalization/Enter-submit behavior; owner presentation
explicitly selects read-only multiline with neither handler. Their first run
and the existing native positive controls are recorded above. No host mount,
RPC, Save, outbox wiring or write authority is added.

## Scope and prerequisite

Reuse `ui/src/components/ContactCard.tsx`, its Dialog, sections, field order,
classes, and CSS. Open a contact by the immutable ID already supplied by owner
Recall/Recent. Preserve the default native component and writer behavior.
Do not build another editor, adapt missing facts into a fake `LoggedQso`, or
mount the whole native Logbook just to display one contact.

The prerequisite is an agreed bounded companion by-ID detail contract. The
owner already implements `logbook_contact` in
`Sources/XLogPrototypeSupport/LiveLogbookMethods.swift`, using
`LogbookContactSnapshot { revision, qso }` and its workspace generation. This is
an existing producer to project, not permission to expose arbitrary worker RPC.
Endpoint names, wire fields, bounds, and policy values remain owner decisions.

The current companion history row is insufficient: its `details_complete`
describes the compact comment projection. It does not certify that all QSO,
confirmation, correction, or award fields are present. Full scalar history
counts also do not supply those per-contact facts.

## Actual dependency boundary

| Current dependency | Required explicit owner behavior |
| --- | --- |
| `CardContact.row: LoggedQso`, `RowRef`, native `RowAnswer` | Separate typed owner detail and immutable owner revision; never manufacture a native edit key or applied answer. |
| `draftOf`, `fieldsOf`, `cardSave` in `features/contactCard.ts` | Native helpers remain native. Owner presentation preserves unavailable versus known-empty facts; owner mutation contains only admitted fields. |
| `callbookAnswers` global subscription, `qrzLookup`, `CallbookPhoto`, QRZ link | No global/native lookup or cached foreign-owner answer in explicit owner mode. Retain controls with honest unavailable capabilities until a scoped service is supplied. |
| `useLogAnswer(callHistory)` and `emptyAnswer` fallback | Explicit owner recent/history read state, actual IDs, nullable frequency and channel evidence. Unavailable is not an empty log. |
| IOTA note/picker and clearing held fields | No native directory request or unsupported mutation. Display supplied reference, with picking/clearing unavailable. |
| `ContactPath` | Do not mount its native data path without an owner-safe read service; no independent path correction. |
| `CardAwardStatus` | No native award mutation or locally inferred eligibility; display only explicit owner facts and their policy/provenance when supplied. |
| `QsoHistory` and Undo/Redo | No native history query or undo. Owner correction history is a separate capability, not the retained outbox receipt. |
| Existing Save and discard/switch controls | Preserve original gestures and draft guard. Explicit owner mode must gate the handler as well as the rendered button. |

Passing `ref: null` today is not a service boundary: it only disables editing.
The card still reads callbook/history and mounts native-dependent descendants.

## Presentation and lifecycle proposal

Add an explicit owner-mode input to the existing component rather than a
permissive global service shim. Native props keep their current semantics.
Both modes use the actual card markup; mode-specific preparation supplies
field presentation and permitted operations. Avoid a duplicate owner JSX card.

Owner state needs loading, current, unavailable, and not-found/deleted outcomes
represented by the producer's fixed DTO. Future admitted current facts must be
bound to contact ID, owner/workspace, native connection token, generation,
revision, and the original read lease. On source/contact change, stale callbacks
cannot restore prior facts or edit authority. Expiry must remove authority
without discarding operator typing. A new read/revision must not remount over
an unsaved draft; use the existing guarded close/switch and explicit reload
flow. The current native JSON-row remount remains unchanged for native callers.

For unavailable fields, keep the original field location and label, but show
an unavailable presentation rather than a fabricated empty/false value. A
known empty value may remain blank. Nullable frequency is not zero MHz.
Confirmation channel status/date is not equivalent to Nexus's aggregate
`confirmed` or `awardConfirmed` booleans. Preserve raw owner statuses and their
meaning; do not reinterpret a policy-specific summary as all-channel evidence.

Start with read-only detail. Once the owner DTO is agreed, the existing core
inputs may edit time, frequency, reports, grid and comment through the durable
outbox. Callsign, band and mode stay read-only under the current owner policy
(`LogbookDatabase.applyCompanionContact`). Other original fields remain
read-only, including end time, name, QTH, notes, power, station fields, arbitrary
ADIF tags, confirmation marks and award/correction actions. No edited value may
be silently omitted from the submitted mutation.

Outbox submission must retain exact request ID, contact ID, workspace and
expected contact revision. Queued, needs-receipt, rejected and applied are
distinct outcomes. An enqueue acknowledgment is not a native `RowAnswer.applied`
and must not produce a "contact saved" claim. Preserve the typed draft until
its durable disposition is explicit; never automatically retry an uncertain
mutation. Reuse the existing durable draft/outbox logic, not QsoPanel's custom UI.

## Remaining integration boundaries

1. The fixed producer now supplies nullable typed fields and explicit core
   editability. The client/native by-ID transport must validate identity,
   bounds, revision and real connection/lease retirement before host mounting.
2. Reuse the original Recent/Recall `onOpenLogbook(call,id)` gesture for detail
   selection. Do not fabricate a page location or a native `LoggedQso`.
3. Supported editing remains a separate logical slice. Use the complete exact
   edit base and existing durable outbox; do not normalize unchanged values,
   replace a dirty draft on refresh, silently renew authority, or call queued
   data saved. Only a matching applied owner receipt confirms a write.
4. Raw retained confirmation fields and unavailable upload receipt projections
   are not independent provider or award authority. Do not derive new flags.

Full Logbook pages are deliberately separate: its query/total/order, by-ID
locate, filters and row-update contracts cannot be fulfilled by the bounded
25-contact history or five recent contacts.

## Meaningful planned checks (not run)

- Native positive controls: existing `Logbook.contactCard.test.tsx`,
  `Logbook.byId.test.tsx`, `Logbook.editHandoff.test.tsx`, and
  `features/contactCard.test.ts`; preserve native save/discard/keyboard behavior.
- Explicit owner mode with native APIs/global subscriptions made throwing:
  opening, expanding sections and attempting denied actions dispatch no native
  callbook, history, IOTA, path, award, undo or write request.
- Actual owner serializer fixture: null frequency, known-empty fields,
  unavailable confirmation evidence, multiline comment, oversized fields,
  exact UUID/revision and deleted/not-found outcomes. No handwritten fixture
  described as producer proof.
- Delayed A read after B selection, same ID after re-pair, expiry during typing,
  and a newer owner revision: no stale facts/authority and no silent draft loss.
- Core edit uses the exact unchanged owner values for untouched fields,
  including frequency precision, time precision and multiline comment. Denied
  fields cannot enter the mutation through submit or keyboard paths.
- Queue failure, uncertain delivery, restart/reconcile and duplicate submit
  retain immutable request identity and do not repeat a write. Existing actual
  disposable Swift/SQLite tests qualify owner effects separately from synthetic
  HTTP fixtures and component tests.
- Actual original-card browser comparison at supported viewport/scale, with
  native positive control and nullable owner states. No native application or
  installed acceptance claim follows from that browser test.

Do not attach the existing FT8 Log action to a selected decode: upstream
`App.handleLogCurrent` asks the QSO controller to log a completed exchange.
`LogEntry.remote` is a Phone/CW/satellite strip seam, not the FT8 cockpit's form.
