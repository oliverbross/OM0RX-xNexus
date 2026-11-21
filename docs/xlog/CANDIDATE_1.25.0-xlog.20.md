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

## Package and installation (2026-11-21)

- **Source:** `66bbf796d`; canonical `main` fast-forwarded locally from `2405e59d4`.
- **Guard:** profile `optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`, the xlog.19 wrapper with only
  the expected version changed. Attempt 1 was interrupted at the final `src-tauri` compile
  (`host-commit-below-6GiB`, 05:52 UTC) with the operator's Home Assistant VM running and no other build.
  Attempt 2 passed after the VM was saved (the operator's approval): `tempo-app` came from cache, compile
  and bundle 06:17–06:50 UTC, peak 5.44 GiB. The VM was running again afterwards.
- **Evidence:** `.build/xnexus-xlog20-package-7g/evidence/20261121T0616521743449Z-bc1101e77db64624a2e49aa516a92718`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.20.json` (source `66bbf796d`, version 1.25.0-xlog.20).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.20_x64-setup.exe`, 356,511,490 bytes, SHA-256
  `89F64F1518AB1E00091D64A82BBE9F349ED0DA562E477FBBB5B5B01C182CB586`.
- **Built exe:** `4BED4E14…38A8`. **Installed exe:** `F81EBA77…29BB`, version 1.25.0-xlog.20
  (bundler-stamped, as before).
- **Install:** Nexus was not running; silent `/S` install, exit 0; the uninstall entry reads 1.25.0-xlog.20.
- **Not checked:** the app was not launched, so the translated screens have not been seen in a real window.
