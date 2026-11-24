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
