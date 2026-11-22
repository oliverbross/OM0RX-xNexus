# 1.25.0-xlog.22: a reliable autopilot, and upstream v1.25.0-beta.2

Source: `main` at `4dc6eb41e`, which merges `xnexus/autopilot-reliability` (`dac98bdbe`). Since
1.25.0-xlog.21 (`110a003df`) main also gained:

- upstream kd9taw/Nexus v1.25.0-beta.2 (`21f743404`, merged in `8e886e30d`);
- review fixes N-01, N-02 and N-03 (`ec7832ff0`, `50fd60096`, `70b16f4c1`; see `REVIEW.md`);
- the fork's keychain test fix (`62807a1b2`).

Identity unchanged, so an installed xlog.21 keeps its logbook and settings. `UPSTREAM.json` now
records beta.2 as the upstream base.

## What changed for the operator (autopilot and Answer callers)

The field report of 2026-11-22 (operator OM0RX):

- **No second report to a station already worked.** SP4NKJ called while we finished EW4M, so
  Answer callers queued it. We then worked SP4NKJ, and once our RR73 was out the stale entry
  answered it again with "SP4NKJ OM0RX -13" (also SQ2BXI and YO4BXX in the log). A station whose
  contact is made is now kept out of the queue and out of the autopilot's pick for ten minutes. A
  caller's 73, RR73 or RRR withdraws its older call, and the station being worked is never
  counted as a caller.
- **The TX watchdog no longer switches the autopilot off on its own.** While the autopilot runs,
  each contact it keeps restarts the watchdog clock. Those contacts are bounded instead by their
  own time budget, the call cap, the hourly cap and the presence check. The watchdog still trips
  when the autopilot's step stops running, and confirming presence restarts it.
- **A DXpedition or ATNO is called for at least 10 minutes.** The rare budget's floor is now 600 s.
- **DX first.** Within one priority tier, a station 3000 km or more away (from the grid it sent)
  is called before a louder station nearby.
- **Moving on.** A band that has offered only ordinary new calls for 10 minutes gives way to the
  next band in the schedule, between contacts.
- **Smaller fixes:**
  - The listening slots count whole slots, not decode passes (N-06).
  - A contact complete on the air counts as worked even when it is not logged (N-04).
  - Switching the autopilot off in Settings stops a running autopilot (N-05).
  - Hound is switched on only in FT8 (N-11a).
  - A hunt that never heard its DXpedition waits 30 minutes before it hunts it again.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib autopilot` / `automation::tests` | 29/29, 21/21 (new: repeat report, watchdog keeps and still trips, 10-minute floor, DX ranking, locator distance) |
| Failing-first | without the fix, the repeat test fails with `"K1ABC OM0RX -05"` |
| `cargo test -p tempo-app --lib` | 4053 passed, 10 failed. 6 of the 10 are the review's known baseline failures on this machine (fd_scoreboard, logstore, 4 upload_start). The other 3 shown in the output (two sweep-timing tests and `radio_claim` peek) pass when run alone, so they failed under load. |
| `cargo clippy -p tempo-app --lib --tests -D warnings` | one error: the existing `logstore::lane_of` dead code, which was already on main before this change |
| Not run | src-tauri tests, tempo-audio full, UI suites, CI (GitHub Actions billing is blocked) |
