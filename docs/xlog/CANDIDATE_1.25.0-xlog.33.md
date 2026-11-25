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
