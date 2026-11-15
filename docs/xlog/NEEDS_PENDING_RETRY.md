# Pending decoded-call lookup qualification — 15 November 2026

The measured owner poll interval was 1,738 ms, while SDK HTTP queries have a
1,000 ms deadline and evidence expires after 2,000 ms. Rotating to a different
32-call group after each transient miss discarded the opportunity to consume
the asynchronously prepared owner result, particularly with 179–200 rows.

The native lookup lane now retains one group for at most six SDK query
invocations or six seconds from its first invocation. It preserves immutable
query IDs, revalidates original rows/context on every retry and attachment,
and advances fairness after useful completion or bounded abandonment. An
ordinary expired snapshot defers without consuming another attempt or renewing
the residence limit. Retired identities, route changes and pending writes cancel
the group. HTTP cadence remains at most once per second; PCM and Stop remain
independent. No evidence TTL, protocol, decoder or SDK source changed.

Cache eligibility now requires a positive `remaining_valid_ms()`, matching
render/attachment eligibility. The prior predicate could retain an unusable
cache hit during the final fractional millisecond of its lease.

## Focused evidence

Actual production SDK, assembler and native worker modules were compiled with
Rust 1.93.1 GNU against hash-verified retained decoder dependencies, in a private
2 GiB Job, one job, BelowNormal, five-minute limit. Final attempt
`attempt-5c9378da2d5e44768640330f65978fb2` passed 23 tests (6.90 s test execution),
including real HTTP 503 → same-ID retry, continued PCM and independent Stop,
row/context retirement, pending writes, original TTL and revocation.
The independent-phase virtual matrix already passed in the first attempt:
179/200 rows, 1,738 ms owner polling, 1,852 ms initial metadata age, independent
snapshot/query phases and 100/218 ms delays. It was not repeated for the final
cache predicate change; its scheduling source remained unchanged.

All evidence is retained under
`C:\Projects\OM0RX-xLog\.build\xnexus-needs-pending-evidence`.
Final guard SHA256:
`85D3BEDCBE0391CC5E503DB65466A51A411216FB0DE50D76E5B71EAAB54D26CB`.
Source-manifest SHA256:
`0F688D1F917AF18E862E1644976875EBF3B79E7B75BC4DD1D56E4BA8AD3A0255`.
The root exited 0; natural drain and cleanup confirmed zero descendants,
without termination or resource/abnormal-exit notification. Peak Job memory
was 832,565,248 bytes.

Earlier failed attempts remain preserved. The first exposed a one-shot helper
transport failure and an inadequate artificial lease-refresh fixture; the latter
was replaced with the continuous actual HTTP/PCM fixture. The second exposed the
fractional-millisecond cache/view discrepancy, corrected above. Neither failure
was relabelled a pass. Full application/platform gates were not rerun for this
focused change. An actual owner QueryCache/Mailbox → SDK → generated FT4 worker
acceptance chain is a separate pending gate; no physical station acceptance is
claimed here.
