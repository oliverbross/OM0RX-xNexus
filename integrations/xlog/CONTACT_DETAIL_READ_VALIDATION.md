# Selected-contact SDK read qualification

This is an SDK-only slice on Nexus base
`0829c6f5bb4223c27474cc0ef73af16602f0032b`. It adds authenticated selected-contact
reads without changing legacy history behavior, native commands, UI, dependency
locks or the application version. It does not qualify an installed owner endpoint,
native application, editor, or contact mutation.

## Read contract and lifetime

`BridgeClient::contact_detail_handle(connection_token)` requires the independent
`logbook.contact.read` capability. It inherits the existing authenticated IPv4
loopback client, with proxies and redirects disabled. It sends only
`GET /xlog/v1/logbook/contacts/{canonical-lowercase-UUID}` with no query, body or
Origin header. The local connection token never goes on HTTP.

The strict eight-key envelope contains protocol/server/workspace identity,
positive JSON-safe worker generation, workspace generation, the original owner
read-start timestamp, an exact 5,000 ms lifetime, and the validated contact detail.
The envelope is bounded to 128 KiB; the retained detail codec keeps its 120 KiB
serialized-detail and individual field limits. A separate schema-1 endpoint
policy validates revision relationships, status-specific revision/null rules,
nonnegative number facts, provider source names, the fixed unavailable Locate
result, reason codes and the ordered six-field editable allowlist. It does not
rewrite or tighten the general read-only retained DTO codec.

HTTP 200 accepts found/deleted/not_found only. HTTP 503 accepts either a fully
validated unavailable detail envelope or a bounded stable `{error: code}` result.
Neither is an empty logbook or a missing contact. Unsupported capability, timeout,
invalid response and busy states never trigger a native fallback.

Capture the request before scheduling HTTP. The cloneable `ContactReadFence`
permits immediate selection/connection retirement without waiting for that HTTP
client. Every admitted capture invalidates the preceding request before reading
the fallible clock; request epochs use checked increments. The response lease
retains session, connection, request and operation authority. Owner/worker changes
retire authority, and retained handles reject decreasing revisions. The caller
must revoke the fence on local connection retirement and must compare the selected
contact and connection token when consuming the result.

The same-host clock rule has no future-skew allowance: local capture wall time
must be no later than the owner's read-start stamp, which must be no later than
local receipt time. A wall-clock regression is rejected. Monotonic expiry is
bounded by capture time plus 5,000 ms and the owner's remaining age; repeated
timestamps cannot renew an earlier deadline. Clock/check failures permanently
invalidate that operation's facts, without invalidating a later operation. Lease
serialization checks authority before fields, after payload, and after the final
serializer callback. Frontends must still anchor `remaining_valid_ms` to their
pre-invoke monotonic time; delayed delivery does not create a new lease.

## Completed focused checks

Final client source SHA-256:
`32C95BA50A8C7F7C6DBDF262434F03D1E9CAEA3B403D14313769B19261DF0E3A`.
Client tests SHA-256:
`B061320BA25C5A639B3BBBD0511C86B60E52ACE0240C89E4CDC401E64B1CF46F`.
The reviewed owner endpoint validator was
`prototype/src-tauri/src/companion_contact_detail.rs`, SHA-256
`6911D786F7D145A1C0B3CAC6C604E3E85F37D6FF343D0056945067755E527F35`.
The SDK does not compile or copy that owner implementation.

All 37 focused tests passed: 15 codec tests and 22 client tests, zero ignored.
The actual [synthetic Swift corpus](fixtures/CONTACT_DETAIL_PROVENANCE.md) is
replayed unchanged both directly and through fake authenticated loopback HTTP.
Cases include fixed shape/nulls, byte caps, status/error distinctions, selected-ID
changes during HTTP, clock bounds, timeout, disconnect/revocation, foreign owner
and worker identity, revision regression, nonrenewal, checked epoch overflow and
serialization-time retirement. The final serializer test includes a successful
control and actual final-object writer revocation. The other 100 SDK tests were
filtered, not re-executed. All-target SDK Clippy passed with warnings denied.

Retained evidence in the `contact-detail-client` worktree:
`target/attempt-0e3f1d9b486f44d7bfb4b520adde0281`.

| Evidence | SHA-256 |
|---|---|
| Final receipt | `ABAD34EE8C55FE138AFAF0D9BEDEBDE17671BCCDDC0F43C5B76AA5B62C38C89D` |
| Guard receipt | `BC08FD3B6530DDB3120BF0F6A1B801E79C10BDEAAD453ADCFB4C1E996A146551` |
| Test log | `EDA70A6F5A0A80E532959BAEF85632BABC00408CB983110A3E400C3912957B8B` |
| Clippy log | `175237D70C0BB81183BDAD737DE6AF6EF218E55B638B9A1CC62A7D9BB24A686D` |
| Reviewed inputs | `0DD529FBF040873F81FFB74F7760BB1709BC964A878538F51ABB7EDDD9DE7965` |

Pinned Rust 1.93.1 GNU used offline locked dependencies and a fresh SDK test
artifact in isolated output. The job had a 2 GiB aggregate cap, 300-second bound,
one build job, BelowNormal priority, 8/6 GiB physical/commit admission and 2/2 GiB
runtime reserves. Peak was 653,193,216 bytes. Root/effective exits were zero,
natural drain was 34 ms, final process count was zero, and no forced cleanup or
resource/abnormal-exit notification occurred. All 28 source/provenance inputs and
1,689 donor cache files were unchanged; only dependency files were copied to the
new output cache.

## Preserved unsuccessful attempts

The first attempt compiled but six tests encountered Windows accepted sockets
inheriting nonblocking mode: 31 passed, six failed. Receipt
`924D5A597F343C1E37E35F737B83C4F28BE89749219EAE8330A3BE6E78B18271` and guard
`9A927AA690526EA93EB4926ACF75EE448B7199F0B9BF2BCF020E18CBD309AD75` remain failed.
The fixture was corrected to explicitly set accepted streams blocking, as the
existing SDK test server already does. Its original source bytes are retained.

The next attempt passed all 37 tests but Clippy rejected one redundant unit
expression. Receipt `7A286CCCF377DC26659D388286887C36CBC761559E5B3BCC84B56FFE8F33E81F`
and guard `B7EB86B2EA21B740B30978D5E94DBDA61D9D9C0E2480F3BB971C06ABDDBAA71B`
remain failed. Removing only that expression produced the final source qualified
above. Both unsuccessful jobs exited naturally with no surviving owned processes;
neither was a memory-limit failure or an installed application test.
