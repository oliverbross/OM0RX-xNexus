# Needs query lifetime and pending results

Qualified 16 November 2026 from base
`f8be70d5122cec63b99ef9cd2511043dbf5c072a`, in the isolated
`codex/needs-query-lifetime` worktree. This is a source correction, not an
installed-candidate or physical-station acceptance record.

## Behavior

An expired captured query deadline rejects that response without revoking a newer
same-authority observation. A genuinely different reply authority, observed
revision/context change, explicit write invalidation, or session revocation still
retires old evidence. No deadline or TTL is extended.

Only HTTP 503 with an application/json body of at most 1,024 bytes, exactly the
four fields below, the expected protocol/server/workspace, and a still-valid
captured authority/session becomes `Error::NeedsPending`:

```json
{"protocol_version":1,"server_instance":"<paired server>","workspace_id":"<paired workspace>","error":"needs_pending"}
```

That result contains no classification. It preserves unrelated batches and rows
only while their original leases remain valid. The existing bounded group retry,
fairness, one-second cadence, PCM, and independent Stop behavior remain intact.
Plain/legacy 503, busy, malformed, oversized, mismatched identity, stale authority
and other failures retain their failure behavior. There is no generic 503 whitelist.

The corresponding owner marker is implemented in xLog commit
`27c669c7816e82dec3863781668f581f6d3fdc75`: only the QueryCache pending paths produce
it, and the bridge checks active pairing before emitting the identity envelope.
The SDK tests below use synthetic authenticated loopback responses; they do not
claim to run that owner implementation or an installed application.

The installed `.6` all-Unknown screenshot also had a separate host capability
check mismatch (`needs.query` instead of `logbook.classify`). That UI correction is
outside this change; these lifetime defects must not be described as its proven
runtime cause. Detailed call/DXCC presentation is also a separate scope.

## Validation

Evidence is retained under
`C:\Projects\OM0RX-xLog\.build\xnexus-worktrees\needs-query-lifetime\target`.
All jobs used pinned GNU Rust 1.93.1, one job, BelowNormal, private descendant Job
memory cap 2 GiB, 300-second deadline, admission 8 GiB physical/6 GiB commit,
runtime floors 2/2 GiB and three-second natural drain. SDK Cargo was locked/offline.

| Stage | Retained evidence | Outcome |
|---|---|---|
| Failing first | `needs-query-red/attempt-0f69f8566e054f2c8b57500b637ef4d3` | Original runtime: exactly two intended assertion failures, two negative controls passed; Cargo exit 101. The wrapper's success records expected reproduction, not passing application tests. |
| First formatting check | `needs-query-green/attempt-f406ca48e93e4f1f8ad95d5b0d829a7e` | Failed before compilation; module ordering in tests and the existing lib module list. Preserved unchanged. |
| Second formatting check | `needs-query-formatted/attempt-52ff9b1e6eaf4b5dad76172ceb7ead7b` | Failed before compilation; remaining lib module order. Preserved unchanged. |
| Final SDK | `needs-query-final-sdk/attempt-7886c02a6602491e98f293870c657473` | 175 passed, three existing external-input replay tests ignored; scoped formatting and all-target Clippy passed. |
| Native Worker | `needs-query-native/attempt-c81a9c04576c41a6906bf345f243a247` | 37 actual Worker tests passed, including the new pending/revoked/error regression. Eight unchanged decoder tests were filtered, not claimed as executed. |

The five new SDK cases cover expired A/new observation B, known A/pending B with
no renewal, eight invalid pending-envelope variations, real revision/write
revocation, and content type/size/captured-expiry/session fences. The native case
executes the actual production lookup loop with genuine SDK fact batches; the
existing 36 cases include PCM/Stop independence, pending scheduling, route/row
invalidation and lifecycle checks.

Both passing jobs exited naturally with zero descendants, confirmed cleanup,
no memory/abnormal-exit notifications, and unchanged before/after source/tool
pins. SDK peak was 725,581,824 bytes (PID 28212); native peak was 822,026,240 bytes
(PID 25012). The read-only SDK dependency donor remained unchanged. Native SDK,
assembler and Worker artifacts compiled freshly; the retained 1,746 DSP dependency
inventory and its qualified receipts were hash checked. `crates`, `libtempo` and
`Cargo.lock` matched dependency source `43e305f1d65ec27f6f72255a14f2eb0775e9b349`.

| Evidence | SHA-256 |
|---|---|
| Red assertion log | `974F639D5F1D1DE0B552193B941C7F00E5945FA9895028DFA6BC4943B22F8244` |
| SDK final receipt | `A1F7B0F93FB93EFD3B1412CA9525A542F01AE0E935330C8317211F90E2F8EE76` |
| SDK test log | `35279757EB56D88AD4FE9A5FDD8257DE1FF538AAE6D6DC8C8F6B3E24D602D31C` |
| SDK Clippy log | `412CFB8C53E515B1297041E5D8DE903E1B5E1FFEF6FF2FA0877179D195827419` |
| SDK source before/after | `4F232C9BB0D3071A0219F89138A2FFC4D86BCD8B4478492620AE8DA19C045208` |
| Native final receipt | `9305347E9F6A409192B2BC2E7F03A984A6A0303115F1E4E1D2489D1387B9B638` |
| Native test log | `963062452A60D77950586B823F105FBE33698919F57E2F61DF4821CC02027DC4` |
| Native source before/after | `C169E21E5874DAA535DFB70F1E6D5DB6897DC6F48D456A7364D54EE520BAA31D` |
| Fresh native test executable | `16358A5F786F7D89ACCBC39594D05BDDA97E9BE4E68FC5B3FAFFAF19E8FEAB5F` |

These are SDK and actual-module tests, not a full Tauri macro/type/link gate,
cross-platform qualification, browser acceptance, installation, production API
exercise, or hardware qualification. No GUI, device, radio, database or real
pairing credentials were accessed by this validation.
