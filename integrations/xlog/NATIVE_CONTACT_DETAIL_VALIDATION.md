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
It did **not** compile the Tauri command macro or run an application. The separate
full native all-target check below subsequently compiled the integrated shell;
these source construction tests do not substitute for it. There is no actual owner endpoint,
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

## Completed integrated shell compilation

Canonical source `2103df7c6ad4fa0e1a23b7e4a2ef53e2cf8a0ad3` passed pinned
Rust 1.93.1 GNU `cargo check --locked --offline --all-targets --features radio -j1`
on 2026-11-15. Cargo recorded both actual `src-tauri/src/lib.rs` normal and test
`tempo_lib` targets with `fresh=false`, including Tauri command registration and
return-type checking. This check executed zero tests and launched no application,
frontend, installer, custom protocol or hardware. The thirteen runtime tests
above remain separate; full linked application execution and installed UI are
not established by this compilation.

Evidence in this worktree:
`target/contact-native-check-fb9ccf12e1a84b1986c0b1008efcb116`.

| Evidence | SHA-256 |
|---|---|
| Final receipt | `FE1FADADF0538D0F3928CD15FD3C011A1D21AD6D1F34E1B711441C75762BE98E` |
| Guard receipt | `2EC04B5FFCA750F18FD8CEBFECAB7D70D0E27C447C1FB3957C67FC130FCAB760` |
| Artifact checks | `795E36D61940912939024CF7BE42B744F2F83D03923107A4CF731F616BC1FA85` |
| Cargo log | `02B86DD50C75A60CE869C5723FC9497A3457D0BE25561B272F2BDA3A610BE8C6` |
| Cargo artifact inventory | `1FCD10D8B5466F95FF3F15A72077D5BCCA963027A05FC379289CB3DAD93CA45E` |

The unchanged 4 GiB / 1,800-second guard admitted at 8/6 GiB physical/commit,
retained 2/2 GiB runtime floors and one BelowNormal job. Cargo finished in 4m39s;
peak aggregate memory was 4,264,153,088 bytes. Root/effective exit were zero,
natural drain 12 ms, final active processes zero, without forced cleanup or
memory/abnormal-exit flags. All 5,295 source pins, 1,495 retained asset/evidence
pins and tool pins remained unchanged. The near-cap peak is observed evidence,
not a guarantee that future candidates fit this limit.

The first source staging receipt remains failed and preserved:
`83BAD3F81DA761BCF2255384D61CF01207A32E0ADD8B0EAAE32ED6B266149227`.
Its ordinary Git checkout produced LF instead of the canonical working tree's
CRLF for exactly `AGENTS.md`, `docs/xlog/BACKLOG.md` and `docs/xlog/UPSTREAM.json`
(39, 18 and 8 CR bytes respectively). Each actual checkout matched its committed
Git blob and declared `text=auto eol=lf` policy. No runtime, fixture or binary
changed. Ten prior source backups and five moved historical source files remain
intact. A separate read-only completion verification pinned both raw source sets
and the three exact equivalences; no staging rerun or source-byte rewrite occurred.
Its receipt is `583AD02DA42B8ED5B5B6B71CD99D1823E0C14C93CC6EA7EB52F1F45108FEB0AD`,
and the resulting compiler inputs are
`3F851C87A1CDBE2AA8475BDF9C6E8CCB163CC8542FAC5F94A713DCB825E72582`.
