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
