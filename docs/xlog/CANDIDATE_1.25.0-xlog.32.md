# 1.25.0-xlog.32: DXpeditions called inside their receiver

Source: `main` at the merge of `xnexus/autopilot-reliability` (`72433730b`). Since 1.25.0-xlog.31
(`acce57abd`), the branch adds one commit: `72433730b`. Upstream base unchanged (kd9taw/Nexus
v1.25.0-beta.2, `21f743404`). Identity unchanged, so an installed xlog.31 keeps its logbook and settings.

## What changed for the operator

- **A DXpedition is called inside its own receiver** (`72433730b`; operator, 2026-11-25: "we can't go above
  the filter bandwidth"): 1200 Hz above only while that stays under 2900 Hz; one higher is called on a clear
  frequency at least 1200 Hz below it, the dial left alone.
- **Our own retune is no sign of a Fox.**
- **A station picked and not called yet is no failure** when a needed one takes its place.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib auto` | 187/187 |
| Failing-first | two mutations (above only, Fox by our dial): each fails a new test; not shown for the uncalled station |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| Not run with this commit | the whole tempo-app suite (4115 passed and the 8 known failures without it), src-tauri, UI (both unchanged since xlog.31), tempo-audio, browser probe, CI (GitHub Actions billing is blocked) |
