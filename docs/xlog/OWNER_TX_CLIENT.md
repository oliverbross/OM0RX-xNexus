# Owner-mediated digital TX client

This SDK slice provides authenticated transport and immutable owner receipt
evidence. It does not encode, schedule audio, open devices, assert PTT, or prove
RF transmission. Native integration and owner backend qualification are separate.

Source base: `0c32b14ee4c7f5185406368d821095b25b8a4aed` (the completed needs fix,
also integrated on canonical main as `3c61a056`). The owner wire definitions were
read from `xlog-digital-tx/prototype/src-tauri/src/companion_tx_contract.rs`, SHA256
`1208BDA0E4B86AB724E76C279C2B02E1258830ED0EE32336F7549E0BE898EC17`.
The DTOs reproduce that protocol boundary, not the owner's implementation.

## Admission, Stop and proof

`BridgeClient::tx_handle(client_id)` requires `digital.tx` and a canonical non-nil
client UUID. `TxClient` supports shared `Arc` ownership with `&self` methods.
Context/heartbeat admission is separate from the ordinary prepare/tune/commit
permit; neither is held by Stop. No SDK mutex is held during HTTP.

Each context lease retains its original monotonic deadline, bounded by both the
request start plus `max_age_ms` and the owner's generation time. A later heartbeat
does not renew an existing lease. Context changes/errors retire admission.
Prepare may return historical evidence after its admission lease expires;
commit requires a fresh matching context. Public DTOs are not proof.

`PreparedOperation` seals the descriptor, PCM, body digest and original binding
before dispatch. Its exact Stop handle therefore exists before a prepare timeout.
The Stop handle retains the authenticated endpoint independently of ordinary
`BridgeClient` retirement, like the existing rotor Stop path. It exposes only
exact-operation release, never retargets a replacement, and retires on owner
401/403. The owner must retain a Stop-before-prepare tombstone; its implementation
and tests are separate. An unknown-operation Stop response cannot invent a
receipt or a release observation.

`ValidatedReceipt` has no public constructor or deserializer. Only bounded,
authenticated HTTP replies matching the immutable operation create it. Cached
proof survives transport failures as historical evidence. It does not grant a
new command or an automatic retry. Phase and outcome are independent:
Prepared/no_start is an acknowledgement, not a completed transmission. Only
Terminal/no_start concludes no start. Actual start and release fields both
survive a first-observed terminal reply. The native adapter must deliver Started
before ObservedReleased so original station accounting is not lost. Final
NoStart/ReleaseObserved cannot change; terminal uncertainty may refine to release
evidence. Receipt timing is owner presentation evidence, not an RF measurement.

## Body and dependency bounds

Controls/replies are limited to 16 KiB; prepared bodies to 4 MiB. Prepare has a
2-second HTTP timeout; other requests use 1 second. Prepare descriptor/manifest
schema 2 requires final 48 kHz mono f32le, exact whole-period frame counts,
filter/gain/support provenance, positive-zero padding and finite bounded samples.
The PCM, content and whole transmitted body have separate SHA256 identities.
The body includes its u32LE JSON-length prefix and exact descriptor bytes.
Tune is explicit, lasts 1–30,000 ms within owner-advertised frequency bounds, and
does not automatically extend or restart. Owner heartbeats and exact Stop remain
required; this SDK is not their scheduler.

`sha2 = "=0.10.9"` was already used by the pure preparer and listed in
`licenses/rust/THIRD-PARTY.txt` (MIT OR Apache-2.0, MIT elected there). Its cached
archive checksum is
`a7507d819769d01a365ab707794a4084392c824f54a7a6a7862f8c3d0892b283`.
All 142 source files across the eight added dependency packages matched their
checksum-verified cached archives. No download or existing package version change
was made. The offline resolver's unrelated WASM downgrade proposal is preserved
only as diagnostic evidence; the final lock adds the SHA2 closure alone.

## Qualification

Evidence root, outside Git:
`C:\Projects\OM0RX-xLog\.build\xnexus-worktrees\owner-tx-client\target`.
GNU Rust 1.93.1, offline/locked, one job, BelowNormal, private 2 GiB Job cap,
300-second deadline, fresh 8/6 GiB physical/commit admission, 2/2 GiB runtime
floors and 3-second natural descendant drain. NativeJob helper SHA256
`D378039B162F807CB4E6949C75E5851CBB51DE42C38E82E44607F63A3377B986`.

* `tx-sdk-release-stop/attempt-042c83e39acb4ec089d64b816461109f`:
  **200 passed, 0 failed, 3 existing external replays ignored**, including all
  25 TX cases. Scoped formatting passed. Test-log SHA256
  `3F30B21DB6DCB90FA3F54929B6332B7953C7DA60C8E6F1DF357C69713105962E`.
  Its aggregate wrapper remains **failed**: the LF-only named-test regex counted
  zero in CRLF output, so Clippy was not reached. The retained receipt is
  `EF32056DD706144F8E1FC29D772192C019D1F7B433B568589D46EC59BE8945A3`.
  Root PID 2332 exited naturally with code 1; final active processes were zero,
  no resource flags occurred, peak memory was 793,702,400 bytes.
* `tx-sdk-clippy-library/attempt-e7e68c69f93348a98a8e1cf11997c1e2`:
  read-only qualification of those exact passing assertions, scoped formatting,
  **fresh SDK library and all-targets Clippy passed**. Tests were not repeated.
  Root PID 16716 exited naturally with code 0, final active processes zero,
  cleanup confirmed, no resource flags, peak memory 625,704,960 bytes.
  Receipt SHA256 `52500CB47619D798A606B3CF5476B92690EFD9AC9A1F438088C317440040205E`;
  checks `21B1A61D52A4E833D349BA3058F867964E7C3404EC2D8AE9902EDAD3145D800D`;
  assertion qualification `802C5D74AB057530953FC62214584886EAD94337985C4FBBBACB252AD09254F4`.

Both relevant runs have identical before/after source-manifest SHA256
`3FBB8A94D0F9376ACE835C6BE766BB02D6F0D9B4F0E61DEC44614EE19EF0025E`.
Tool and read-only donor pins were unchanged. Fresh library
`libxnexus_xlog_client-7f2457b8816749dc.rlib` SHA256 is
`A56B38B2556CFFE9C4B13B050A9AFBA6B8C9C78F5766D7FDEFE26AC82D741132`;
the companion dependency inventory is
`2C9536D7BC02988DB027B1515ACBB5BD275F30A386677683AEACB5D0F0BC0882`.
The SHA2 cache provenance record is
`4684AB30FD4EA5B9A99C5F815409932C70E839B55D90D23647C7759DC8E3D292`.

The 25 synthetic cases cover capability/identity/null/schema constraints,
first-enable admission, fixed deadlines and revocation, same-stamp nonrenewal,
context replacement, delayed fresh/expired context replies, both-mode body
geometry and digests, malformed samples/provenance, Tune bounds, receipt
monotonicity and immutable start evidence, exact slot timing, foreign handles,
blocked prepare with independent Stop, BridgeClient drop before Stop, revoked
Stop authentication, missing-status uncertainty, no automatic mutation replay,
and authenticated terminal start plus release. Synthetic PCM is a finite pulse
for body validation, not an encoded modem waveform.

Earlier failures remain preserved:

* `tx-sdk-preparation/attempt-a9fa59b9584e4e57bcdd9c183fc6721a`: 195 passed,
  one failed, three ignored. The two-mode debug-hashing fixture reused a single
  one-second lease; the second admission correctly returned Stale. The fixture
  now obtains independent leases without widening the production lifetime.
* `tx-sdk-corrected/attempt-915e281ee9234accbace06dce1fdd5b7`: 198 passed,
  one failed, three ignored. The repeated-stamp assertion demanded exact Instant
  equality; conservative submillisecond shortening is valid. It now asserts no
  extension. The wrapper failure above remains a separate third record, not a
  rewritten green receipt; no exception text was captured, and its pinned
  artifact predicates reproduce the sole CRLF count mismatch.

No full application, Tauri integration, physical output, PTT, RF, package or
installed-station qualification is claimed by this document.
