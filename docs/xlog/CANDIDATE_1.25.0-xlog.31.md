# 1.25.0-xlog.31: every spotted DXpedition hunted, for the minutes the operator sets

Source: `main` at `6d3a5d238`, which merges `xnexus/autopilot-reliability` (`5b7ba3085`). Since
1.25.0-xlog.30 (`dac3c61de`, installed, never on the air), the branch adds one commit: `5b7ba3085`.
Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an
installed xlog.30 keeps its logbook and settings. The saved time budget in seconds (900) is no longer
read: the new setting starts at ten minutes. An older version ignores the new one.

## What changed for the operator

- **Every spotted DXpedition is hunted** (operator request 2026-11-25): its FT8, FT4 and FT2 spots, when
  that call is not worked on that band in digital, whatever its entity has there, and heard by a station
  on the operator's continent. VK9XY was spotted on nine bands on 24 November and hunted on none.
  The newest spot first; each band and mode with its own cool-down; five minutes of nothing decoded ends
  a hunt and rests the call for three. An unanswered call gives way; an answered one is finished. One
  heard under 1000 Hz on the band (FT8) is hunted there.
- **Call a DXpedition for (minutes)**, Settings, Autopilot: 1 to 180, ten by default, from when it or its
  callers are first heard; also for an ATNO and a Club Log top-50 station.
- With xlog.30's clear transmit frequency and spot filter.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check`, src-tauri too | pass |
| `cargo test -p tempo-app --lib auto` | 185/185 |
| `cargo test -p tempo-app --lib` | 4115 passed, 8 failed, the known ones |
| Failing-first | six mutations (no-need spots, cool-down by call, unanswered call kept, not in place, oldest first, time from the start): each fails a new test |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| src-tauri `cargo check --features radio`; its new spot feed test | pass; pass (the test program given a common-controls manifest, without which it does not start here) |
| UI `tsc -b`; i18n, autopilot, feed and settings suites | pass; 841/841 |
| Not run | the rest of the src-tauri tests and its clippy, tempo-audio, the whole UI suite, browser probe, CI (GitHub Actions billing is blocked) |
