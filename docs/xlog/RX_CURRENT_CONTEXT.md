# Native receive context before the first decode

`xlog_rx_status`, `xlog_rx_start` and `xlog_rx_stop` serialize the existing native
`RxView` with an additive nullable `current_context` field. It has exactly the
existing decode-row context shape: worker instance/epoch, owner server/workspace,
complete audio route, and selected FT8/FT4 decoder mode.

`Worker::view()` expires old authority, refreshes needs, then derives this field
from enabled current native audio authority, route and mode under the same State
lock. It is independent of decoded rows. Stop, reset, stale authority and absent
mode/route yield null. No new owner HTTP request, clock, device or lease is added.
Old UI clients can ignore the additive field. New waterfall clients must treat
its absence as unavailable; they must not invent a worker epoch or infer it from
a first spectrum response or a retained decoded row.

Focused source check on base `c686560bb7e8938b783581f13d8d31fd9a76b686` freshly
compiled the current SDK and actual native Worker against the qualified,
source-identical assembler/DSP inputs. All 36 native Worker tests passed, including
context before any decoded row, exact serde shape, ordinary expiry/missing mode,
and actual Worker stopped/running-gap/restart current-context checks.

Receipt in the isolated `spectrum-context-client` worktree:
`target/spectrum-context-check/attempt-8af250da3ef249eaab55905918c37573/receipt.json`
SHA256 `C48C7B4C217E5CA252173559A047BCB474E83DB7D821E8924D6E0D20C1F45D67`.
The 2 GiB/300-second BelowNormal job exited 0 naturally, final active processes 0,
cleanup confirmed, peak 842,534,912 bytes. Admission 8 GiB physical/6 GiB commit,
runtime floors 2/2 GiB and three-second natural drain remained unchanged.

Exact Rust-produced fixtures, from generated PCM/synthetic local HTTP authority:

| File in receipt directory | SHA256 |
|---|---|
| spectrum-frame.json | F984D072CDA492C257F0EFC99E655D16DBD4C0CB466402E845F6275C1B1AD15C |
| rx-context-view.json | 66661FA17D0ECA4F67F8EBF6F9CC7AA07547C21ABF4B405B84ECC50540F18ED8 |

These establish native serialization, not physical owner capture or station
qualification. Tauri application compilation and original cockpit rendering are
separate integration gates. A serialized context/frame is a snapshot, not a live
revocable lease: the UI must also fence session/lifecycle changes and use the
monotonic IPC request-start plus the original remaining validity for display.
