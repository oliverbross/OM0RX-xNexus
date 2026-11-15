# Independent passive bearing reads — SDK qualification

Base: `c00a6795e4d3f77c7d4bc5b773b7eeab0bc965ed`. The pure body codec's
qualification remains separate. This SDK slice passed the bounded checks below.
It adds no native command, UI, resolver, polling loop, capability advertisement,
rotor command, retry or transmission path.

`BridgeClient::bearing_handle(connection_token)` creates an independent,
non-Clone blocking reader using the paired HTTP client's existing no-proxy,
no-redirect policy. `BridgeClient::bearing_context()` captures only a currently
admitted SDK snapshot: exact saved-station binding, grid, paired session and the
snapshot's original monotonic deadline. The shared display identity parser is
unchanged; bearing's private six-key binding validates canonical lowercase nonnil
UUIDs, normal logging, positive worker generation and conservative safe counters.
An incomplete saved identity may provide a real grid without inventing a call.

Before scheduling, `client.fence().capture(token, exact_call, context)` retires the
previous request's facts and captures wall/monotonic start and a checked request
epoch. No lock is held across HTTP. The main SDK poll synchronously retires this
identity authority on binding/grid/worker changes or absent/invalid identity;
poll failure, disconnect, re-pair and drop revoke through the existing session.
Ordinary snapshot expiry prevents admission and expires all captured contexts.
An identical later poll can admit a new context, never extend an existing one.

`read(&request)` sends only authenticated
`GET /xlog/v1/bearing?callsign=EXACT_CALL`, encoding portable slashes as `%2F`.
The raw call uses the pure codec's ASCII grammar; no trimming, case repair or
base-call fallback occurs. The response is bounded to 20 KiB. Its nine mandatory
keys and six mandatory binding keys reject unknown, omitted, duplicate or null
values. Nested body nullability remains governed by the qualified 12-key codec.
Server/workspace/generation, full binding, worker and body origin grid must equal
the admitted context. Body requested call must equal the captured call.

HTTP 200 requires an available body. A typed HTTP 503 may return an unavailable
body and reason, only under the same identity/expiry contract. A bounded plain
`{"error":"stable_code"}` HTTP 503 remains `BearingReadError::Unavailable`;
it never manufactures a typed body, geometry, empty success or movement grant.

Both wall and monotonic read duration are below five seconds with at most 250 ms
disagreement. Owner generated time is bounded by request start and receipt.
The original snapshot deadline also caps network timeout and all result use.
Repeated owner timestamps retain their previous deadline, while timestamp or
logbook revision rollback retires facts. Saved binding, worker or origin-grid
mismatch yields `ContextChanged` and retires only its captured passive identity
authority, conditional on the same live Arc still being current. A delayed A
error cannot clear replacement B or A→B→A. It leaves the main bridge, receive and
other read handles usable; a fresh main poll may admit new bearing context.
True paired server/workspace/protocol/auth failures retain the existing global
session-revocation policy. Request replacement, including
A→B→A, revokes old results; an old queued request cannot clear a newer result.

`BearingLease::bearing()` and its guarded serializer check current authority.
Serialization checks before, during and after the final writer callback and
exposes only connection token, payload and remaining duration. Clock/expiry
failure latches that operation stale; restoring a clock cannot revive it. A
future native/UI layer must still apply a request-start display deadline and
its own selected-target/connection fence. This slice does not supply that layer.

Twenty-four client tests use a bounded synthetic loopback server and the
four byte-pinned actual Swift body fixtures. Synthetic envelopes and current
timestamps are explicitly test-generated: this is not an actual owner endpoint
or station qualification. Tests cover strict wire shape, exact calls and grids,
typed/plain unavailable, bounded errors and redirects, original snapshot expiry,
same-identity poll nonrenewal, identity retirement, ABA, delayed HTTP, clock
failure, revision/stamp highwater, serializer retirement and reconnect. Completed
validation covered all 173 SDK library test cases: 170 passed, with three
existing external-producer replay cases intentionally ignored because their
separate private inputs are not supplied. The included bearing corpus replay is
ordinary and passed. Offline/locked all-targets Clippy also passed.

## Exact validation evidence

Production client: `F685051B9A85D86730D268B3AED571F3991D034C97A765F6D661CBF6028EF4BB`.
Final client tests: `C1A5D9D35C25EB2147034BD5D2C85A71C99E43586358E166F91A482C05BCAA17`.
Shared SDK hooks: `B7A7E86086B6AFDAA9A5F5085E5F7F784F993E11DD144B14376EF35F1E3A72F1`.
Bearing module export: `7C6B33B9A60B7184B8C1FBFCDB92AD04B0ED315852C4D3F902D8A4D5053210C6`.
Actual Swift corpus remains byte-identical:
`307BD860A42BC5A3191FA0F421BFCA32AB0C27CC44DF0843840AE0F57B43FE8D`.

The original failed attempt remains at this worktree's
`target/bearing-client-preparation/attempt-620bbcf6f0c94aa182466dc44384ac19`:
receipt `DE099067C4ADD7464B0E420AB5B0A6FB7C6271CBC19E014BE0D970360BA51B39`,
test log `29A1708A6FCEE7E6A5D193D9862CE7D837AD4423DFB7F1F4FA4793A2DE11EC42`.
It had 168 passes, two test failures and three ignored cases; Clippy was not
reached. The corpus test incorrectly requested `VK8RX/P` for the two actual
`VK8RX` bodies; the freshness fixture created its supposed newer snapshot before
the first snapshot. The correction used each actual exact requested call and
its exact GET encoding, plus a case-specific one-second-old initial snapshot and
a refreshed snapshot created in the response handler. No production, deadline
or admission rule changed. Original test/runtime bytes and evidence are retained.

The separately approved corrected attempt passed on 2026-11-15 at
`target/bearing-client-fixture-preparation/attempt-0b80cf01ba5d46e59f8f7897dfa6c878`:

- Guard receipt: `EDB2EF5E7E401C5F0F50CCDF393BD2F2F8769235B3C5941A51CB74AFFF80D5D2`.
- Checks: `49C2CB10DEED48D540BE933DA30310C315D9CF718C03209BE1F6C63ED4EA28E0`.
- Identical source before/after: `B951F2E29CBE5A2A550334868BAD3BFCEB973215F16C3CF08EBA1EF999524470`.
- Fresh SDK test executable: `B7ECFB2BFA71225742925AE3A4EED7CE5A3D284F7A9307D033B74961CFC03984`.
- Full test log: `18971B4CCCEA5800FF6001673E6E2CCD45FC038C2B6253EF210A4F5D228DCB30`.
- Clippy log: `32E6240DB8B1D7930C7178713350E5E17BC75036ABD18B1E410FB88B50642A94`.

Exact GNU Rust 1.93.1; offline/locked, one job, changed-file rustfmt and SDK
all-targets Clippy with warnings denied. The target was fresh, seeded only with
the read-only retained third-party inventory; all 1,702 donor files rechecked
unchanged. No dependency/lockfile changed. The D378 private Job had 2 GiB/300 s
limits, 8/6 GiB physical/commit admission, 2/2 GiB runtime floors and three-second
natural drain. Root 21156 exited 0, natural and final active processes were zero,
cleanup confirmed, no resource/abnormal-exit notification; peak 726,278,144 bytes.
Only this qualification prose changed after the pinned run. This proves the SDK
and synthetic HTTP boundary; actual owner endpoint, native IPC, UI and physical
rotor behavior remain separate gates.
