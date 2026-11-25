# 1.25.0-xlog.34: countries before callers, and silent bands left

Source: `main` at the merge of `xnexus/autopilot-reliability` (`aa7c1ab77`). Since 1.25.0-xlog.33
(`cdc113b5d`), the branch adds five commits: `a7826088f`, `a7b435220` (tests), `bc2879a0b`, `5e1c1c439` and
`aa7c1ab77`. Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an
installed xlog.33 keeps its logbook and settings.

## What changed for the operator

- **A country never confirmed has its calls before a caller is answered** (`a7826088f`; operator, 2026-11-25: "new or
  unconfirmed country has priority, band slot not").
- **An unanswered needed station gives way to one ranked above it,** and one left for a better station or for a
  caller is called again after it, its chase going on.
- **A band on which nothing at all is decoded is left after two minutes** (`bc2879a0b`) and rests half an hour while
  another band of the schedule is open (`5e1c1c439`). A spot chase there ends too, except for a country never
  worked; states and activations are not chased to it meanwhile. DXpedition hunts keep their time.
- **A rogered report from a station we gave none is no call** (`aa7c1ab77`): not answered, not logged.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all` | clean |
| `cargo test -p tempo-app --lib -- auto station_guard` | 207/207 |
| Failing-first | 17 mutations, each fails a test in the end; three first passed and the tests were tightened (`a7b435220`) |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| Not run with these commits | the whole tempo-app suite (8 known failures), src-tauri and UI (unchanged since xlog.33), tempo-audio, browser probe, CI (GitHub Actions billing is blocked) |

## Package and installation (2026-11-25)

- **Source:** `59239713a`, canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.33 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed; Nexus, SmartSDR, CAT, DAX, xLog and Chrome closed.
  Admission 16.7 GiB physical / 12.8 GiB commit; 22:26-23:25 UTC; peak 5.44 GiB; lowest free 12.5 GiB
  physical / 7.4 GiB commit; cleanup confirmed. The Home Assistant VM was not saved.
- **Evidence:** `.build/xnexus-xlog34-package-7g/evidence/20261125T2226573718069Z-0631c1ebc0e5410896843e83ef0e372e`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.34.json` (source `59239713a`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.34_x64-setup.exe`, 356,800,442 bytes, SHA-256
  `702671D58F7EEDC44878931D50D8795F92F988D381F0805C3475732060F38BA7` (matches the receipt).
- **Built exe:** `D56EACD5...217A`. **Installed exe:** `7D7F0388...1CEB`, version 1.25.0-xlog.34.
- **Install:** 23:26 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **Not launched** when this was written: going on the air needs the operator's approval.
