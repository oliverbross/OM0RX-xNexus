# Main-poll needs authority inspection

Base: `036066d9db99e626ef50adf3b8409caf2731906f`.

The native main poll accepted SDK needs metadata, then created a temporary
`NeedsClient` to inspect its remaining lifetime. Dropping that client invoked
`disconnect()`, invalidating the shared authority and display facts held by
the receiver's persistent lookup client. Its next lookup saw zero authority
and deferred without HTTP, even when independent owner queries succeeded.

`BridgeClient::needs_remaining_authority_ms()` now inspects the accepted lease
without creating an owning handle. The main poll uses this accessor. It does
not renew any deadline or change `NeedsClient` drop/disconnect semantics,
write fences, session revocation, lookup scheduling, or owner contracts.

## Focused red/green evidence

One maintained SDK regression uses the actual BridgeClient, persistent
NeedsClient and synthetic loopback HTTP. It accepts a first real query, polls
again, inspects readiness, checks retained authority and cached facts, repeats
the poll, performs a second query, then confirms actual disconnect revokes both
batches. Inspection does not extend the cached batch's original deadline.

The baseline used the exact former native temporary-handle expression and
failed at `main-poll readiness inspection revoked the retained worker handle`:
0 passed/1 failed, exit 101. Raw test log and baseline source bytes remain in
`target/probe-red-preparation/attempt-78b88cab59734a8dbd59ee6a34c8d80a` in the
`needs-nonowning-probe` worktree. The expected-failure wrapper exited naturally
with zero remaining processes; it does not label that assertion as passed.
Baseline log SHA256: `E9EA9BF6E838E5C550D2BB43B8F425BCA9469E224922D9F285CB6132900A7961`.

After replacing the probe with the non-owning accessor, the same named
regression passed (1 passed, 0 failed, 208 unrelated tests filtered).
Evidence: `target/probe-green-preparation/attempt-3139bf7d2e094f44b1735a3707223ca2`.
Receipt SHA256: `DD36904E8B795B054C78AD786E52013433C36B1BFF1B27A589C64AD13C22E8F5`.
Log SHA256: `BF3C8CE73D112E462A94B0926E12668FF7BB94C6EFA381D433B01BA070C132DA`.
Source manifests before/after match
`0BA4462C0D95A73B8ECA0779831FB921CABFB99F86CF398F57C43B0B125C367A`;
tool and read-only dependency pins also remained unchanged.

Both runs used direct GNU Rust 1.93.1 compilation of the actual SDK against
qualified cached dependencies, one compiler job, a 2 GiB/300-second guard,
8/6 GiB admission and 2/2 GiB runtime floors. The green run retired naturally
at 2026-11-17T02:20:46.547Z with root exit 0, final active processes 0, cleanup
confirmed, peak 799,502,336 bytes and no resource/abnormal-exit flags.

This is focused SDK/source-wiring qualification, not a full Tauri build or
live GUI acceptance. No owner database, private credentials, radio, output,
GUI automation, version, package or installation was touched. The full native
callsite compile remains the next integrated candidate gate.
