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
