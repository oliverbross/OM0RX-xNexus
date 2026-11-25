# 1.25.0-xlog.33: the tuner and amplifier guard, and the beam before the DX

Source: `main` at the merge of `xnexus/autopilot-reliability` (`93781ec83`). Since 1.25.0-xlog.32
(`7d61bc66d`), the branch adds seven commits: `d7a7e97df`, `791a340ae`, `69e0deff6`, `8cfccb0a5`,
`1b69d56ef`, `21b22433e` and `93781ec83`. Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2,
`21f743404`). Identity unchanged, so an installed xlog.32 keeps its logbook and settings.

## What changed for the operator

- **A KAT500 or KPA500 fault pauses the autopilot and stops transmit at once** (`21b22433e`, `93781ec83`). The
  OM0RX Station Agent on this PC is asked while the autopilot runs; its token is read from its own settings
  file each time and sent to 127.0.0.1 only. A second switch, off until the operator turns it on, has the Agent
  clear the fault, tune and align, each step after the one before, and only then transmit comes back. Once
  per fault; the operator's own TX switch, Stop TX or a clicked station always ends it. The amplifier is left
  as the Agent leaves it.
- **The station listened for is reported to loggers as the DX call** (`69e0deff6`), so OM0RX-xLog can turn the
  antenna first; never with a hash's brackets.
- **DXpeditions are tuned under 500 Hz and called from 1000 to 2500 Hz** (`8cfccb0a5`), inside their receiver.
- **A Hound contact ended leaves the autopilot on** (`1b69d56ef`).
- **Switched on, the autopilot goes to the band's channel first** (`d7a7e97df`).
- **No call is started and dropped again while a spotted DXpedition waits** (`791a340ae`).

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check`, src-tauri fmt | pass |
| `cargo test -p tempo-app --lib -- auto station_guard` | 200/200 |
| Failing-first | ten mutations of the guard and the Hound ending: each fails a test; the missing hold release on a clicked station was found by it |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| `cargo check` src-tauri `--features radio` | pass (with `21b22433e`; unchanged since) |
| src-tauri `station_agent` test | 1/1 |
| `tsc -b`, vitest (i18n, autopilot, Settings) | pass, 841/841 |
| Not run | the recovery against the real tuner and amplifier, the whole tempo-app suite (8 known failures), tempo-audio (unchanged), browser probe, CI (GitHub Actions billing is blocked) |

## Package and installation (2026-11-25)

- **Source:** `cdc113b5d`, canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.32 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed; Nexus, SmartSDR, CAT, DAX, xLog and Chrome closed.
  Admission 17.0 GiB physical / 12.9 GiB commit; 12:23-13:20 UTC; peak 5.44 GiB; lowest free 12.6 GiB
  physical / 7.5 GiB commit; cleanup confirmed. The Home Assistant VM was not saved.
- **Evidence:** `.build/xnexus-xlog33-package-7g/evidence/20261125T1223106152054Z-f7f7c9f1813442d88ea6216c18e9261d`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.33.json` (source `cdc113b5d`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.33_x64-setup.exe`, 356,600,998 bytes, SHA-256
  `00C26A227E8FFE35D79326C03C05A32A6557AB2C59A6160C38E60DC0C93BF728` (matches the receipt).
- **Built exe:** `9B78E844...7397`. **Installed exe:** `9110CC61...CE3C`, version 1.25.0-xlog.33.
- **Install:** 13:21 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **Not launched** when this was written: going on the air needs the operator's approval.
