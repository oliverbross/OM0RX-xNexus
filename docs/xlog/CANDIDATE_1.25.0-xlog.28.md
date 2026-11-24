# 1.25.0-xlog.28: what the xlog.27 live run found

Source: `main` at `6c4b7efad`, which merges `xnexus/autopilot-reliability` (`7e330a19a`). Since
1.25.0-xlog.27 (`5821abf5a`), the branch adds two commits: `b146e4846`, `7e330a19a`.
Upstream base unchanged (kd9taw/Nexus v1.25.0-beta.2, `21f743404`). Identity unchanged, so an
installed xlog.27 keeps its logbook and settings.

## What changed for the operator

- **A needed station is called when it ends a contact** (`b146e4846`; XE2X on 80 m went from one caller
  to the next with 73 and RR73 and no CQ). A needed station is anything but a grid, an ordinary new
  call or a caller to us; those still wait for a CQ or a call.
- **An unanswered call for a grid or an ordinary new call gives way to a needed station** (`7e330a19a`;
  XE2X on 160 m called CQ beside the calling of MW7TTA). A station that has answered is worked to the
  end.

Not changed: a low-confidence decode (marked `^`) is never acted on, XE2X's 160 m CQs among them.

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo test -p tempo-app --lib auto` | 168/168 |
| `cargo test -p tempo-app --lib` | 4098 passed, 8 failed, all of which fail without the changes too |
| Failing-first | shown for both changes |
| `cargo clippy -p tempo-app` `-D warnings` | pass |
| Not run | tempo-audio and src-tauri (no change there but the version), the UI suites (no UI change), browser probe, CI (GitHub Actions billing is blocked) |

## Package and installation (2026-11-24)

- **Source:** `e16246c43`, canonical `main`, clean, equal to `origin/main`.
- **Guard:** the xlog.27 wrapper and profile (`optimized-no-lto-cgu16-7GiB-physical14-commit12p5-reserve6`)
  with only the expected version changed. It waited from 06:18 UTC for commit memory (11.76 GiB free)
  until the operator closed SmartSDR, CAT and DAX and had xLog closed. Admission 17.0 GiB physical /
  13.1 GiB commit; 06:25–07:21 UTC; peak 5.44 GiB; lowest free 12.7 GiB physical / 7.9 GiB commit;
  cleanup confirmed. The Home Assistant VM was not saved.
- **Evidence:** `.build/xnexus-xlog28-package-7g/evidence/20261124T0625187092032Z-a46944555eab44678da534ce63231028`.
- **Receipt:** `target/xnexus-provenance/1.25.0-xlog.28.json` (source `e16246c43`, upstream base
  `21f743404` v1.25.0-beta.2, toolchain 1.93.1-x86_64-pc-windows-gnu, unsigned).
- **Installer:** `OM0RX-xNexus_1.25.0-xlog.28_x64-setup.exe`, 356,576,984 bytes, SHA-256
  `3E3F45FDE65010C25BA4C005C2A5678E1A56CF0A808227FF7C2135FFB7ABDD0C` (matches the receipt).
- **Built exe:** `C5021264…4C57`. **Installed exe:** `81E7A045…FDF5`, version 1.25.0-xlog.28.
- **Install:** 07:21 UTC, `/S`, exit 0, with no Nexus running; `settings.json` unchanged.
- **Not launched** when this was written: going on the air needs the operator's approval.
