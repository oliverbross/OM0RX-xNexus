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

To be filled in as the gates, package, install and live checks run.
