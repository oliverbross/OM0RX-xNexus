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

## Package and installation (2026-11-22)

- **Source:** `73bcd5b6e` (this reservation), canonical `main`, clean, equal to `origin/main`.
- **Guard:** profile `optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`, the xlog.21 wrapper
  with only the expected version changed.
  - Attempt 1 (`20261122T0932532730843Z-…`) was interrupted at 11:34 UTC, `host-commit-below-6GiB`.
    The operator's Home Assistant VM had been saved for the build with their approval, but its
    availability task restarted it at 09:34, one minute in. The guard stopped the build cleanly and
    nothing was retried inside that attempt.
  - Attempt 2 set the availability task's own `maintenance.pause` file and saved the VM again. It
    passed: admission 19.9 GiB physical / 15.9 GiB commit; 11:36–12:26 UTC; peak 5.44 GiB. Afterwards
    the pause file was removed and the VM started again.
- **Evidence:** `.build/xnexus-xlog22-package-7g/evidence/20261122T1136245185978Z-93b7e8bd5dcc43d0b9f6ce93c0cc4a79`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.22.json` (source `73bcd5b6e`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.22_x64-setup.exe`, 356,536,348 bytes, SHA-256
  `9AB71A4B8076796A34B7AE784B0ED9579F1B08EE242A0C94C1BC89BE21C2F47C`.
- **Built exe:** `AC6030CC…4255`. **Installed exe:** `9B65CCE5…4043`, version 1.25.0-xlog.22
  (bundler-stamped, as before).
- **Install:** Nexus was not running; silent `/S` install, exit 0; the uninstall entry reads 1.25.0-xlog.22.
- **Launch:** SmartSDR started first, and its CAT was listening on 5002. One Nexus instance started
  (12:28:52 UTC). The log reads `Nexus 1.25.0-xlog.22`, CAT connected through rigctld (model 2036),
  and DAX capture is delivering.
- **Live run:** TX on, then the autopilot on at 12:29 UTC.
  - It followed the schedule to 20 m and worked LA2JKA, M0WWE, UB1AUW, BA3KY (an Answer-callers
    caller), RU4LM, then UA3QQG and SP5UTI on 17 m.
  - Within its tier it called the distant BA7SAY (-19 dB) first, and moved on after the
    120 s budget.
  - No station was given a second report, and the TX watchdog did not stop it.
  - One defect found: the ordinary-dwell move came 14 minutes in, not 10, because a held move let
    another ordinary call start. It is fixed on `xnexus/autopilot-reliability` (`4d5b7d016`) for the
    next candidate.