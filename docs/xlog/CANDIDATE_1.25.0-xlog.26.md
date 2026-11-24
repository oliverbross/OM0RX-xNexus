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

## Package and installation (2026-11-23)

- **Source:** `d089512d1` (the reservation `3eae65321` plus a correction to `UPSTREAM.json`), canonical
  `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.25 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed. The first attempt (13:01 UTC) was held at admission, 12.47 GiB
  commit of the 12.5 required; nothing was built. Second attempt passed: admission 19.5 GiB physical /
  13.8 GiB commit (after the operator closed Docker); 13:04–13:59 UTC; peak 5.44 GiB; cleanup confirmed.
  The Home Assistant VM had been saved with the operator's approval and its availability task paused;
  afterwards the VM was started and the pause file removed.
- **Evidence:** `.build/xnexus-xlog26-package-7g/evidence/20261123T1304436137617Z-dd781f0e99f44d8094418f3b3a39a529`
  (held attempt: `20261123T1301198675929Z-c0c2dc6f37084f3d95dbc0efb658cdc5`).
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.26.json` (source `d089512d1`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.26_x64-setup.exe`, 356,568,561 bytes, SHA-256
  `BEE6426AA06A16EA0CA22C8D133B733B43B89BE06A19F5225ABC9C5927989A51`.
- **Built exe:** `A166701D…8482`. **Installed exe:** `83B99DC5…4D58`, version 1.25.0-xlog.26.
- **Install:** 14:00 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **First launch:** 2026-11-23 23:00:57 UTC, with the operator's approval to go on the air.

## Live run (2026-11-23 23:02 to 2026-11-24 02:21 UTC)

Night schedule, FLEX-8400 through SmartSDR CAT, amplifier in standby, transmit offset held at 1375 Hz.

- **Reliability:** 3 h 19 min, closed cleanly by the operator's order at 02:21:24; no stop of the autopilot, no transmit disarm, no watchdog stop, no
  PTT alarm, CAT connected throughout, longest over 13.7 s.
- **Contacts:** 21 (60 m 13, 40 m 5, 80 m 2, 160 m 1). Needed band slots ER3ZW, ER1CW (60 m) and 3B8FA
  (40 m); spotted US state K8MM worked on the second hunt. G0KLD, TA4SSK and N3FWE called while
  another station had not answered, and were worked.
- **Decode periods dropped:** 11, between 23:33 and 00:11, while test builds ran on the same machine.
  None after the builds were confined to three cores.
- **Not seen:** a DXpedition, an ATNO, a lost CAT link, FT4.

Defects found, fixed on `xnexus/autopilot-reliability`, not in this candidate:

- **8Q7XX (80 m, needed) left four calls unanswered and called CQ beside an idle autopilot**, whose
  hunt ended as "not heard" (23:20–23:25). `598ae431a`.
- **D2ACE answered late, sent its report three times, and was dropped after four R-reports** (23:30–23:32).
  `1d223d965`.
- **30 m came up once, for 75 s, and 160 m had 40 s in the first 74 minutes**: after chases 40 m counted
  as due again. `05cf0d4fd`. A chase still leaves a scheduled band at once.

Observed, not changed: on 40 m, 50 of 53 periods had a signal overlapping the held 1375 Hz offset, and
most calls there went unanswered.
