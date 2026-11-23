# 1.25.0-xlog.26: what the xlog.25 live run found, and the stuck-key alarm

Source: `main` at `1b2436956`, which merges `xnexus/autopilot-reliability` (`96155e064`). Since
1.25.0-xlog.25 (`8bdffe9fa`), the branch adds three commits: `70979e548`, `fdecba989`, `96155e064`.
Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an
installed xlog.25 keeps its logbook and settings.

## What changed for the operator

- **A caller is answered even before the first call to a station just picked** (`70979e548`). VU2NRO
  called between the pick of R3WF and the first call to it, and R3WF was called.
- **A station that answered and then works others gets four overs, not eight** (`70979e548`), and a
  caller is answered first; if it comes back, the contact goes on (SQ2MMX, F1CCQ).
- **A DXpedition below 1000 Hz is called without Hound** (`fdecba989`). In FT4, which has no Hound, it
  was only listened to (VK9XY, 12 m FT4, 14 minutes). It is brought under 500 Hz and called from
  1200 Hz above, as the operator's rule says. FT8 Fox/Hound is unchanged.
- **A radio left keyed by a lost CAT link raises the alarm** (`96155e064`, operator approval
  2026-11-23). An accepted unkey of an FT8/FT4 over is proved only by a dial reading at least 1.5 s
  after it. If CAT is declared down first, TX halts with PTT OFF FAILED — TX STOPPED, the unkey is
  sent again at each missed reading, and once more when CAT is back. One missed reading stops nothing.
  Not covered: voice, CW, Tune and other non-slot overs; a CAT program that answers readings while
  it loses the unkey.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib auto` | 157/157 |
| `cargo test -p tempo-app --lib` (at `70979e548`) | 4083 passed, 11 failed: the 8 that fail without the change, and 3 timing tests that pass alone |
| `cargo test -p tempo-audio --lib --features device,serial` | 2734 passed, 6 failed: 4 fail the same without the change; `a_session_lost_mid_over…` and `a_stalled_meter_read…` pass alone (3 of 3) |
| `failed_unkey` tests | 18/18 |
| Failing-first | shown for the caller before the first call, the station working others (two tests), the DXpedition below 1000 Hz, the lost link, the missed reading |
| `cargo clippy` tempo-app, tempo-audio (device,serial) `-D warnings` | pass |
| Not run | src-tauri check (no src-tauri change), the UI suites (no UI change), browser probe, CI (GitHub Actions billing is blocked) |
