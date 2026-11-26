# 1.25.0-xlog.35: the radio's late band change, and quiet DXpeditions

Source: `main` at the merge of `xnexus/autopilot-reliability` (`e70a54256`). Since 1.25.0-xlog.34
(`59239713a`), the branch adds four commits: `3964d8cdb`, `1e63fa61e`, `53564a3d7` and `364ca448d` (tests).
Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an installed
xlog.34 keeps its logbook and settings.

## What changed for the operator

- **The radio settling after the autopilot's own band change is no knob** (`3964d8cdb`; live run 2026-11-26
  02:49). For 90 s the old band read back is the change not taken yet, and it is sent again; its late arrival
  keeps transmit and the autopilot on. A third band, or any change after the 90 s, is still the operator's.
- **A needed station left unanswered round after round waits longer** (`1e63fa61e`): one minute after the
  first round, then two, four and so on, half an hour at most.
- **A DXpedition or ATNO not decoded for three minutes is no longer called** (`53564a3d7`, `364ca448d`), and is
  called again at once when heard. Its hunt goes on; this is not counted as a failed attempt.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all` | clean |
| `cargo test -p tempo-app --lib -- auto station_guard` | 211/211 |
| Failing-first | every mutation of the four commits fails a test |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| Not run with these commits | the whole tempo-app suite (8 known failures), src-tauri and UI (unchanged since xlog.34), tempo-audio, browser probe, CI (GitHub Actions billing is blocked) |

## Package and installation (2026-11-26)

- **Source:** `d95358f37`, canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.34 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed; Nexus, SmartSDR, CAT, DAX, xLog and Chrome closed. The Home Assistant VM
  was saved (not powered off) on the operator's word to free commit memory. Admission 20.5 GiB physical /
  16.4 GiB commit; 08:04-09:04 UTC; peak 5.44 GiB; lowest free 11.8 GiB physical / 6.9 GiB commit; cleanup
  confirmed.
- **Evidence:** `.build/xnexus-xlog35-package-7g/evidence/20261126T0804086730113Z-a47cf6f252554e81b75509bb54d4bf29`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.35.json` (source `d95358f37`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.35_x64-setup.exe`, 356,803,192 bytes, SHA-256
  `FB80A1C2F3DC817DD2DCA63A90DDF435448C2E4FA3668D77D983C3D0571E1189` (matches the receipt).
- **Built exe:** `D4CD2ACD...1EC7`. **Installed exe:** `FFB160CC...7F49`, version 1.25.0-xlog.35.
- **Install:** 09:05 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **Not launched** when this was written: going on the air needs the operator's approval.
