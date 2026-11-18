# 1.25.0-xlog.16 full Nexus with xLog candidate

Pre-version integrated source: `8ddb9b751` (full-mode default) on top of the CS/DS
commit `2dfd77ec9`, the build fixes `c703d081c` and the upstream merge `d4b93f539`
(upstream `39b0e946f`, 256 commits since `c88740a9`; `UPSTREAM.json` now names it). This commit
changes only the version surfaces and adds this record; the package step binds the
resulting exact clean commit.

## What changes for the operator

- **Full Nexus is the normal start.** A plain launch opens the complete upstream app
  (all modes, the full Settings, Nexus's own FT engine and limits). The reduced
  xLog-owned companion is a separate explicit launch, `--xlog-companion`.
  `--independent-mode` is still accepted and changes nothing.
- **CS and DS columns** in the Call Roster and on every Band Activity row: the call's
  status and the DXCC entity's status in the operator's own Nexus logbook on the
  current band, by mode class (CW, phone, digital), in xLog's codes and colours.
  CS: New Call, New Band, New Mode, New Band+Mode, Worked, Confirmed. DS: ATNO, C/ or
  W/ New Band, New Mode, New Band+Mode, Worked, Confirmed. "Confirmed" is
  award-grade (LoTW or card), as Nexus's awards count it. Off the bands, for own TX
  and for a call with no entity the cells stay empty, never "unworked".
- **Upstream 1.25.0 work**, merged.

## Working with xLog (xLog stays the master logbook)

- Nexus → xLog: Settings ▸ Services ▸ WSJT-X UDP API on, `127.0.0.1:2237`.
- xLog → Nexus: xLog's sync agent (`tools/nexus_sync` in the xLog repo) sends new
  xLog contacts to Nexus's contact port; confirmations arrive by ADIF import.
- Radio: SmartSDR CAT (`127.0.0.1:5002`) and DAX, PTT method CAT.

## Verification at this checkpoint

- UI typecheck (`tsc -b`) clean; fork policy test 8/8.
- HotIndex CS/DS unit test covers every CS and DS outcome and the self-check
  (`verify`, `answers_as`) includes the new sets.
- Launch-mode unit test: plain launch is full, `--xlog-companion` is the companion.

Not run here: CI's browser probe (`scripts/browser-probe`) and the macOS/Linux jobs.
No on-air TX was made with this candidate before installation.

## Package and installation (2026-11-18)

- Source: `de1b9edb3` (this record's reservation commit), clean canonical checkout.
- Guarded package attempt 1 (default 4 GiB job cap, LTO profile not reaching the
  child): stopped by the guard at the job memory limit during the final
  compile, no artifact. Attempt 2: external supervisor profile
  `optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6` (7 GiB job cap,
  `CARGO_PROFILE_RELEASE_LTO=false`, `CODEGEN_UNITS=16`, one job, 6 GiB host
  floors; admission commit lowered from 13 to 12.5 GiB because a VM held 4 GiB).
  Status `passed`, peak 6.17 GiB, evidence
  `.build/nexus-16-build/20261118T1724105025890Z-17a059b8bc8844cf887d11db77f960a0`.
- Installer `OM0RX-xNexus_1.25.0-xlog.16_x64-setup.exe`, 286,500,241 bytes,
  SHA-256 `d7dcad06372263f74be6ef2e70ae033bed805878980edf2c5f8edf7821afc1e6`.
  Built executable SHA-256 `f34233d4…f3710b`; installed executable
  (bundler-stamped) `6f80015d7394efb1369cc0eacb0ca4ad6c22b63cc4a7ddabf8ebd6d609f084a9`,
  product version `1.25.0-xlog.16`.
- Installed current-user, silent, after closing the previous full Nexus normally.

## Live acceptance (receive only, no transmission)

- A plain launch (no arguments) opened full Nexus 1.25.0-xlog.16: the full mode
  bar, all cockpits, the full Settings; TX Off at launch.
- FT8 on 30 m (FLEX via SmartSDR CAT/DAX): 17 stations decoding in the Call Roster
  and Band Activity, CS and DS filled on every row.
- CS checked against xLog's database for four calls: OM2BK (30 m FT8 confirmed)
  → C; EA5EB (not in the log) → NC; EB5R (worked on other bands only) → NB;
  CN2JA (30 m worked, unconfirmed) → W. All match xLog's rules.
- Settings preserved across the upgrade: WSJT-X UDP `127.0.0.1:2237` on, PTT CAT.
  Nexus's contact port `127.0.0.1:2239` is listening; xLog's sync agent running.
