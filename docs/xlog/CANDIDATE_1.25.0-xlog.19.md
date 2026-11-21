# 1.25.0-xlog.19: no band change mid-QSO, needed US states, DXpedition hunt

Source: the `claude/automation` branch, `1f714e33f` on top of `5569546a8` (1.25.0-xlog.18).
Upstream base unchanged: kd9taw/Nexus `39b0e946f` (1.25.0, see `UPSTREAM.json`). Identity
unchanged, so an installed xlog.18 keeps its logbook and settings.

Built after the operator's first on-air test of the xlog.18 autopilot (2026-11-21): it changed
band in the middle of a QSO. The operator's rule: the autopilot may change band only when it
cannot call anyone and every contact is finished. The operator also asked for a needed-US-state
chase and a DXpedition hunt that follows the cluster to the DXpedition's own frequency and then
returns. WSJT-Z's and MSHV's sources were read first: WSJT-Z hops only from its CALLING state (after
the 73 is sent); MSHV has no hopper; neither follows spots or DXpeditions.

## What changed for the operator

**The band never changes in the middle of a QSO.** The cause: a contact counted as finished once
our closing over had gone out once, or once its retry budget ran out, so the autopilot could change
band while the other station was still sending to us. Stations calling us and CQ runs were not
considered either. Now every band or mode change waits while:

- a contact is being called or worked, or a CQ run is going;
- the other station of a contact made on our side was heard in the last 4 slots (they owe a 73 or
  are repeating);
- anyone called us in the last 4 slots, even with Answer callers off;
- Answer callers has someone waiting.

Nothing new is started while that station is still repeating, and "nothing to call here" is counted
only from the moment everything is finished. The top bar shows **Holding the band** or **Letting
the contact finish** while it waits, and the log says why.

**Needed US states** (Settings ▸ Automation ▸ What the autopilot may call, off by default). Calls a
station in a US state not yet worked on the band (the need board's New state), ranked below the
DXCC tiers. **Only these states** narrows it to a typed list (e.g. `VT, ND, AK`); empty = every
needed state. A tier that is switched off no longer hides a station that also falls in a tier that
is switched on (an ATNO in a needed state is called as a state with ATNO off).

**DXpedition hunt** (Settings ▸ Automation, on by default). A needed, listed DXpedition spotted on
any band your licence allows, not only the scheduled bands:

- waits until every contact is finished and nobody is calling;
- goes to the frequency the spot names (a spot inside the standard channel's passband keeps the
  standard dial; any other is rounded down to the kHz), switching FT8/FT4 if the spot says so;
- calls only the DXpedition and stations calling you;
- switches to Hound when the DXpedition is heard transmitting below 1000 Hz (optional, on by
  default), and puts your setting back when the hunt ends, on Stop TX, TX Off, a presence timeout or
  when the autopilot is switched off;
- ends when worked, after the DXpedition time budget (default 15 min), when not heard within 5 min,
  or when the spot goes away, and returns to the band, frequency and mode it left, waiting again
  for every contact to finish.

## Sign-off

FT QSO sequencing changes need the maintainer's explicit approval (CLAUDE.md). The operator asked
for these changes on 2026-11-21 for testing, as a continuation of the fork-only autopilot accepted
on 2026-11-20.

## Where it is

| Part | Files |
|---|---|
| Settings and pure decisions | `crates/tempo-app/src/automation.rs` (US-state tier, wanted states, hunt settings, `hunt_dial`, `best_allowed_tier`) |
| Engine | `crates/tempo-app/src/engine/autopilot.rs` (band hold, partner tracking, hunt), `engine/autopilot_tests.rs`, `engine.rs` (`halt_tx` puts Hound back) |
| UI | `SettingsAutomation.tsx`, `AutopilotControl.tsx`, `features/autopilotFeed.ts` (+ test), `types.ts`, `i18n/en.ts`, `i18n/hardcoded-strings.test.ts` |

## Verification (before packaging)

| Gate | Result |
|---|---|
| Automation and autopilot tests | 116/116, including 14 new |
| Positive control | with the band hold and the repeat guard disabled, `the_band_never_changes_while_the_partner_is_still_sending` fails at slot 38 (the reported mid-QSO change); restored, it passes |
| `cargo fmt` (workspace) | pass |
| `cargo clippy -p tempo-app --all-targets -D warnings` | one error, pre-existing and outside this change: unused `lane_of` in `logstore.rs` |
| tempo-app (whole crate) | 4030 pass, 7 fail, all outside automation (`remote_logging` journal, `fd_scoreboard`, `logstore`, `upload_start` x4: Windows path and network conditions; xlog.18 had 8 in the same families) |
| `tsc -b` | pass |
| UI | `AutopilotControl.test.tsx` 9/9, `autopilotFeed.test.ts` 3/3 |
| i18n | the orphan scanner now reads the automation components; still failing from earlier work: translated catalogs lack the automation keys (86 per locale, 13 new here), and `LogStatusFilterPicker.tsx` is not on the scanner's list |
| src-tauri | not compiled separately; no struct it builds changed |

Local toolchain note: before these gates ran, 36 MSYS2 MinGW packages under `C:\msys64\ucrt64` had
lost files outside pacman; they were reinstalled at the same versions with the operator's approval.

## Package and installation (2026-11-21)

- **Source:** `d9f6909cd`; canonical `main` fast-forwarded locally from `5569546a8` (not pushed).
- **Before the build:** the same overnight deletion (00:56) that emptied `C:\msys64\ucrt64\lib` had also
  removed 11 tracked `.wav` test fixtures from both checkouts, `target\xnexus-vpx-1.17.0\lib\libvpx.a` and
  three cached static libraries (`libopus.a`, two `tract-linalg` kernels). With the operator's approval
  the fixtures were restored from git; libvpx 1.17.0 was rebuilt by `scripts/xnexus/prepare-windows-deps.sh`
  (pinned tarball, SHA-256 checked; new archive `0647B456…BA28`); the two stale build-script caches were
  cleared so cargo re-ran them. A scan of every build-script output found 12 static libraries present and
  only those 3 missing.
- **Guard:** profile `optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`, the xlog.18 wrapper with
  only the expected version changed. Attempt 1 stopped before compiling (libvpx missing). Attempt 2 was
  interrupted at the final `src-tauri` compile (`host-commit-below-6GiB`) when an xLog `cargo test` started
  beside it. Attempt 3 passed after the operator's Home Assistant VM was saved: compile 32 min 40 s,
  03:17–03:57 UTC, peak 5.44 GiB.
- **Evidence:** `.build/xnexus-xlog19-package-7g/evidence/20261121T0317078490619Z-41aec97421cb424688ed5b62e722bd35`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.19.json`.
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.19_x64-setup.exe`, 356,515,494 bytes, SHA-256
  `9C2B9A5144DBE15F9836780DA354FF46591DF03D7F166B0E88795F9CA0AC801A`.
- **Built exe:** `178C81FD…3DD2`. **Installed exe:** `0C2CB489…6F1`, version 1.25.0-xlog.19
  (bundler-stamped, as before).
- **Install:** Nexus was not running; silent `/S` install, exit 0; the uninstall entry reads 1.25.0-xlog.19.
- **Not checked:** the app was not launched (it connects to the radio over CAT); the autopilot changes need
  transmit and are for the operator to try at the station.
