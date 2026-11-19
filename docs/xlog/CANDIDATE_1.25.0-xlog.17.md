# 1.25.0-xlog.17: full Nexus without the xLog companion

Source: the `claude/remove-companion` branch on top of `124c2953c` (1.25.0-xlog.16),
one fast-forward commit that removes the companion. Upstream base unchanged:
kd9taw/Nexus `39b0e946f` (1.25.0, see `UPSTREAM.json`).

## What changed for the operator

Nothing visible in normal use. Since 1.25.0-xlog.16, a plain launch already opened
full Nexus. This candidate removes the companion launch mode (`--xlog-companion`)
and all the code behind it. Identity is unchanged byte for byte: product name,
bundle identifier `sk.om0rx.xnexus`, data folder `%APPDATA%\om0rx-xnexus`, the
`OM0RX-xNexus` local folders and keychain services. An installed xlog.16 therefore
keeps its logbook and settings across the upgrade.

## How the fork was rebuilt

The tree was reset to upstream `39b0e946f` and only what the fork uses was put
back. Everything removed remains in Git history.

**Removed.**

- `integrations/xlog`, `xlog-paired`, `xlog-rx`, `xlog-tx-paired`,
  `xlog-tx-prepare`, `xlog-tx-sequence`: the companion client, RX, TX and paired
  acceptance crates.
- `src-tauri/src/xlog_*` and the `xlog_mode` startup branch in `lib.rs`, plus
  the `xnexus-xlog-*` and `xnexus-tx-prepare` dependencies, `sha2` and
  `objc2-foundation` (companion-only).
- `ui/src/xlog/**`, `ui/xlog.html` and the companion probe, together with every
  fork change to shared UI files. Those changes were the companion's
  owner/external data path: operate services, external log/rotor/spot sources,
  the owner contact editor and manual create, nullable operating radio, and the
  xLog tint. Their translations went with them. `ui/` is upstream again except
  for CS/DS.
- `scripts/xnexus/acceptance/`, the 1.24.1-xlog.5 companion upgrade chain
  (`upgrade-inputs`, `execute-owned-upgrade`, `upgrade-transaction` and its
  test, `UPGRADE_ACCEPTANCE.md`), and the companion test drivers
  (`test-paired`, `test-rx-owner-chain`, `test-rx-recovery`,
  `test-needs-cache-chain`, `test-ui-needs-owner`, `start-ui-owner`).
- The companion plans, protocol, guides and old candidate records under
  `docs/xlog/`.
- The macOS private-WebView-store policy tests; the minimum macOS version is
  back to upstream's 12.0.

**Kept, and why.**

| Kept | Why |
|---|---|
| `src-tauri/tauri.conf.json`, `tauri.distribution.conf.json`, `Cargo.toml` package/bin names, `ui/package*.json` version | Fork identity and version `1.25.0-xlog.17`. Identical identifiers keep the installed app's data. |
| `src-tauri/src/lib.rs` (data folders, `XNEXUS_*` variables, keychain names, paths), `remote_service/vault.rs`, `street_map.rs`, `sstv_scope_test.rs`, `test_dirs.rs` | Identity: the fork never shares upstream Nexus's settings, logbook or keychain. |
| Upstream self-updater disabled (`lib.rs`, `client_mode.rs`, `capabilities/default.json`, `Cargo.toml`) | The fork has no signed release channel; it must never install an upstream build over itself. |
| `crates/tempo-core/src/logbook/hot.rs`, `crates/tempo-app/src/{dto,engine,lib,station}.rs` | CS/DS: the hot index answers call and DXCC status by band and mode class. |
| `ui/src/components/LogStatusCell.tsx` (+ test), `OperateRoster.tsx`, `OperateDecodes.tsx`, `types.ts`, `styles.css`, 9 i18n catalogs, `OperateRoster.park.test.tsx`, `hardcoded-strings.test.ts` (one list entry) | CS/DS columns in the Call Roster and Band Activity, in xLog's codes and colours. Standalone now; no companion cell. |
| `crates/tempo-fast-sys/build.rs` | Native GNU Windows build: static runtime libraries including `gomp` (OpenMP), bounded parallel CMake. |
| `scripts/gen-rust-licenses.py`, `test-gen-rust-licenses-offline.py`, `licenses/rust/*`, `scripts/fetch-hamlib-unix.sh`, `scripts/macos-system-imports.txt` | Build and licence tooling. `THIRD-PARTY.txt` was regenerated for the smaller graph (529 crates). |
| `scripts/xnexus/**` (build, guarded packaging, NSIS template and guards, installer acceptance, Linux validation, fork policy) | Building and installing the fork on Windows. |
| `.github/workflows/*` guards and `xnexus-ci.yml` | Inherited jobs stay disabled outside kd9taw/Nexus; fork CI runs the policy test, UI build and desktop checks. |
| `.xnexus-hooks/pre-push`, `.gitignore`, `AGENTS.md`, `README.md`, `NOTICE`, `CHANGELOG.md` | Fork workflow and attribution, reworded for full Nexus. |
| `docs/xlog/README.md`, `UPSTREAM.json`, `WINDOWS_DISTRIBUTION_HARDENING.md`, `MACOS_SYMBOL_REVIEW.md` + evidence | How the fork works beside xLog; documents still cited by NOTICE and the tooling. |

## Verification at this checkpoint

Windows 10, GNU toolchain `1.93.1-x86_64-pc-windows-gnu`, MSYS2 UCRT64, Ninja,
`CARGO_BUILD_JOBS=4`.

| Gate | Result |
|---|---|
| `cargo check --locked --manifest-path src-tauri/Cargo.toml --features radio --all-targets` | Pass (expected AI CW and Hamlib resource warnings only). |
| `cargo fmt --all --check`, `cargo fmt --manifest-path src-tauri/Cargo.toml --check` | Pass (two fork files formatted in this commit). |
| `python scripts/gen-rust-licenses.py --check --offline` | Pass, 529 crates. |
| `node --test scripts/xnexus/fork-policy.test.mjs` | 6/6 pass. |
| `ui`: `tsc -b` | Pass. |
| `ui`: `vitest run --maxWorkers=3` (full suite) | 1580 files, 23077 pass, 21 fail in 12 files, all upstream UI that this change leaves byte-identical: Windows paths (`file-accept` `C:\C:\`, `StopLayer` and `noWholeLog` with `\` separators), jsdom without `navigator` or `crypto.subtle` (`SettingsPanel.rttyauto`, `sstvIdInImage`, `SstvView`), and timing (`Logbook.exportRows` ResizeObserver loop, `PhoneScope` draw frames, `Js8Cockpit.lists`, `RadioProgView.map`, `Toasts.action`). The suite caught two CS/DS faults first, both fixed here: the row's colour class must stay right after `decode-row`, and a row's class list must stay upstream's; the CS and DS tracks therefore live in `.decode-row`'s own grid, with no extra row class. The CS/DS, decode, roster, need-chip and park tests pass. |
| `cargo test -p tempo-core` | 1981 pass, 4 fail: three Linux AppImage environment tests and one database path name, on Windows. The crate differs from xlog.16 only by `cargo fmt`. |
| `cargo test -p tempo-app` | 3986 pass, 10 fail: Windows paths (`upload_start`, NotADirectory), journal sync, panadapter, scoreboard and claim timing under load. The crate is byte-identical to xlog.16. |
| `cargo test --manifest-path src-tauri/Cargo.toml --lib --features radio` | The raw harness cannot start on Windows (no Common-Controls v6 manifest, `0xc0000139`); run as `scripts/xnexus/test-windows-native.ps1` prepares it, on a manifested copy. 1934 pass, 37 fail. 14 pass when run alone. Of the other 23, 13 fail on an xlog.16 (`124c2953c`) harness built the same way: Icom keychain naming (the fork's own identity), remote stop-road lock, account grants, rotator (Mac serial paths), stream credentials (mock-runtime ACL) and DXKeeper queue. The remaining 10 remote stop-road UDP timing tests are flaky on both builds: run side by side, each build failed some the other passed. src-tauri differs from xlog.16 only by deleted companion modules and their declarations. |

No failure is caused by this change. Package, install and live check: below.
