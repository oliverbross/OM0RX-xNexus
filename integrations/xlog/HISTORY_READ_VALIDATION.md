# History and recent-contact read validation

15 November 2026. This is SDK/source qualification, not a packaged or installed
application qualification. The installed application and version are unchanged.

The integrated source is based on `7e36f9e7a6e792f8f845ed1e2ea4aef09635e38f`.
History changes preserve the existing station-identity validators, identity tests
and audio authority guards byte-for-byte. The additive TypeScript API retains
station identity and exposes owner payloads without creating Nexus `LoggedQso`
records. Native command wiring and frontend consumers have separate validation.

## Combined SDK result

The bounded GNU Rust 1.93.1 run compiled the actual integrated SDK afresh, passed
97 synthetic tests, separately ran both producer replay tests, and passed
all-target Clippy with warnings denied. The three normally ignored producer
tests remain explicit gates; the older needs-envelope replay was not rerun here.

Evidence directory, relative to the xLog workspace:
`.build/xnexus-worktrees/history-integrated/target/history-integrated-validation/attempt-47b8f78ad51a44759af27ceabcd29bc3`.

| Receipt | SHA-256 |
| --- | --- |
| `receipt.json` | `A929E1340E71441ACCBF66C236FE53CFBD2B51932F511E622B7733F935CCFACB` |
| `guard/guard.json` | `8A8B239F453A8E9BFB5FFB56101A62A75EDFA1E5419B228939ED26D110F08222` |
| `checks.json` | `18EBD170937E0634EED71F33057AE15A615B698C576424826A34133AA3285944` |

The run used a 2 GiB aggregate Job limit, one Cargo job, BelowNormal priority,
300-second deadline, 8/6 GiB physical/commit admission, 2/2 GiB runtime floors
and five-second natural drain. Peak Job memory was 579,096,576 bytes. Root and
effective exit were zero, final active processes zero, cleanup confirmed, with
no forced cleanup or memory/abnormal-exit notification. All source/tool pins and
1,702 donor-cache hashes were unchanged. The receipt's top-level initial
`actualProducerReplay: pending` marker was not updated by the wrapper; the nested
checks and separately hashed replay logs record both completed passes. The
original receipt is preserved.

## Actual producer scope

The history replay consumed the actual Swift test exports unchanged:

| Export | SHA-256 |
| --- | --- |
| `history-fixtures.json` | `D783191924D1728429C6A2BB68DBC244F88CEAB5D77A2B2D349FC640B9596116` |
| `history-fixtures.lifecycle.json` | `46D7D1FD8E803018418B4A1F82FB930BFB0B91F1C4F82AB0B890861131BF3545` |

Seven payloads passed structural validation at their original generated times;
two retired-workspace responses remained refusals. Expired payloads failed at
the actual current clock. The only added fields were a labeled synthetic public
protocol/server/workspace wrapper matching the producer generation. No contact,
summary, policy, revision, timestamp or completeness field was rewritten. This
is cross-language DTO evidence, not installed HTTP or station evidence.

The unchanged actual identity projector corpus also passed all six cases;
its input hash is `3557063A96591AA695224780D2725393E356B7A689DA15789B6B33FC59678D9E`.
The combined history replay log hash is
`9F272BB620FCB5EC7B231EF22578AE0744FEC16CC61A740C7A0CDED0687D01EF`;
the identity replay log hash is
`F5700CCF7E0CD3F562B63D069822CD05F13BB3272400E42592249E5C7416EC94`.

## Integrated frontend type and bundle gate

The original-host integration passed full `tsc -b` and Vite with this exact
additive API (`AB9ED9D83D092722177CD20AE2C3453949BA836DD81E0A29B58B9DFC8FB712B8`),
including the existing station identity, history reader/Recall and spectrum
overlays. Its receipt is
`.build/xnexus-worktrees/original-xlog-host/target/host-types-vite-28e94c378e0041df91f1cad0abf0755c/receipt.json`,
SHA-256 `5679771FE25F3965136C987516ECD32212C74E1867FBEC623AC1B3CCB4B05A55`.
All 2,589 pinned inputs were unchanged; root exit and final active processes were
zero, with cleanup confirmed. This qualifies the combined TypeScript contract
and bundle compilation; browser and installed UI behavior remain separate gates.

## Remaining boundaries

Actual UI consumer checks remain separate. The focused
native read-lane tests do not compile Tauri command macros or qualify the full
shell. No package, installer, native launch, production log, device or RF operation
was used in these SDK checks. Keep these boundaries explicit when promoting the
source; no live history/recent UI pass is claimed here.
