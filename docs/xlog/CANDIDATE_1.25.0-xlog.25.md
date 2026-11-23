# 1.25.0-xlog.25: operator rules from the xlog.24 live run

Source: `main` at `9dcaa73ec`, which merges `xnexus/autopilot-reliability` (`80a88fd3b`). Since
1.25.0-xlog.24 (`17177d04b`), the branch adds two commits, `3ee505282` and `80a88fd3b`. Upstream base
unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an installed xlog.24
keeps its logbook and settings.

## What changed for the operator

Asked for during the live run of xlog.24 (operator OM0RX, 2026-11-23), with the autopilot on:

- **A station calling us is answered at once when no contact is under way.** N9ATF called while
  DO8JB, double-clicked by the operator, left nine calls unanswered, and was not answered ("A caller
  takes over" was set to 0). A caller now takes over from an unanswered call whatever that setting
  says, also in the listening slots and after a station was given up. Never from an ATNO, a
  DXpedition, a top-50 station, or a station that has already addressed us.
- **Four unanswered calls, then the next station**, the operator's double-click included. The answer
  to the last call is waited for, and a late answer is taken. Settings ▸ Autopilot ▸ Timing has the
  number; the autopilot's line shows the count. Rare stations keep their own time budget.
- **Continents to call:** a checkbox for each in Settings ▸ Autopilot. An ATNO is called anywhere; a
  station calling us is always answered.
- **Every scheduled band gets its turn.** A chase restarted the rotation from the spot's band, so
  from 02:33 to 05:03 UTC 30 m was never reached (80 m 13 times, 160 m 10). The rotation goes on from
  the band the chase left, and a DXCC-slot spot waits five minutes after an unheard chase.

Found by watching xlog.24 and xlog.23:

- **A garbled callsign is not chased** (E34XQXUYE9H, 89EARO/J8Z5).
- **PSK Reporter's dot is a slash:** KH6XX.W0 is KH6XX/W0, in the US, not a Hawaiian slot.

## Not changed, noted

- **No alarm when CAT is lost around an over.** On 2026-11-23 SmartSDR CAT and DAX closed during an
  over; Nexus recorded the unkey as done, and the radio stayed keyed for about a minute. See
  CANDIDATE_1.25.0-xlog.24.md.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib auto` | 151/151 |
| `cargo test -p tempo-app --lib` (before a final clippy-only edit) | 4081 passed, 8 failed; the same 8 fail with the change reverted (`upload_start` ×4, `remote_logging`, `fd_scoreboard`, `logstore`, `settings` one each) |
| `cargo test -p propagation --lib` | 802/802 |
| Failing-first | shown for the garbled call and the PSK Reporter slash. The other new tests were not run against the old code |
| `cargo clippy -p tempo-app --lib -D warnings` | pass |
| `tsc -b` | pass |
| Vitest | AutopilotControl, stale copy and i18n files: 814 of 815 passed, the one a 5 s timeout under load, and two files did not start; those passed on a re-run (416/416 in four files) |
| Not run | src-tauri check, the whole UI suite, the browser probe, CI (GitHub Actions billing is blocked) |
