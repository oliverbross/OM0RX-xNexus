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

## Package and installation (2026-11-22)

- **Source:** `617615f44` (this reservation), canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.22 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed.
  - Attempt 1 (`20261122T2108398756475Z-…`) was held at admission: 12.26 GiB commit available,
    12.5 needed. The Home Assistant VM had been saved with the operator's approval and its
    availability task paused. Nothing was built.
  - Attempt 2 passed after the running Nexus had closed: admission 18.3 GiB physical / 13.7 GiB
    commit; 21:09–22:29 UTC; peak 5.44 GiB. Afterwards the pause file was removed and the VM
    started again.
- **Evidence:** `.build/xnexus-xlog23-package-7g/evidence/20261122T2109127786560Z-09fefd1730af43efaaa2e979ffb56f8e`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.23.json` (source `617615f44`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.23_x64-setup.exe`, 356,521,737 bytes, SHA-256
  `F2A5571DE7110FFB9D7A617C9C633C67BF0B4FC4B36A461DC131F1AC4D0EDECF`.
- **Built exe:** `10408E6B…1524`. **Installed exe:** `E7F336A8…DFB0`, version 1.25.0-xlog.23.
- **Install:** the first silent install exited 2 because the operator had restarted Nexus; with
  their go-ahead it was installed after they quit it (22:31 UTC): `/S`, exit 0.
- **Launch:** 22:32 UTC. CAT failed on the first open (Hamlib: empty reply from SmartSDR CAT,
  `verify_flexradio_id`) and did not recover by itself; a graceful restart at 22:34 connected at
  once. Whether the CAT breaker would have recovered is not in the log (its re-probes write
  nothing there).
- **Live run:** TX on, then the autopilot on at 22:35 UTC.
  - 60 m: F4BND (Grid), G0ORC, IU8STX (an Answer-callers caller); 40 m: BA7LUI, TA2TDW while it
    listened for a spotted SU3YM.
  - Defects found, all fixed for the next candidate:
    - the autopilot tuned 60 m to the US channel 5.3715 MHz instead of the station's 5.357;
    - five US-state spot chases ran back to back with no contact in 22 minutes;
    - VK9XY (a DXpedition, CQ on 40 m) was passed over for a US-state spot, because the
      autopilot ranked the boundary decode fold alone;
    - Answer callers' takeover dropped the operator's call to VK9XY for a caller, twice.
  - The operator called VK9XY by hand from 23:32 at 7.0571 MHz (VK9XY at 401 Hz, TX at 1375–2375 Hz):
    it answered at 23:48 (-16), did not copy our R report in three tries, and was not heard after
    23:50. No contact.