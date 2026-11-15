# Rotor request and later owner state

Tested, unwired helper based on Nexus `7e36f9e7a6e792f8f845ed1e2ea4aef09635e38f`.
Owner source inspected at `453e83f34dc40b5d9345fca34d3eba083e879628`.
All 31 focused pure observer tests passed. No control request, device, application
or browser was launched for this work. The original cockpit host integration is separate.

`ui/src/xlog/controlObservation.ts` observes one explicit `rotor.point` request.
It has no transport, polling, timer, retry, Stop or native dependencies. Existing
`api.ts` types are imported as types only. No SDK/API/hook or owner file changes.

## Actual contract and limits

- Owner `companion_bridge.rs` retains a non-QSO command receipt without later
  telemetry reconciliation. `companion_host.rs` maps owner acknowledged to
  accepted, confirmed to observed, and unresolved dispatch to uncertain.
- SDK `commands.rs` validates receipt request/server/workspace identity, clears
  its pre-command snapshot after send, and keeps rotor Stop on an independent
  client. The observer does not alter these paths or the returned receipt.
- Owner `companion_host::snapshot` advances the projection revision after one
  second even without new device telemetry. `companion_projection::project`
  checks retained owner freshness but exports no individual field sample times.
  A later matching snapshot cannot prove that the command caused movement, or
  even that the physical field was sampled after admission.
- Rotor identity is the opaque owner connection route. Generic radio IDs do not
  export equivalent selection/worker evidence; station tune is outside this slice.
- Owner normalizes valid azimuth 360 to 0. The SDK only accepts commands and
  observations in [0,360). This helper uses that wire range with exact equality;
  it does not normalize malformed wire values or invent angular tolerance.

## Caller seam

At explicit rotor Point intent, create one observer from the immutable UUID,
native connection token, local UI session generation, server/workspace, exact
rotor route/target and current validated baseline display lease. Do not generate
a token as a substitute for native connection identity. A missing token means
this observer is unavailable.

On the exact command's return, call `receipt(scope, receipt)` once; on transport
failure call `uncertain(scope)` once. Receipt status remains unchanged, including
rejected and uncertain. Even a matching later snapshot must not replace it with
success. Late or mismatched receipts cannot settle another request.

Feed only successful **poll** observations with actual request-start monotonic
time and the existing validated display deadline. A lease by itself, cached
status read, response arrival time, or observer-created deadline is insufficient.
The poll must start strictly after command completion and exceed both baseline
revision and generated timestamp. Target/connection/owner changes retire the
observer permanently. Newer out-of-order reads cannot replace newer evidence.

Render `inspect(currentScope)` on every render and an expiry timer. A returned
object is a view, not authority that can be retained without reinspection.
Repeated evidence cannot extend a deadline or resurrect expired state. Clear
the observer synchronously on Stop intent, supersession, disconnect or re-pair;
then invoke the existing independent Stop path without waiting for this helper.

Suggested copy keeps two separate facts:

- **Request:** accepted / uncertain / rejected / owner-reported observed, exactly
  as recorded by the dispatch result.
- **Later xLog snapshot:** requested azimuth shown / different azimuth shown /
  unavailable. Motion true, false or unknown remains separate. Matching azimuth
  does not assert stopped motion, causal completion or a new physical sample.

## Focused validation boundary

Focused tests cover immutable ID/receipt/context; all five receipt statuses;
pending/pre-completion/status reads; token/epoch/owner/route replacement;
null/unknown motion and azimuth; strict target equality; both baseline fences;
out-of-order and inconsistent duplicate evidence; expiry/no lease renewal;
Stop invalidation; transport uncertainty and monotonic-clock regression.

The focused run used the reviewed private Windows Job guard: 2 GiB total
descendant limit, BelowNormal, one Vitest worker, Node 20.19.5 with 1024 MiB heap,
300 seconds, 8/6 GiB physical/commit admission, 2/2 GiB runtime floors, three-second
natural drain and independently recorded cleanup. Source/tool/lock hashes before
and after. Full TypeScript/build and actual original-host hookup remain separate
gates. The host TypeScript receipt `5679771FE25F3965136C987516ECD32212C74E1867FBEC623AC1B3CCB4B05A55`
does not inventory this unwired helper and is not claimed as its qualification.
The full host browser exercise likewise does not exercise this rotor helper.

Evidence is preserved under this worktree's
`target/rotor-observation-check/attempt-836cef319e9d49389a9cb58ff3519bd4`:

- `receipt.json`: SHA256 `1385D6725BB98AF379EFD38FC5394F4D2A1C344480E420BD4FA45B7D6A44C84D`.
- `tests.log`: SHA256 `92BACF2A2E3516E547DA8B514779559F2FA2DB96C3710CCA16F46ED19E0E1229`.
- Helper source: SHA256 `375B110C81EC966CBDECD3E275F613ABF881CE5D22D9ED3FAEADF59214811947`.
- Test source: SHA256 `8E815647929F8AD43078EA2571ADEA1D7AD7D152A6A839C49E6AC08A5E5BCFFE`.

Vitest 4.1.12 completed 31/31 tests in 2.22 seconds. Guarded peak was 259,805,184
bytes; root exited normally with zero remaining descendants, no limit/abnormal
notification, and confirmed cleanup. All pinned source/tool inputs were unchanged.
This is synthetic state-machine evidence, not an actual rotor or host acceptance.

## Narrow TypeScript gate

The actual UI `tsconfig.app.json` strict/no-unused policy also passed with only
this helper and its test as explicit roots, retaining their real imported types.
This does not qualify the whole UI or host wiring. The resolved configuration and
unchanged source inventories are retained in
`target/rotor-typecheck/attempts/attempt-2cf597248591467daa9deb884d30c8a8`.
Receipt SHA256: `84A8D6653BDD22102461A4D30CDC8637B8DF3283BB3C1E6C6AAE6994F9765455`.

The separate bounded attempt used Node 20.19.5, 512 MiB heap, a 1 GiB descendant
Job cap and the same admission/floors/deadline/natural-drain policy. TypeScript
exited 0, peak Job memory was 176,525,312 bytes, root and descendants exited
naturally, and final active count was zero. No browser or control IO occurred.