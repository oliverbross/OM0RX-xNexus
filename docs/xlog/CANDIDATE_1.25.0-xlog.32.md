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

## Package and installation (2026-11-25)

- **Source:** `7d61bc66d`, canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.31 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed; Nexus, SmartSDR, CAT, DAX and xLog closed.
  Admission 17.1 GiB physical / 13.5 GiB commit; 06:22-07:20 UTC; peak 5.45 GiB; lowest free 12.4 GiB
  physical / 7.8 GiB commit; cleanup confirmed. The Home Assistant VM was not saved.
- **Evidence:** `.build/xnexus-xlog32-package-7g/evidence/20261125T0622522439638Z-605eb71a67d0424cad7729ed315dfa9b`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.32.json` (source `7d61bc66d`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.32_x64-setup.exe`, 356,581,308 bytes, SHA-256
  `7796ADBAA972A85143143F6C2A14E22A4C9C2E5175090B0587A1FF4213214905` (matches the receipt).
- **Built exe:** `F85111E2…6563`. **Installed exe:** `9E8051A6…5F2E`, version 1.25.0-xlog.32.
- **Install:** 07:21 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **First launch:** 2026-11-25 07:29:56 UTC, with the operator's approval to go on the air.

## Live run (2026-11-25 07:30 to 11:58 UTC, stopped for the xlog.33 build)

Day schedule (17/20/10/15/12 m), FLEX-8400 through SmartSDR CAT, xLog open before Nexus, the rotor connected,
RF checked; the operator at the radio throughout, with pauses (09:15 and 09:20 tuner work, 11:09 to 11:18 SSB).

- **Contacts:** 21 (17 m 9, 20 m 7, 15 m 4, 10 m FT4 1), all in xLog, among them 3B8/E70A on the second call
  (the fix of this version), VK9XY on 10, 20 and 17 m after the beam was turned to it, TR8CA, 8S2DX, VK6ABC,
  VK3AXI, YC8PAD and the activation G8XDD/P. 222 overs keyed and unkeyed, longest 13.7 s. No crash and no
  TX watchdog trip.
- **Clear transmit frequency:** picked 25 times.
- **Switched off by a Hound ending** at 09:40:41 (VK9XY), switched back on by hand (`1b69d56ef`).
- **BG3TFT picked and dropped** eight times in three seconds at 08:51:42 (`791a340ae`).
- **The antenna had to point at the DXpedition before it was heard** (`69e0deff6`).
- **KAT500 fault**, cleared by the operator in the Station Agent: transmit stopped, clear, tune, align
  (`21b22433e`).
- **CAT lost from 11:10:** the slice was away for SSB work, and at 11:11:23 probes from the internet reached
  SmartSDR CAT's port 5002. CAT then logged answers that did not arrive here; restarting the CAT program
  fixed it at 11:21. The operator is closing the port.

Not seen yet: the FT8 + FT4 patrol, the stalled-contact end (`5ee7f0c26`), and a run of five hours.
