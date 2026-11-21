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
