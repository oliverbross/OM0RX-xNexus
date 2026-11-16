# Native original FT host integration

The host owns one original `tempo_core::qso::Station` coordinator and uses the
xLog SDK's authenticated digital TX client. It owns no radio, playback device,
PTT backend or local logbook. This source does not authorize a physical test.

## Runtime boundaries

`xlog_rx_worker` publishes the actual accepted `NativeDecodedPeriod`, exact UI
row identities and its original audio lease to a one-entry replaceable mailbox
before presentation. Epoch and freshness checks remain on both sides of DSP.
The host checks current audio authority again before observing or selecting a
row. Up to 200 original row IDs retain their actual period Arc for visible
Roster/Decode Work selection; current context/audio authority is checked again,
and retirement clears the selection cache. These records are never replayed
into Station. There is no UI-message reconstruction or decode catch-up queue.

The saved owner identity, connection token, radio/receiver/frequency, RX epoch,
TX context and clock epoch bind a coordinator. Changed authority disarms it.
CQ, directed CQ, exact-row Work, manual target/message override, bounded free
text, resend, parity, auto-parity, auto-sequence, hold-frequency, skip-Tx1,
S&P, explicit Enable, Tune and Stop map to original control semantics. Auto
parity is separate from QSO auto-sequence. Zero drive inhibits transmission.
A new CQ waits for the first complete accepted RX period; an RX route does not
commission a TX route. Tune availability comes directly from the fresh
commissioned owner context and does not require an RX coordinator.

A preparation worker renders immutable final 48 kHz mono PCM outside the
coordinator, RX, connection and Stop locks. Prepare/commit use exact body
identity and continuously refreshed matching owner authority. The captured
slot/latest-start deadlines never renew. Native preparation starts several
seconds early and refuses commit less than 500 ms before the slot; the owner
still enforces its own readiness and first-frame limits. Frame zero includes
the source lead and complete FIR delay/tail, without compensation or trimming.

Heartbeat runs independently every 200 ms while an owner operation is retained.
Explicit Stop captures an independent release-only SDK handle before dispatch;
connection retirement cannot erase that obligation. Unknown outcomes retain
it. An exact sealed actual-start proof advances the original frozen Station
once. A first terminal receipt containing start accounts start before release.
Prepare/commit acknowledgement alone never advances Station. Duplicate terminal
proofs cannot release or stop a replacement operation.

Turning TX Enabled off disarms future acquisitions, unlike explicit Stop. The
owner cancels an operation that has not started, while an already-started over
may finish under its retained heartbeat. Exact start/release accounting remains
valid after future disarm. Stop, lost route/authority and watchdog expiry still
request immediate exact release. There is no automatic rearming after reconnect.

`xlog_ft_status`, `xlog_ft_action` and `xlog_ft_log` are registered in the xLog
entry only. FT status exposes the original QSO state and a conservative owner
lease budget for frontend request-start expiry. Stop remains available for a
retained old operation when ordinary authority is unavailable.

Explicit Log freezes one stable request/contact identity for the current
selected QSO, including an explicitly selected target before transmission.
Unsent reports remain blank; a new target or CQ clears the prior-QSO fallback.
The draft preserves its original owner workspace, durably enqueues first, then uses
the existing xLog submit/receipt path. An uncertain write is reconciled, never
automatically resent. The command returns only that QSO's one outbox entry;
queued or uncertain is not presented as an applied log entry.

## Qualification

Focused native evidence and the full Tauri check are separate. The focused
harness compiles actual coordinator/encoder/output/decoder/host source against
the qualified SDK and retained GNU 1.93.1 native dependencies. Its loopback
owner and actual-start events are explicitly synthetic; no physical output,
GUI, package or installed/live station acceptance is implied.

The first committed host focused run passed **45 tests plus one explicit FT4 export test**.
Evidence: `owner-ft-runtime/target/host-attempt-13f217d0e07640c3bcc40e5536fb4fae`.
Receipt SHA-256 `E0F5C7211372E1E4FAC35E4993B59E32EDA4F9F085D9ABB116CDBCFD8EEA6D31`;
checks `53DA5D34277F199C31BFEF6BB0D48E29F14E675AF8B70AA3BD56A28A1DB5EC59`;
guard `CAE4FCC2A397F244162C34827EC31942C0A6E832F9F62FC5FEB9E4768225E73A`.
Input manifest `5490BC86CCE77AD1802E278B73E18B45E8BB036358723ED111A03F772858AE04`
pins the actual native source, SDK library from `c1fe0124`, tools and retained
core dependencies. All pins were unchanged before/after. GNU 1.93.1 fresh direct
compilation ran under 2 GiB/300 seconds, BelowNormal, one job, fresh 8/6 GiB
physical/commit admission and 2/2 GiB floors. Root19232 exited naturally with
final active0, no resource flags or forced cleanup, 25 ms drain and peak
884,281,344 bytes. This is not Cargo/Clippy or full Tauri qualification.

The tests include actual generated FT8/FT4 decoder/output controls, original
Station behavior, future disarm before/after actual start, an independent Stop
winning during verification, separate heartbeat during blocked owner HTTP,
exact native Work/parity/hold, terminal-first sealed start then release, durable
QSO identity, and duplicate terminal proof isolation from a replacement.

The earlier `1e497be1` run is preserved failed (42 pass/two failures): its
Stop-versus-disarm bit and enable/heartbeat race were corrected without weakening
the assertions. `4510a9fe` is preserved failed (44 pass/one fixture failure): the
dormant test host's second operator intent needed a fresh owner observation,
not renewal of an expired lease. `d63b3454` remains the earlier 44-test pass.

The explicit exporter produced synthetic actual FT4 final PCM: 1,440,000 bytes,
SHA-256 `91CFD59268BC6037AB32232970FED4CE20876D95B865DE6F526BED0E44CCD7D8`;
manifest `3F494E1EBD94EC717B93393934B537F85A75441F5FCF047374A6E8306F3D2BB8`.
The owner's independent validator replay is a separate gate.

**Separate full Tauri metadata checking was deliberately not run** after the
final Tune slice: the coordinated canonical candidate build will compile this
integration once, including IPC registration and worker feed wiring. Frontend
and owner/backend qualification remain separately evidenced.
No application version changed; no package, installation or physical TX ran.

## Tune drive and original interaction follow-up

Tune now requires explicit finite `gain_f32_bits`, greater than zero and at most
0.8, bound into the existing exact JSON body digest. A user drive change during
Tune stops the exact old operation, waits for observed release, and only then
creates a new immutable Tune with the new gain. Intent/context changes, urgent
Stop or uncertain release prevent restart. Zero drive stops and inhibits.
The original FT coordinator is future-disarmed when entering Tune.

Focused evidence is **26 SDK TX passes and 51 distinct native cases passing
across recorded runs**, not a claim that the aggregate native run was green.
All runs used the unchanged 2 GiB/300-second, BelowNormal, one-job GNU 1.93.1
profile with 8/6 GiB admission, 2/2 GiB floors and five-second natural drain.
Only the SDK test-harness compilation used opt-level=0; the fresh SDK library,
native coordinator/encoder and output compilation retained opt-level=1.

- `host-attempt-9a917ff2180c4458899eb4ff5c2259e9`: SDK26 passed; native47 passed,
  four failed, one explicit-export test ignored. Receipt
  `DBE4D3D6F80CD7D25A0222D0951E8D90D8C20DCD21C1C85EC18350A99CDB955D`;
  SDK log `94EEA217DE3D9ED2FB575C26092ABD0761CD12C61D1E145CF466820C254E38AB`.
  The terminal-first control returned `Stale`; its exact release obligation was
  retained, so the following three Tune controls refused a replacement. This
  failed aggregate remains preserved. Its underlying intermittent admission
  timing cause is not established.
- `host-attempt-e8ea9531868247e3945685b693c876d7`: fresh native compilation with
  diagnostic stage labels, then the exact terminal-first case passed in 1.89s.
  Receipt `DDDD52356C106088D1108F53C124B6AD21C284CEB8839528BDEB7B93805E3037`;
  guard `632EDB1D6C7DBF8BCECBB170AC9F8DA88C936A393267614382E181E611841CF4`.
  No runtime lease or timing workaround was inferred from that pass.
- `host-attempt-7cdafbebac1d4325b2287430ba8c3739`: the same exact compiled native
  binary ran the three Tune controls in separate test processes. All passed:
  observed-release replacement, unknown-release refusal and urgent-Stop
  cancellation. Receipt
  `F6F7D57EB685300B3F99D55DFEAD2C02ECC7C8FE2C85812BBA0377F834191904`;
  guard `37C812868636431D0888737053A855F19F5813EC0ECB76C6B079B18D75166894`.
  No in-test release reset or lease widening was used.

All children retired naturally with final active zero; source/dependency pins
were unchanged. The new availability, bounded original-ID selection and
unkeyed-current-QSO Log regressions passed in the aggregate run. The previous
`4a8fea2b` and `7d2c2b42` SDK fixture failures also remain preserved: rendering
and independent malformed cases now obtain fresh admission rather than reuse
an expired one-second lease. No lease duration changed. No new export was
needed: final PCM/manifest code and the retained actual FT4 export are unchanged.
No new version, package, application launch, hardware or RF action occurred.
