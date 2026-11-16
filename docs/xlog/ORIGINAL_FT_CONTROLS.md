# Original FT controls through xLog

This source change connects the original Nexus FT8/FT4 cockpit to the native
owner transmitter. It does not start the upstream standalone station engine.
The original TX latch, Tune, Stop, CQ, Work, period selection, Drive, frequency
marker, Hold, Skip Tx1, manual messages and Log issue typed native intents.
Auto period selection and automatic QSO sequencing remain separate controls.

Work carries the immutable decoder row ID. Neither the browser nor the roster
reconstructs a decoded message as authority. Physical transmitting indications
continue to use xLog station observations; the sequencer's historical keyed flag
is not a physical PTT observation.

The frontend polls without arming, measures freshness from request start, rejects
foreign or stale connection evidence, and never retries a mutation. Stop and
disarm bypass pending ordinary requests and retain access after stale status.
Log displays only the native current-QSO outbox result and does not automatically
retry an uncertain submission. A commissioned route can offer Tune before the
first complete received period; native CQ/Work still require actual RX context.

## Source qualification

- Original-host bindings, transmitter lifetime and exact decode gesture: 17 tests.
- Existing cockpit service/CQ/clear behavior and exact decode gesture: 49 tests.
- Strict application/node TypeScript and production Vite build passed before the
  narrow mode-null availability correction. The final source check found a test-only
  optional Tune callback missing a non-null assertion; this is corrected. The
  candidate build supplies the final full compile. A narrower1536MiB Node heap
  attempt exhausted its heap and is preserved; known2560MiB profile is retained.
- Actual original host, mocked native transmitter, three private headless Edge
  viewports: TX enable, CQ, Tune, Stop, Hold, all period choices, Skip Tx1,
  exact decoded-row Work, explicit Log receipt, stale-start refusal and reachable
  Stop passed. No global desktop input, fallback native commands or hardware IO.
  Input manifest SHA256:
  `8EED07F8BD040D8454C37C839C1C156530056EC4771B4D03B625EA8E2B089B41`.
  Result SHA256:
  `C7ADDC95A3F03D00F5977EAD5CE7E224FCED55B2A9E41E73111F509605C6A735`.
- Earlier type failures are preserved: a void mutation receipt was incorrectly
  treated as a full snapshot, and the optional connection token needed null
  normalization. Both were corrected, without synthesizing station snapshots.

This is frontend source qualification. Native/owner integration, final packaging,
installation, DAX level adjustment and supervised on-air acceptance are separate
release gates. No live radio transmission was used for these checks.
