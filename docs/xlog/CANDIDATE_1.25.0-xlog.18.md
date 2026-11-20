# 1.25.0-xlog.18: CS/DS filter, Answer callers and the supervised autopilot

Source: the `claude/automation` branch on top of `6551f2bea` (1.25.0-xlog.17). Upstream base
unchanged: kd9taw/Nexus `39b0e946f` (1.25.0, see `UPSTREAM.json`). Identity unchanged, so an
installed xlog.17 keeps its logbook and settings.

Built from the xNexus automation proposal the operator accepted on 2026-11-20, with one change
for testing: Settings ▸ Automation has an Autopilot on/off switch and an editable presence-check
interval of 2–999 minutes, default 10.

## What changed for the operator

**Band Activity and Call Roster: Status filter.** A **Status** chip opens xLog's two boxes, Call
status and DXCC status, plus quick choices (Show everything, Only new calls, New DXCC slots, Not
yet confirmed). A row shows when both its CS and its DS are ticked. The filter combines with the
existing chips, so CQ+73 plus "Only new calls" shows only CQ and 73 lines from stations still
needed. Your own lines, stations calling you and your QSO partner always show. Each pane keeps its
own choice per window.

**Answer callers** (Settings ▸ Automation, off by default). While you work a station you called,
stations calling you are queued. Once your 73 or RR73 to that station is on the air, Nexus answers
the best caller instead of turning transmit off. A caller may take over if the station you called
has not answered after N calls (default 2; 0 = never). After N answered callers in a row (default
3) transmit goes off as before. A caller not heard for 3 slots is dropped. TX off means nobody is
answered.

**Supervised autopilot** (Settings ▸ Automation ▸ Allow the autopilot, off by default). An
**Autopilot** button appears in the top bar beside the Tx cycle. With transmit on, pressing it
starts the autopilot, which:

- follows the band schedule (UTC windows, each with bands in order and FT8, FT4 or FT2; default
  07:00 20–10 m and 20:00 160–20 m, FT8);
- listens 2 slots on a band, then calls the most needed CQ it hears, in strict tiers: callers to
  you, ATNO, needed DXpeditions, DXCC worked-not-confirmed slots, DXCC new slots, new
  parks/islands, new calls; never Worked or Confirmed;
- gives each station 2 minutes (15 for an ATNO or DXpedition), then tries again after 10, 30 and
  60 minutes, and leaves it after 3 failures;
- moves to the next band after at least 3 minutes on a band and 5 minutes with nothing to call;
- moves to another scheduled band for a needed spot (ATNO, DXpedition or DXCC slot, same mode),
  listening there first and calling only once it decodes the station itself; gives up after 5
  minutes unheard;
- stays below 60 contacts started per hour;
- leaves a DXpedition calling below 1000 Hz (Fox/Hound) alone: Hound stays the operator's choice.

The top bar shows the status, band and target, a presence countdown with **I'm here**, and a log
of recent decisions.

**Safety.** The autopilot never turns transmit on by itself. Stop TX, TX Off and a tripped TX
watchdog switch it off; it does not restart. In the last minute of each presence interval a prompt
asks "Autopilot: are you at the station?". Without an answer, the contact in progress finishes,
then transmit goes off and the autopilot switches off. A band or mode change made by the autopilot
halts TX like any QSY and then restores the operator's arm and the watchdog clock exactly as they
were. The TX-enable latch, licence gating, identity validation at keying and the wall-clock
watchdog are unchanged. None of this can be switched on or driven from another computer (the
three new commands are refused from afar, and a settings save from afar keeps the station's own
automation settings).

## Sign-off

FT QSO sequencing changes need the maintainer's explicit approval (CLAUDE.md). The operator
accepted the proposal, including the fork-only divergence from WSJT-X, on 2026-11-20 for testing.

## Where it is

| Part | Files |
|---|---|
| Status filter | `ui/src/logStatusFilter.ts` (+ test), `ui/src/components/LogStatusFilterPicker.tsx`, `OperateDecodes.tsx`, `OperateRoster.tsx`, `styles.css`, `i18n/en.ts` |
| Settings and pure decisions | `crates/tempo-app/src/automation.rs` (+ 15 tests), `settings.rs` (`automation`) |
| Engine | `crates/tempo-app/src/engine/autopilot.rs`, `engine/autopilot_tests.rs` (13 tests), hooks in `engine.rs` (decode fold, `halt_tx`, after-73 disable, `call_station_ctx_arming`) and `engine/ft_midover.rs`, snapshot `dto.rs` |
| Commands | `src-tauri/src/lib.rs` (`set_autopilot`, `autopilot_confirm_presence`, `autopilot_feed`), `crates/tempo-client/src/classes.rs` (refused from afar), `src-tauri/src/command_dispatch.rs` |
| UI | `ui/src/components/AutopilotControl.tsx` (+ test), `SettingsAutomation.tsx`, `TopBar.tsx`, `App.tsx`, `features/autopilotFeed.ts`, `api.ts`, `types.ts`, `remote-web/configuration-schema.ts` |

## Verification

| Gate | Result |
|---|---|
| `tsc -b` | pass |
| fmt (workspace and src-tauri) | pass; the sweep touched only this change's files |
| fork-policy | 6/6 |
| New tests | `automation` 15/15, `engine::autopilot_tests` 13/13, `AutopilotControl.test.tsx` 9/9, `logStatusFilter.test.ts` 8/8 |
| UI suites touched | 51 Band Activity, Call Roster and stop-line files: 417 pass, 1 skipped; 16 top bar and settings files: 130 pass; remote-web: pass |
| tempo-app (whole crate) | 4016 pass, 8 fail (xlog.17: 3986 pass, 10 fail). The 8 are Windows-path and timing tests (`logstore`, `upload_start`, `fd_scoreboard`, a sweep timing, a remote-logging journal); none in the sequencer or automation. |
| src-tauri lib (manifested harness) | 1951 pass, 20 fail (xlog.17: 1934 pass, 37 fail), all in the families that failed there (remote LAN stop-road, rotctld, keychain, DXKeeper uploads). Command classes, dispatch and settings: 146/146. |
| Full vitest run | not re-run end to end: stopped to free memory for the guarded build. |

## Package and installation (2026-11-20/21)

- **Source:** `a3d8c23c9`; canonical `main` fast-forwarded locally (not pushed).
- **Guard:** profile `optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`; admitted on the third
  attempt, after Docker Desktop was stopped and WSL shut down (a separate virtual machine
  stayed running). Compile 53 min, peak 5.43 GiB.
- **Evidence:** `.build/xnexus-xlog18-package-7g/evidence/20261120T2206473860734Z-d32db23101484cd9bc1a2297839b069c`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.18.json`.
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.18_x64-setup.exe`, 356,471,460 bytes, SHA-256
  `1D60F2EFB861C9BEAAF6453D649AECF6428B29EAE66038E8B5E2FE9BB776ACA4`.
- **Built exe:** `85EF6FFE…0C06`. **Installed exe:** `2009FA06…B28F`, version 1.25.0-xlog.18
  (bundler-stamped, as before).
- **Install:** Nexus closed normally first (fresh launch, TX off); silent `/S` install, exit 0.

## Live check (receive only)

- Nexus `v1.25.0-xlog.18` opened on 20 m FT8, TX Off, CAT ✓, decoding live.
- The Status chip is in Band Activity and the Call Roster; its menu shows the quick choices and
  both boxes with xLog's codes and colours.
- Settings ▸ Digital shows the Automation section with the defaults: Answer callers off,
  autopilot off, presence check every 10 minutes. With the autopilot off, no Autopilot button is
  shown, as designed.
- Not checked: Answer callers and the autopilot on the air. Both need transmit, so they are for
  the operator to try at the station.
- Layout note: the Automation section renders in one narrow column of the Digital grid; usable,
  to be widened in the next candidate.
