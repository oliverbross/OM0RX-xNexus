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

Also open: a station that answered once and then works others is still sent the report until the
sequencer's own limit (SQ2MMX, 2026-11-23). The four-call limit is for a station that never answered.

## Package and installation (2026-11-23)

- **Source:** `8bdffe9fa` (this reservation), canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.24 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed. Passed on the first attempt: admission 18.7 GiB physical /
  13.5 GiB commit; 07:06–09:39 UTC; peak 5.44 GiB; cleanup confirmed. The Home Assistant VM had been
  saved with the operator's approval and its availability task paused; afterwards the VM was started
  and the pause file removed.
- **Evidence:** `.build/xnexus-xlog25-package-7g/evidence/20261123T0706371333222Z-8bf07d4f0c994c7f864eea9b346689c9`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.25.json` (source `8bdffe9fa`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.25_x64-setup.exe`, 356,546,642 bytes, SHA-256
  `B7F0A00266605EDB4196DBEB4BA56764F8286CEA2B16C68AC218C13A9D6F483E`.
- **Built exe:** `477AE555…AD81`. **Installed exe:** `0B8B928C…47DB`, version 1.25.0-xlog.25.
- **Install:** 09:42 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **Launch:** 10:20:43 UTC, after SmartSDR, CAT and DAX were started with the operator's go-ahead.
  CAT connected on the first open (10:20:50).
- **Live run:** TX and the autopilot on from 10:21 UTC. First hour, to 11:22 UTC:
  - 14 contacts: BG5IOG, TA4INO, TA2TDW, F1CCQ (20 m); RU3GF (17 m); VU2KPH, OE2VEL, R8LCA (12 m);
    R9WKU, RA1NBG, YH8MJ, VU2NRO, TA3LFT, HZ1YJ (15 m). 70 key-downs, 70 releases logged. No error,
    no watchdog stop, no unplanned disarm.
  - **Seen working:** a caller answered after one unanswered call (RU3GF, with "A caller takes over"
    at 0); the four-call limit (Z62NS, EW7EL, SH2PMO); the answer to the fourth call taken (OE2VEL,
    YH8MJ); a late answer taken (F1CCQ); a station called again later (Z62NS after 11 minutes);
    the rotation going on from 17 m to 15 m after a chase to 12 m.
  - **Not seen:** 30 m (night schedule only), a DXpedition, an ATNO, a switched-off continent.
  - **Defects found, not fixed in this candidate:**
    - VU2NRO called at 11:07:45, after R3WF was picked (11:07:32) and before the first call to it
      (11:08:00). Only a caller heard since our last over counts, so R3WF was called. VU2NRO was
      answered at 11:08:30 without an autopilot log line, so probably by the operator.
    - F1CCQ called us once, then called DL7JU and worked IU0SKX, and got three reports meanwhile
      (TA2TDW and LZ2MP were waiting). Same gap as SQ2MMX.
    - The log says "answering caller F1CCQ (4 of 3)": the chain limit does not apply with the
      autopilot on, so the count should not be shown as "of 3".
  - xLog follows the call Nexus works (VU2NRO, TA3LFT and EW7EL seen), but its rotor did not turn:
    Auto rotor was off in xLog.
- **Live run, to the end (11:22–12:29 UTC):** 7 more contacts: SV7LYG (12 m), EP2AES (10 m, a needed
  DXCC slot), R5VCV, YC8BUL, VU2NRO (20 m), VK9XY (12 m FT4) and CT2KCK (20 m FT4, by hand).
  - The rotation visited 20, 17, 15, 12 and 10 m. No watchdog stop; the autopilot stopped only when
    the operator changed the mode by hand (FT2 at 12:26:49, FT4 at 12:27:50). Clean close at 12:29:43.
  - FT2 dropped decode periods (12:26:52–12:27:48), with a test build and xLog's worker (over two
    cores) running on the same machine.
  - **Defect found, not fixed in this candidate:** VK9XY was hunted on 12 m FT4 from 11:54:26 and heard
    at 978 Hz, +3 to +5 dB, but never called by the autopilot: below 1000 Hz it called only with Hound
    on, which is FT8's. The operator called it by a double-click (12:08:22, worked 12:10).
