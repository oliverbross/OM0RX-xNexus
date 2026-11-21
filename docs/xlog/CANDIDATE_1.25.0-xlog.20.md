# 1.25.0-xlog.20: automation and Status filter in every language

Source: the `claude/automation` branch, `2405e59d4` on top of `e37218c6d` (1.25.0-xlog.19).
Upstream base unchanged: kd9taw/Nexus `39b0e946f` (1.25.0, see `UPSTREAM.json`). Identity
unchanged, so an installed xlog.19 keeps its logbook and settings. No engine change: the
autopilot behaves exactly as in xlog.19.

## What changed for the operator

The strings added in xlog.18 and xlog.19 were English only. The 86 of them are now translated
into German, French, Spanish, Italian, Japanese, Russian and Simplified and Traditional Chinese:
Answer callers, the autopilot button, status, tiers, presence prompt and log, Settings ▸
Automation (schedule, tiers, needed US states, DXpedition hunt, Hound), and the Band Activity /
Call Roster **Status** filter. Band and mode names, 73/RR73, DXCC, ATNO and WAS stay as they are,
as the catalogs require. Each catalog's existing terms and register were reused; the wording
has not been reviewed by native speakers.

The autopilot labels, the tier switches and the filter presets now look up their text with
literal keys, so the i18n checks read every one. What the operator sees in English is unchanged.

## Where it is

| Part | Files |
|---|---|
| Catalogs | `ui/src/i18n/{de,fr,es,it,ja,ru,zh,zh-hant}.ts` (86 entries each) |
| Literal keys | `AutopilotControl.tsx`, `SettingsAutomation.tsx`, `LogStatusFilterPicker.tsx` |
| Scanner list | `ui/src/i18n/hardcoded-strings.test.ts` |

## Verification (before packaging)

| Gate | Result |
|---|---|
| `tsc -b` | pass |
| i18n suite (27 files: completeness, placeholders, orphans, computed-key pin, plurals) | 766/766 with the automation UI tests |
| Band Activity, Call Roster and Status filter suites | 245/245 |
| `scripts/browser-probe` | not completed: the hosted UI bundle built, then the worker bundle stopped (`wrangler` is not installed in this worktree's `remote/`) |
| Rust | unchanged since xlog.19 (116/116 automation tests there) |
