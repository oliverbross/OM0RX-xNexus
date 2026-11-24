# 1.25.0-xlog.27: what the xlog.26 live run found

Source: `main` at `e65bff460`, which merges `xnexus/autopilot-reliability` (`05cf0d4fd`). Since
1.25.0-xlog.26 (`d089512d1`), the branch adds three commits: `598ae431a`, `1d223d965`, `05cf0d4fd`.
Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an
installed xlog.26 keeps its logbook and settings.

## What changed for the operator

- **A needed station still calling CQ is called again after a minute** when there is nobody else to
  call (`598ae431a`; 8Q7XX on 80 m called CQ beside an idle autopilot). Anybody else wanted comes first,
  the move to the next band is not put off, and grids and ordinary calls are not called again so soon.
  A chase that called its station no longer ends as "not heard".
- **A station that answers late is not dropped after four overs** (`1d223d965`; D2ACE).
- **The scheduled band that has waited longest comes next** (`05cf0d4fd`; 30 m). A band counts as
  visited after its minimum dwell. A chase still leaves a scheduled band at once.

Not changed: the transmit offset stays where Hold Tx puts it.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib auto` | 164/164 |
| `cargo test -p tempo-app --lib` | 4093 passed, 9 failed: the 8 that fail without the changes, and `a_single_90_ms_stall…`, which passes alone (3 of 3) |
| Failing-first | shown for calling again, for the late answer and for both rotation tests; the "not heard" test was not run against the unfixed code in its final form |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| Not run | the three commits one by one, tempo-audio and src-tauri (no change there but the version), the UI suites (no UI change), browser probe, CI (GitHub Actions billing is blocked) |
