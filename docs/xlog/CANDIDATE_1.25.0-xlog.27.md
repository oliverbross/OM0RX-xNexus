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

## Package and installation (2026-11-24)

- **Source:** `5821abf5a`, canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.26 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed. It waited from 02:22 UTC for commit memory (11.66 GiB free of
  the 12.5 required) until the operator freed some. Admission 17.3 GiB physical / 13.2 GiB commit;
  03:37–04:36 UTC; peak 5.44 GiB; lowest free 12.7 GiB physical / 7.9 GiB commit; cleanup confirmed.
  The Home Assistant VM ran throughout: it was not saved.
- **Evidence:** `.build/xnexus-xlog27-package-7g/evidence/20261124T0337025532525Z-a64de3a7cb4c441a95a5bd3ca3ceb3d6`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.27.json` (source `5821abf5a`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.27_x64-setup.exe`, 356,588,685 bytes, SHA-256
  `3402F2AC06389A11D9ECC9F54A31DE41E705EE03860ADAE39A70BC4A6B491E6C` (matches the receipt).
- **Built exe:** `B75496B2…575D`. **Installed exe:** `E83C57D3…DB9E`, version 1.25.0-xlog.27.
- **Install:** 04:37 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **Not launched** when this was written: going on the air needs the operator's approval.
