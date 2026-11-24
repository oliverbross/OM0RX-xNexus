# 1.25.0-xlog.29: FT8 and FT4 together, and what the xlog.28 live run found

Source: `main` at `4b6bafe54`, which merges `xnexus/autopilot-reliability` (`cb815e653`). Since
1.25.0-xlog.28 (`e16246c43`), the branch adds seven commits: `c87dfcc8d`, `5ee7f0c26`, `541e5a762`,
`353ccc88e`, `e2b25fdb5`, `8bcffc8ea`, `cb815e653`.
Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an
installed xlog.28 keeps its logbook and settings. An older version drops a schedule window set to
FT8 + FT4.

## What changed for the operator

- **The FT8 decoder no longer ends the program** (`cb815e653`, Windows): the executable wraps
  `setlocale`, so the Fortran runtime hands no freed text to the C runtime.
- **FT8 and FT4 at the same time** (`8bcffc8ea`, operator request 2026-11-24): a window can be FT8 + FT4,
  and windows with the same start run together. Every band has its turn in each mode, and needed spots
  in either are chased. A later start still takes over alone.
- **The TX watchdog no longer stops the autopilot over a contact that went quiet** (`5ee7f0c26`): the
  autopilot ends a stalled contact after two minutes. The watchdog is unchanged.
- **A DXpedition that others are heard calling is followed for ten minutes** and hunted again each hour
  (`c87dfcc8d`). **The transmit offset goes back where it was** after one (`541e5a762`).
- **A band with only new grids gives way after ten minutes** (`353ccc88e`).
- **A spot is not hunted for a new mode alone** (`e2b25fdb5`); a DXpedition still is.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib auto` | 177/177 |
| `cargo test -p tempo-app --lib` | 4107 passed, 8 failed, all of which fail without the changes too |
| Failing-first | shown for the ten minutes, the hourly hunt, the stalled contact, the offset and the new mode; not for the grids dwell and the two-mode patrol |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| `setlocale` wrap | test programs: 7,880 garbage texts in 20 s without, none with; the Rust file built and run with the release toolchain; not yet in Nexus itself |
| UI `tsc -b`; i18n, autopilot and feed suites | pass; 773/773 |
| Not run | the commits one by one, tempo-audio, src-tauri tests and clippy, the whole UI suite, browser probe, CI (GitHub Actions billing is blocked) |
