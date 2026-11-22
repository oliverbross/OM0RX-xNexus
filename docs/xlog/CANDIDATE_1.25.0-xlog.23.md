# 1.25.0-xlog.23: your own autopilot priorities, and what the first live run found

Source: `main` at `8df1ff822`, which merges `xnexus/autopilot-reliability` (`ac7df64a1`). Since
1.25.0-xlog.22 (`9c1718168`), the branch adds four commits: `4d5b7d016`, `288a22ddd`, `02803d361`
and `ac7df64a1`. Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity
unchanged, so an installed xlog.22 keeps its logbook and settings.

## What changed for the operator

Asked for during the live run of xlog.22 (operator OM0RX, 2026-11-22):

- **Settings ▸ Autopilot.** The autopilot and Answer callers moved off Settings ▸ Digital to a tab
  of their own, in groups: Autopilot, Priorities, Band schedule, Timing, Answer callers.
- **Priorities you order yourself.** A list of rules, each switched on or off and moved with the
  arrows (*Default order* restores it). The default order:
  1. ATNO
  2. Club Log Most Wanted top 50 (new)
  3. DXpeditions
  4. DXCC never confirmed
  5. Band slots not confirmed
  6. US states
  7. POTA/SOTA/IOTA/WWFF
  8. New grids (new)
  9. Any new call

  "Never confirmed" and "not confirmed" now also cover an entity or band slot already worked but
  unconfirmed, when a new call there could confirm it. A station calling you always comes first.
- **Outside Europe first** within each rule, by DXCC continent. A European ATNO stays first: it is
  its own rule. It can be switched off.
- **Never on the DXpedition's frequency.**
  - A DXpedition, ATNO or top-50 station is called from at least 1200 Hz above its frequency,
    never below.
  - One heard above 500 Hz is first brought to 400 Hz by moving the dial, between contacts, with
    transmit put back as it was.

Found by watching xlog.22 on the air:

- **The 10-minute band move came late** (14 and 16 minutes). A held move let another ordinary call
  start, which held the band again. A due move now waits for its hold.
- **A chased spot was crowded out.** While listening for 5U4JSF and C6AVD the autopilot called and
  worked ordinary stations. A chase now calls its station, or anything above an ordinary new call,
  and nothing ordinary.
- **A spot never heard was chased again after 15 minutes** (W5BUY, KB1ZBZ, C6AVD). Now 30.

## Live run of 1.25.0-xlog.22 (2026-11-22, 12:29–15:30 UTC)

- **What it did:**
  - 20 m from 12:29, then 17, 12, 10 and 15 m.
  - Contacts included LA2JKA, M0WWE, UB1AUW, BA3KY (a caller), RU4LM, UA3QQG, SP5UTI, 9A7R, 4J3DJ,
    UA6EED, CT2FZZ, TF4CU, KN0BLE, W1ARY, EB5B, EA2FLK, LZ1DIO, MI0TBE, EA7R, CT2GRF/P, F5SJH/P,
    SP9LKN and EA3FAB.
  - It chased cluster spots of needed stations (5U4JSF, W5BUY, KB1ZBZ, C6AVD, N1AQ, KI5F).
- **What did not happen:**
  - No second report to a worked station.
  - The TX watchdog never stopped it.
  - No unplanned disarm.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check`, `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | pass |
| `cargo test -p tempo-app --lib automation` / `autopilot` | 25/25, 33/33 |
| `cargo test -p propagation --lib dxcc` | 58/58 |
| Failing-first | each live-run fix fails without it: the held band move, the chase, the 1200 Hz split |
| `cargo clippy -p tempo-app --lib --tests -D warnings` | only the existing `logstore::lane_of` dead code |
| `cargo check src-tauri --lib --features radio` (with the libvpx prefix) | pass |
| `tsc -b` | pass |
| Vitest, the 187 files touching Settings, the registry, the catalogs and the autopilot control | 2343 passed, 3 failed: `SettingsPanel.rttyauto` (no `navigator`), one `contestgroups` and two `experience` tests. All three fail identically on `main` |
| `node scripts/gen-settings-reference.mjs --check` | up to date |
| Not run | the whole vitest suite to completion with its summary, browser probe, CI (GitHub Actions billing is blocked) |
