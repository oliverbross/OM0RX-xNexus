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

## Package and installation (2026-11-25)

- **Source:** `acce57abd`, canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.30 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed; Nexus, SmartSDR, CAT, DAX and xLog closed.
  Admission 16.7 GiB physical / 13.1 GiB commit; 02:51-03:48 UTC; peak 5.45 GiB; lowest free 12.1 GiB
  physical / 7.7 GiB commit; cleanup confirmed. The Home Assistant VM was not saved.
- **Evidence:** `.build/xnexus-xlog31-package-7g/evidence/20261125T0251362573566Z-4c4fbd911e5c47c8823f7b530fd267ab`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.31.json` (source `acce57abd`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.31_x64-setup.exe`, 356,590,092 bytes, SHA-256
  `3B4D96493E280E82EDC3A5B71438A37ADC53B59F0AFFE705BE36827EDD1AFAA5` (matches the receipt).
- **Built exe:** `83D3255E…71E7`. **Installed exe:** `3FDA1F2E…1595`, version 1.25.0-xlog.31.
- **Install:** 03:49 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **Not launched** when this was written: going on the air needs the operator's approval.
