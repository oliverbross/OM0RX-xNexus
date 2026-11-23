# 1.25.0-xlog.24: what the xlog.23 live run found

Source: `main` at `cb17ed5e0`, which merges `xnexus/autopilot-reliability` (`031976d89`). Since
1.25.0-xlog.23 (`617615f44`), the branch adds one commit, `031976d89`. Upstream base unchanged
(kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an installed xlog.23 keeps its
logbook and settings.

## What changed for the operator

Found by watching xlog.23 on the air (operator OM0RX, 2026-11-22, 22:35–24:00 UTC):

- **60 m is tuned where the station's own 60 m button tunes it.** The autopilot's band changes used
  the US 60 m channel (5.3715 MHz) for every station, so for OM0RX it listened where nobody was,
  and a call there would have gone out above the 5351.5–5366.5 kHz segment. It now uses the
  station's band plan (5.357 MHz for a non-US call) with the operator's own working frequencies.
- **US-state, park and grid spots no longer take over the evening.** After a chase that never heard
  its station, the next spot of those rules waits 10 minutes, so the band schedule gets its turn.
  New entities, Most Wanted, DXpeditions and DXCC slots are still chased at once.
- **A DXpedition heard on the band is called, not passed over.** An FT8 slot comes in two decode
  folds, and the boundary one carries only what the early one did not. The autopilot ranked each
  fold alone, so VK9XY's CQ (early fold, spent listening) was missed in the boundary fold and a
  US-state spot on 60 m was chased instead. It now ranks everything heard in the slot.
- **A station calling you never takes you away from a DXpedition.** Answer callers' takeover ("A
  caller takes over after this many unanswered calls") dropped the operator's call to VK9XY for
  N5EI, twice. A DXpedition, a new entity or a rare autopilot target is never taken over.

During the run the operator's takeover setting was set to 0 (never) by hand, to keep the VK9XY call;
with this candidate it can go back to 2.

## Not changed, noted

- **CAT on the first launch.** xlog.23's first launch did not connect CAT (Hamlib reported an empty
  reply from SmartSDR CAT) and did not recover by itself in about two minutes; a restart connected
  at once. The CAT breaker's re-probes write nothing to the diagnostic log, so whether it would have
  recovered is unknown. Left for a separate change, with logging first.
- **A DXpedition spot naming the wrong mode.** The cluster spotted VK9XY on 40 m FT4 (7.048) while it
  was running FT8 at 7.056, so the hunt listened in the wrong mode. Not changed: the spot is what
  the cluster says.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib autopilot` / `automation` | 38/38, 25/25 |
| Failing-first | each fix fails without it: the 60 m dial (5.3715 vs 5.357), the chase rest, the takeover guard, the slot-wide ranking |
| `cargo clippy -p tempo-app --lib --tests -D warnings` | only the existing `logstore::lane_of` dead code |
| Not run | src-tauri check (no src-tauri change), the UI suites (no UI change), browser probe, CI (GitHub Actions billing is blocked) |
