# Passive bearing native lane — focused qualification

Base: `37c0c0f1da88ad1ab86c7be3de91f0f359f579e0`. This draft adds the
`xlog_bearing_read(connectionToken, callsign)` IPC command. It does not add a
bearing UI, a resolver, a Point action, or motion authority.

The caller's exact connection token is checked against the current connection.
The lane then retires its previous native selection synchronously, before
capability, busy, context, or callsign admission can fail. A checked selection
counter and distinct `Arc` prevent A→B→A from reviving a prior response. An
unsupported owner does not prevent connection establishment.

Only the main SDK client's `try_lock` is used to capture its opaque bearing
context. This does no HTTP and cannot wait behind a main poll. The original
snapshot deadline remains the upper limit for the read and its result. Capture
occurs before `spawn_blocking`; one permit bounds the dedicated lane to one
scheduled or active read, without a queue. A failed new selection retires old
display authority even when no SDK request can be captured.

The dedicated client mutex covers its own read only. Global session, main SDK
client, and Stop locks are not held through that HTTP request or serialization.
Connection begin/disconnect and poll failure revoke the lane synchronously;
main SDK observations also retire changed saved station identity authority.
Each response retains its original SDK `BearingLease`, native connection scope,
and selection until serialization finishes, checking native authority both
before and after the SDK serializer. A typed unavailable response can retain
its owner's reason within that same lease; a plain 503 remains an error.

## Verification boundary

The focused harness compiled the actual SDK and native lane modules; all 19
new bearing tests and 13 unchanged contact-detail tests passed, with no ignored
or filtered cases. Scoped GNU 1.93.1 rustfmt checks passed. The tests exercise the actual four
Swift-produced bodies inside explicitly synthetic HTTP envelopes, selection
and scope retirement, main-client contention, stalled HTTP, serialization
retirement, snapshot expiry, changed identity, plain versus typed 503, and an
actual independent SDK Stop request against a synthetic server while bearing
HTTP and the ordinary client mutex are held. No hardware request is made.

The harness reads the actual `xlog_mode.rs` for construction guards. It does
not compile Tauri's command macro, the full application, or its linker inputs;
an integrated native check remains required. The direct SDK build uses a
fresh output and copies 86 hash-pinned third-party dependency files from a
retained qualified GNU 1.93.1 donor without modifying it. Source, tools, donor
files and qualification receipts are pinned before and after the attempt.

The resource profile was the existing D378 private Job guard: 2 GiB
total descendant memory, BelowNormal, one serial test thread, 300 seconds,
8 GiB physical / 6 GiB commit admission, 2 GiB physical / commit floors, and
three seconds of natural descendant drain. The single approved attempt ran
15 November 2026, 18:48:10–18:48:41 UTC. Root PID 1684 exited 0, with zero active
processes at root exit and after cleanup, natural drain confirmed, no memory or
abnormal-exit notification, and peak job memory 637,698,048 bytes. Source, tool
and donor hashes were unchanged. No retry or resource-limit change occurred.

Evidence directory, retained outside Git:
`C:/Projects/OM0RX-xLog/.build/xnexus-worktrees/bearing-native/target/bearing-native-preparation/attempt-f21eb87b880a4279bb90a78f1bfb4e54`.

| Evidence | SHA-256 |
|---|---|
| `receipt.json` | `B8BD700383E65DA88672FE99100C12D1E7A80EA44DA67D054F9E1D0C077E892D` |
| `checks.json` | `7AEEEFC655B0E1A17EA7876C17976689175845E5FD533B28897892E64131166F` |
| Source before and after manifests | `71333384C7869BDE850ECD8C78669186B18BBF2E8702D9E8715F9D307201C335` |
| `tests.log` | `21BFDDE447EAAB2BB2E3A3578310CC84C9B1E116788E1A3EA5E065724F914194` |
| `list.log` | `60A1FE1B9C9943D7FEFC27599BBFE5B492384A7FD789685BEA45E97AA17BA286` |
| Native test executable | `53F5EE38D27B135F8811515876F45A30C023654304D12BACCE06E6E7CA9C2186` |
| Fresh SDK rlib | `3ACBAEBB49CB1E8D1C687F5EF111D36FEAA25CD8697C5A1DA3E79215B3465874` |
| `xlog_bearing_read.rs` | `3F05FBBB043CAC7F461DC9F0C48108B43C07129E3B1216EE10C83687C552DD47` |
| `xlog_bearing_read_tests.rs` | `49B13F17377AB19DA971D26CA798F5B662D87D8ECDFE611B70D5A4FD584829F5` |
| `xlog_mode.rs` | `D0914E5E5CF6DF167F9401566CBB1F9F9B8FD8B130A810202B3F9A4354E6632E` |
| Actual Swift bearing corpus | `307BD860A42BC5A3191FA0F421BFCA32AB0C27CC44DF0843840AE0F57B43FE8D` |

Preparation pins remain in the adjacent `bearing-native-preparation` directory:
`pins.json` SHA `71B55913A6D90BAB8707356D32266A50B479EC590F6CE0B25EC6737A93C3D45C`,
runner `A0E610A19AD030B38A82E8B957EE134E9FCC7D106DE36D2CBC613BC42A096978`,
child `F0D16863EDFDDCE4505159203575857AA1F946CA4A4CAC6431110053E85FBBA1`.
The passing direct-module tests do not claim an actual owner endpoint, full
Tauri command type/link check, installed application, UI, rotor or RF acceptance.
