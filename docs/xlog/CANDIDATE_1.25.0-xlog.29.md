# 1.25.0-xlog.29: FT8 and FT4 together, and what the xlog.28 live run found

Source: `main` at `4b6bafe54`, which merges `xnexus/autopilot-reliability` (`cb815e653`). Since
1.25.0-xlog.28 (`e16246c43`), the branch adds seven commits: `c87dfcc8d`, `5ee7f0c26`, `541e5a762`,
`353ccc88e`, `e2b25fdb5`, `8bcffc8ea`, `cb815e653`.
Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an
installed xlog.28 keeps its logbook and settings. An older version drops a schedule window set to
FT8 + FT4.

## What changed for the operator

- **The FT8 decoder no longer ends the program** (`cb815e653`, Windows): the executable wraps
  `setlocale`, so the Fortran runtime hands no freed text to the C runtime.
- **FT8 and FT4 at the same time** (`8bcffc8ea`, operator request 2026-11-24): a window can be FT8 + FT4,
  and windows with the same start run together. Every band has its turn in each mode, and needed spots
  in either are chased. A later start still takes over alone.
- **The TX watchdog no longer stops the autopilot over a contact that went quiet** (`5ee7f0c26`): the
  autopilot ends a stalled contact after two minutes. The watchdog is unchanged.
- **A DXpedition that others are heard calling is followed for ten minutes** and hunted again each hour
  (`c87dfcc8d`). **The transmit offset goes back where it was** after one (`541e5a762`).
- **A band with only new grids gives way after ten minutes** (`353ccc88e`).
- **A spot is not hunted for a new mode alone** (`e2b25fdb5`); a DXpedition still is.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib auto` | 177/177 |
| `cargo test -p tempo-app --lib` | 4107 passed, 8 failed, all of which fail without the changes too |
| Failing-first | shown for the ten minutes, the hourly hunt, the stalled contact, the offset and the new mode; not for the grids dwell and the two-mode patrol |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| `setlocale` wrap | test programs: 7,880 garbage texts in 20 s without, none with; the Rust file built and run with the release toolchain; not yet in Nexus itself |
| UI `tsc -b`; i18n, autopilot and feed suites | pass; 773/773 |
| Not run | the commits one by one, tempo-audio, src-tauri tests and clippy, the whole UI suite, browser probe, CI (GitHub Actions billing is blocked) |

## Package and installation (2026-11-24)

- **Source:** `a6c881614`, canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.28 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed. SmartSDR, CAT, DAX and xLog were closed normally first.
  Admission 17.5 GiB physical / 13.5 GiB commit; 13:18-14:16 UTC; peak 5.44 GiB; lowest free 12.7 GiB
  physical / 7.8 GiB commit; cleanup confirmed. The Home Assistant VM was not saved.
- **Evidence:** `.build/xnexus-xlog29-package-7g/evidence/20261124T1318020863523Z-b4998ca9f18e497a8ee2ee9b8ba7fc86`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.29.json` (source `a6c881614`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.29_x64-setup.exe`, 356,580,652 bytes, SHA-256
  `37A29A754D7F651D26F3286CB4AE1CB72B9E56AB2F4A3DF67E2696C9A9876094` (matches the receipt).
- **The `setlocale` wrap is in the built exe** (`objdump`): the three calls in the Fortran runtime's range,
  the one in `finalize_transfer` that crashed among them, go to `__wrap_setlocale`.
- **Built exe:** `392EC0B9…B20C`. **Installed exe:** `311C06AA…F94F`, version 1.25.0-xlog.29.
- **Install:** 14:17 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **First launch:** 2026-11-24 21:07:55 UTC, with the operator's approval to go on the air.

## Live run (2026-11-24 21:17 to 22:17 UTC, the first hour)

Night schedule (FT8 160/80/60/40/30 m), FLEX-8400 through SmartSDR CAT, xLog running from 21:33.

- **No RF from 21:10 to 21:14.** The DAX program passed no audio to the radio although Nexus played into
  the DAX TX device at full level (Windows peak meter 0.90, device active and not muted). The radio was
  keyed for the calls without output; the operator noticed it. Restarting DAX, with Nexus closed,
  fixed it (SmartSDR and CAT were restarted in the same minute); the cause in DAX is not known. Nexus
  has no meter readings over this CAT connection, so it cannot see it. Counted from 21:17:29.
- **Contacts:** 7 (40 m 5, 80 m 2): YB7YGR, SV7LWI, R0TA and YC8BUL called for a new grid, one late answer
  (BH9FPK, taken up after the four calls) and two callers answered (BA3QIQ; JA1EYN, during the contact
  with YC8BUL). No needed slot was worked: Z62NS, 8Q7XX, KP4AA and JT1BZ (twice) did not answer;
  XX9TXN was hunted three times and not heard.
- **Transmit:** 72 overs keyed and unkeyed, longest 13.7 s. No TX watchdog trip, no autopilot switch-off,
  no crash, no dropped decode period logged.
- **`353ccc88e` seen:** 40 m to 30 m at 21:33:11 and 80 m to 160 m at 22:00:48, ten minutes after the last
  call that was not only for a grid. A grid gave way to a needed station (BA7IWL to Z62NS, JH4ALY to
  8Q7XX).
- **Contacts made while xLog was closed** do not reach it (they go out once, by UDP). Compared: 207 contacts
  since 20 November, four missing in xLog (SO9DXX, BH9FPK, BA3QIQ, and SV5BYP of 20 November, complete in
  ALL.TXT). Sent again as WSJT-X LoggedADIF; no duplicates, synced to Wavelog.
- **A false decode** "OM0RX JT1BZ RR73" (22:10:00, no report had been exchanged) was ignored.
- **Fixed offset:** DK2WH called JT1BZ on 1374 Hz, ours is 1375 Hz, and was answered.

Not seen yet: the FT8 + FT4 patrol, the stalled-contact end (`5ee7f0c26`), a DXpedition, and a run as long
as the one that crashed xlog.28 (five hours).

### The whole run (to 2026-11-25 00:41 UTC, stopped for the xlog.30 build)

- 3 h 24 min, 24 contacts (40 m 10, 60 m 7, 160 m 3, 30 m 2, 80 m 2), all in xLog and synced. 228 overs
  keyed and unkeyed, longest 13.7 s. No crash, TX watchdog trip or autopilot switch-off. Shorter than the
  five hours after which xlog.28 crashed, so the `setlocale` wrap is not proven over that time.
- **"5T4LS/85PD6" was hunted as an ATNO** (a false decode somebody reported): `5cfc68a1c`.
- **JT1BZ** answered on 40 m (-21), did not copy our roger twice and went on to others: left.
