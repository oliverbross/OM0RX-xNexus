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
