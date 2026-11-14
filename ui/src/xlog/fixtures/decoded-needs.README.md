# Native decoded-needs fixture

`decoded-needs.json` is the exact JSON emitted by
`xlog_rx_worker::tests::native_serialization_fixture_preserves_exact_owner_and_unknown_fields`
on 2026-11-14. The test serializes the actual Rust worker `Row`/`View` types
after an authenticated SDK fixture lookup. Its known K1ABC and Unknown W9XYZ
rows are synthetic; this is a serialization contract test, not a live radio test.

SHA-256: `00a4efc0828fbc78cd88aa8743ce5ec5ddeea316b726a8d504685e66320fb3dd`.

The generating native suite passed 19 tests. Evidence is retained under
`C:\Projects\OM0RX-xLog\.build\xnexus-native-needs-evidence\attempt-8a9a8c5fb2b44f5caaac74f459440a07`
(`compile-receipt.json`, `guard.json`, `tests.log`). The receipt records the
pre-commit source hashes; the resulting native implementation is committed as
`d82c0d58e514dba1b0f5005e430ba937de0866d1`.

`RxNeeds.test.tsx` consumes the JSON unchanged, with a controlled monotonic
clock. The owner wall timestamp is display-only; acceptance uses the native
remaining lease measured from before IPC. The test deliberately retains all
explicit null fields, native enum spellings and route/context fields.
