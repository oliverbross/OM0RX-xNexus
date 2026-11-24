# 1.25.0-xlog.30: a clear transmit frequency for each call

Source: `main` at `5cfc68a1c` (`xnexus/autopilot-reliability`, fast-forward). Since
1.25.0-xlog.29 (`a6c881614`), the branch adds two commits: `87b03889e`, `5cfc68a1c`.
Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an
installed xlog.29 keeps its logbook and settings; the new setting (Settings, Autopilot: "Call each
station on a clear transmit frequency") is on until switched off, and an older version ignores it.

## What changed for the operator

- **Each station is called on a clear transmit frequency** (`87b03889e`, operator request 2026-11-24):
  the clear one nearest to the operator's own, by the signals heard lately in our own period (Band
  Activity, 12 periods back); with none clear, the widest gap. The operator's comes back for the next
  call and when the autopilot stops. A DXpedition is still called from at least 1200 Hz above, on a
  clear frequency there. A station calling us is answered where we are. Limits: it knows the signals
  it decoded, hears nothing in its own period while it transmits, and stays between 400 and 2600 Hz.
- **A spot of a garbled compound call is not hunted** (`5cfc68a1c`): "5T4LS/85PD6", hunted as an ATNO.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib auto` | 180/180 |
| `cargo test -p tempo-app --lib` (without `5cfc68a1c`) | 4109 passed, 9 failed: the 8 known ones, and `a_single_90_ms_stall_in_a_33_ms_poll_draws_every_sweep`, which passes alone (the PC was on the air) |
| Failing-first | shown for both offset tests (with the pick seeing no signals); not for the callsign |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| UI `tsc -b`; i18n, autopilot, feed and settings suites | pass; 840/840 |
| Not run | the commits one by one, tempo-audio, src-tauri tests and clippy, the whole UI suite, browser probe, CI (GitHub Actions billing is blocked) |
