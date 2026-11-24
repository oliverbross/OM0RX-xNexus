# 1.25.0-xlog.28: what the xlog.27 live run found

Source: `main` at `6c4b7efad`, which merges `xnexus/autopilot-reliability` (`7e330a19a`). Since
1.25.0-xlog.27 (`5821abf5a`), the branch adds two commits: `b146e4846`, `7e330a19a`.
Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an
installed xlog.27 keeps its logbook and settings.

## What changed for the operator

- **A needed station is called when it ends a contact** (`b146e4846`; XE2X on 80 m went from one caller
  to the next with 73 and RR73 and no CQ). A needed station is anything but a grid, an ordinary new
  call or a caller to us; those still wait for a CQ or a call.
- **An unanswered call for a grid or an ordinary new call gives way to a needed station** (`7e330a19a`;
  XE2X on 160 m called CQ beside the calling of MW7TTA). A station that has answered is worked to the
  end.

Not changed: a low-confidence decode (marked `^`) is never acted on, XE2X's 160 m CQs among them.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib auto` | 168/168 |
| `cargo test -p tempo-app --lib` | 4098 passed, 8 failed, all of which fail without the changes too |
| Failing-first | shown for both changes |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| Not run | tempo-audio and src-tauri (no change there but the version), the UI suites (no UI change), browser probe, CI (GitHub Actions billing is blocked) |

## Package and installation (2026-11-24)

- **Source:** `e16246c43`, canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.27 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed. It waited from 06:18 UTC for commit memory (11.76 GiB free)
  until the operator closed SmartSDR, CAT and DAX and had xLog closed. Admission 17.0 GiB physical /
  13.1 GiB commit; 06:25–07:21 UTC; peak 5.44 GiB; lowest free 12.7 GiB physical / 7.9 GiB commit;
  cleanup confirmed. The Home Assistant VM was not saved.
- **Evidence:** `.build/xnexus-xlog28-package-7g/evidence/20261124T0625187092032Z-a46944555eab44678da534ce63231028`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.28.json` (source `e16246c43`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.28_x64-setup.exe`, 356,576,984 bytes, SHA-256
  `3E3F45FDE65010C25BA4C005C2A5678E1A56CF0A808227FF7C2135FFB7ABDD0C` (matches the receipt).
- **Built exe:** `C5021264…4C57`. **Installed exe:** `81E7A045…FDF5`, version 1.25.0-xlog.28.
- **Install:** 07:21 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **First launch:** 2026-11-24 07:23:56 UTC, with the operator's approval to go on the air.

## Live run (2026-11-24 07:24 to 13:15 UTC)

Day schedule (FT8 20/17/15/12/10 m; the operator ran an FT4 window from 10:47 to 11:30), FLEX-8400
through SmartSDR CAT. xLog showed the amplifier in Standby.

- **Contacts:** 28 (15 m 8, 20 m 8 of them 2 in FT4, 12 m 6, 17 m 6), among them the needed slot TR8CA
  (12 m) and the needed references G5USH (20 m) and G0SBN (15 m). RC0JD, CX7ACH and G5USH answered late
  and were taken up; HB9ITZ/P and 4U1ITU were called as they ended a contact (`b146e4846`).
- **Transmit:** 214 overs keyed and unkeyed, longest 13.7 s. One decode period dropped (12:44:15).
- **The program crashed at 12:32:46** (0xC0000409, ucrtbase.dll, an invalid argument to `setlocale`
  called by libgfortran after the internal WRITE of `chkcrc14a` in the JTDX-style FT8 decoder). Dump
  kept in `.build/crash-20261124` of the xLog checkout. Restarted at 12:33:39; CAT answered after
  SmartSDR CAT was restarted; autopilot on again at 12:37:20. `cb815e653`.
- **The TX watchdog disarmed transmit at 08:10:22** and the autopilot went off: the contact with XU7O,
  stalled by the sequencer's call cap, was left armed. On again at 08:11:21. `5ee7f0c26`.
- **DXpedition XU7O:** on 17 m FT4 left after five minutes as not heard while up to five stations a
  period called it (`c87dfcc8d`). On 12 m FT4 (08:01) heard in ten seconds at 406 Hz and called at
  1606 Hz; its report came, and its RR73 as a low-confidence decode, which nothing automatic acts on,
  so the contact was not logged. The held 1375 Hz offset then stayed at 1606 Hz (`541e5a762`).
- **15 m held the autopilot for half an hour** with new grids only (`353ccc88e`). **FT4 spots were
  hunted for a new mode alone** and then counted as confirmed (OM8AKP, ES2TT, ES6DO: `e2b25fdb5`).

All fixed on `xnexus/autopilot-reliability`, not in this candidate.
