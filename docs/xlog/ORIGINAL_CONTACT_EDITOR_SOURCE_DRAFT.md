# Original ContactCard supported owner editor

Source base: `a13077213ee96aff663ea989e5906aa5dd71584d`. This slice extends the
actual original ContactCard; it does not mount the editor in the operating host,
add native RPC, change a product version, or qualify an installed application.

## Authority and presentation

The existing `ContactCardSurface`, Dialog, field order, CSS and sticky actions
are retained. Native callers keep their existing editing, one-line comment and
Save behavior. An explicit owner controller enables only start UTC, frequency,
RST sent/received, grid and multiline comment. Call, band, stored MODE, Notes,
provider data, confirmations and extended fields remain read-only. Enter inserts
a newline in an owner comment without submitting; native behavior is unchanged.

The caller must supply a validated contact detail, its original monotonic read
deadline, current authenticated connection/workspace binding and write grant.
Business `editability.allowed` alone never admits a mutation. Intent checks are
synchronous and include an in-flight latch. Expiry, source/revision changes and
invalid or backward clocks retire admission without overwriting the draft.
A latched clock fault requires a fresh controller; retained local queue
inspection and same-ID receipt reconciliation remain available.

The complete base, workspace, contact revision, fields and request UUID survive
read-key remounts and restarts. Outer/request UUID equality stays strict; the
validated base UUID comparison accepts equivalent alphabetic case. Nullable
frequency is not replaced by zero. Date and MHz conversion reject rollover,
fractional Hz and unsafe bounds; untouched values and multiline text remain
lossless. Restored frozen payloads must match the displayed fields exactly.

## Persistence and write lifecycle

The existing private draft key owns the additive version-2 editor record; no
second key, database, file or credential store is introduced. Connection tokens
remain in memory. A version-1 draft, another contact, corrupt/oversized data or
an unknown version is preserved and blocks replacement. Exact prior-byte
comparison protects writes. A stale legacy writer cannot replace an editor
record. The existing legacy Clear button uses a separate explicit CAS operation
bound to the bytes it observed; it can recover an invalid legacy draft but
cannot erase changed data, recognized editor records or unknown versions.

Queue correction first persists the exact immutable intent, then calls the
existing native outbox. Storage failure prevents dispatch. After enqueue is
attempted, its payload and ID remain frozen even if the reply is lost. Local
inspection matches the complete operation; an explicit retry uses the same ID.
Submit and Check receipt are separate operator actions. Queued, accepted,
observed and uncertain are not applied; only a matching applied receipt confirms
the correction. There is no automatic submit, resend, new-ID retry or rebase.

Pending intents block close. Dirty unqueued edits use the original discard/keep
confirmation. An unresolved frozen operation can be retained when closed.
Explicit Reload preserves conflict awareness and requires a suitable current
base; expiry, reconnect and late replies cannot silently replace typing.

## Validation

All evidence below is in the preserved isolated `original-contact-editor`
worktree under `target/`. It uses cached, pinned dependencies and unchanged
resource guards. Tests and browser fixtures use synthetic bindings/outbox
receipts; the contact payload is exact actual Swift serializer output, SHA
`B53A2BE67F986C41A908D3F2BA802AFA055D747AEF6C62EC67EF1A7E6443043F`.
This is not a physical station, native application or actual SQLite-write test.

| Gate | Result and evidence |
| --- | --- |
| Initial eight focused suites | 128 passed, 2 failed; preserved receipt `4A7E3995E29401E71B0F730DE16554051DF9C5372AABB808C879959420A7B4B2`. One expected error label was incorrect; the other exposed legacy Clear compatibility, corrected with the explicit CAS path. The passing native card, read-only owner and producer cases remain separately attributable to this run. |
| Corrected five focused suites | 85 passed: controller 30, editor card 8, storage 29, legacy recovery 8, QsoPanel 10. Receipt `83AC5B105CF44E67A94AE098E4E6E6965035EA9FF3DF71CF081149EF8E0D1835`. |
| Initial catalog gate | 434 passed, 1 failed: the new translated helper was missing from the migrated-file coverage list. Receipt `36F999F9DFC38F80E2544805BA4F169DABE92662C2819D3ABDB334391B733A9A`. |
| Final catalog gate | 436 passed after adding the helper to coverage, without exemption or budget changes. Receipt `1872DF8ECF3D9B87D9701B2646243C52170065FA9825735D5B33E8A4161357A7`. All shipped catalogs and narrow Chinese CSV additions are included. |
| Original-card browser | 8 passed: three native controls, three owner editing workflows, one static-footer negative control and one legacy recovery/stale-Clear case. Receipt `8BDDB1F2292522727843A3DFE4BF81E58C3058A6861D82A969F04CAF958C5677`; results `493C3E6193BF4CA20439C64B0F823B2F2993F1399B8CD266CB7FFA647868D4E9`. |
| Full strict TypeScript | Full `tsc -b` passed with no diagnostics, including all new modules/tests. Receipt `79A1FED6CF3940E9D82FCD2481D218A05DB733C2218F0AF0A493ED4ACFE8A524`; root exit 0, final active processes 0, peak 2,621,337,600 bytes, unchanged source pins. Fresh-worktree preflight found no existing tsbuildinfo. |

The browser exercised the original component at laptop, supported floor/high
scale and UHD/200% sizes. It checked ordinary hit targets, sticky actions,
multiline Enter, pending-close protection, lease expiry without remount or draft
loss, same-ID queue/uncertain/restart/reconcile/applied flow, explicit reload and
native default behavior. Owner cases made zero native invocations; only the
stock native fixture's documented QRZ-page refusal was permitted. No external,
page or console errors were accepted. Browser and Job processes exited normally
with final active process count zero; peak aggregate memory was 1,356,533,760
bytes, with no forced cleanup or resource notification. Source pins matched.

Two earlier browser failures are retained: `D159C8AC...D3ABCA2` caught a missing
fixture JSX brace; a parse-only stage then qualified both fixture files and the
probe (`31A2160AB6FD21BCE36676E09BB5EAD2F86A5DD12CE531BDC40369701D84580D`
is not a browser result). Receipt
`933344D26FE7F3D10753681E47A44BECBEB3714C40D58AE5711066AB734787CD`
then caught a CDP Enter event without keyboard text. The final fixture sends
Enter with `text` and `unmodifiedText` set to carriage return, preserving the
actual key handler and strict newline/non-submit assertions. No product change,
forced click, timing relaxation or assertion weakening resolved either failure.

Focused/catalog jobs used 2 GiB/300 seconds and one worker; browser used
2 GiB/600 seconds. Full TypeScript uses 4 GiB/600 seconds. Admission requires
8 GiB physical and 6 GiB commit headroom, with 2 GiB runtime floors. Evidence
retains source/tool hashes, failed attempts and natural retirement. This is
proportional qualification, not the complete upstream CI or a package claim.

## Remaining integration

The original Recent/Recall contact gesture must still supply the qualified
by-ID reader and synchronous current-connection authority to the controller.
That host mount and native paired lifecycle remain separate gates. Full paged
Logbook, new-QSO creation and actual owner SQLite create/edit/restart acceptance
are not completed by this slice. A received decode is never treated as a logged
or completed QSO.
