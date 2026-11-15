# Native selected-contact read lane

The native source slice on Nexus base
`aa237af25e82b3fd08ecac183e08f80a2381fd0e` adds
`xlog_contact_detail_read(connectionToken, contactId)`. It has no frontend mount,
contact editor, hardware access, package or version change. The
[SDK read contract](CONTACT_DETAIL_READ_VALIDATION.md) remains unchanged.

Connection setup creates the optional detail client after validated hello and
snapshot reads, using the existing native connection token. Capture happens before
`spawn_blocking`, binding the selected ID and original read time. An atomic permit
admits at most one queued/running detail job; a busy new selection invalidates the
old selected authority and reports busy without scheduling another HTTP request.
Dropping a prepared request releases admission. HTTP uses the independent detail
client's `try_lock`, never the ordinary client, station, RX, Stop or global session
lock. Capability absence and owner unavailable/busy responses are explicit.

Import, connect, disconnect/forget and failed poll retirement revoke native and
SDK scopes immediately without waiting for detail HTTP. SDK session authority
also rejects auth/owner/worker changes. The response retains and serializes
`ContactLease` itself, with native scope checks before and after serialization;
it never converts the lease to a cloned bare envelope. The existing response
shape is `{connection_token, payload, remaining_valid_ms}`. Consumers still must
compare selection and connection identity and use their pre-invoke monotonic
time for the remaining lifetime.

## Completed focused gate

Thirteen exact headless tests passed, zero ignored: eleven tests exercise the
actual SDK/native lane against fake authenticated loopback HTTP, and two inspect
command construction, setup, capture ordering, registration and retirement.
They cover the unchanged actual synthetic Swift found/deleted/not_found payloads,
unsupported/invalid inputs, one-job admission, cancellation, busy selection
changes during HTTP, immediate retirement, serialization after retirement,
typed/stable unavailable results, independent client locking, foreign prepared
requests and final serializer callback revocation with a positive control.

The harness freshly compiled actual SDK and lane source using pinned Rust
1.93.1 GNU and copied only qualified dependencies into a new isolated directory.
It did **not** compile the Tauri command macro or run an application. Full native
all-target checking remains required before a candidate package; these source
construction tests do not substitute for it. There is no actual owner endpoint,
UI, installed process or station qualification from this gate.

| Source | SHA-256 |
|---|---|
| Native lane | `33F41541E5862DF7C4BD4CECDD50831DA20215DC2E2D0391F787A7FBF191F3E6` |
| Native tests | `2D6D19A3FCD0807E7BBEF8C7A0C1BFB4728A4170C206894847BB026A0791DD86` |
| xlog_mode wiring | `6A4A8A4745EE223FBD92BFA5BDB4310D66674EC4EB3FE01403AE725657140B58` |

Local evidence in the `contact-detail-native` worktree is
`target/attempt-3b5c723a38a140bb882ad8cad12ead85`:

| Evidence | SHA-256 |
|---|---|
| Final receipt | `2DFB202DA4E070013C1B4AA89E2E005342AF50BFD68805ACD096166B2B98D2C6` |
| Guard receipt | `A9900746311619E123502E9128EFD6ADF209392FC12EBA33199E9C39593EA6DF` |
| Test log | `42B00B95D0CE0F30A87D6F380F2540C194BF437FF3045FF7FC72E85EC622602A` |
| Reviewed inputs | `EB195FF4F088258A31DAD80237154DE81F4D54AFB0C1615F5B07FC92285CC1DD` |

The 2 GiB / 300-second guard used BelowNormal priority, 8/6 GiB physical/commit
admission and 2/2 GiB runtime reserves. Peak aggregate memory was 592,068,608 bytes.
Root/effective exit codes were zero, natural drain was 19 ms, and final active
process count was zero without forced cleanup or resource/abnormal-exit flags.
All 29 source pins and 86 dependency files remained unchanged.

One exact source-byte difference from the previously qualified SDK was explicitly
reviewed: Git checkout removed one CR from the appended module boundary in
`contact_detail.rs`. Current SHA-256
`995B6E3B66C6F1456F3DE854F3D393F2BE5C72EF6AEC5A3B3839F20113BAFAB3` and prior
`E02B6BCE28060327C0F81EC2B0B368A1257CF1B93BBFD7A1147422AC8797A7E1` were both
pinned, with exact one-byte size difference and LF-normalized equality. No broad
normalization exemption was used. The harness compiled the actual current bytes;
all other SDK Rust files and dependency manifests matched the qualified source.
