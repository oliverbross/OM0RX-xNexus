# Fork patch inventory

The accepted upstream base is recorded in [UPSTREAM.json](UPSTREAM.json).
At this checkpoint it is `c88740a94172b3e78ae587d62bd0a54394d1e85f`.
Keep its ancestry and original attribution. This inventory identifies the seams
that an upstream merge must preserve; it is not a claim that every upstream
feature is available in connected mode.

| Area | Fork-owned code or modified upstream seam | Merge review obligation |
|---|---|---|
| Companion startup | `src-tauri/src/lib.rs`, `xlog_mode.rs` | Default companion branch returns before local station, logbook, feed and hosted-service startup. Independent mode requires an explicit restart argument. |
| Native API and credentials | `integrations/xlog`, companion invoke allowlist | Preserve typed capabilities, identity/freshness checks, native-only pairing and independent Stop. No generic owner RPC or browser token access. |
| Logbook writes | SDK outbox and companion queue commands | Preserve durable request identities, workspace binding, uncertain-outcome receipt recovery and pending-write classification fence. |
| Receive processing | `integrations/xlog-rx`, `src-tauri/src/xlog_decode.rs`, `xlog_rx_worker.rs`, `xlog_decoded_needs.rs` | Consume xLog PCM only; retain route/epoch/period identity, bounded lanes, literal-sender parsing and revocable owner classifications. |
| Companion UI | `ui/src/xlog`, `ui/xlog.html`, `ui/vite.config.ts`, build-ID declaration | Keep the separate entry and owner-provided facts. Expired or unavailable authority is Unknown, never inferred as unworked. |
| Reused band map | `ui/src/components/BandMap.tsx` | Preserve optional external call menu/title, Digital label and explicit action callbacks; connected selections must not invoke local station commands. |
| Private storage | Companion window/profile-store modules; `lib.rs`, `street_map.rs`, remote vault and test-root helpers | Preserve explicit WebView builder storage setters, fork namespaces and tests. Configuration fields alone did not isolate the locked Tauri WebView. |
| Product/update identity | Tauri config/manifests/capabilities, `lib.rs`, `client_mode.rs` | Preserve product ID, executable/data roots, removed updater plugin and disabled inherited update paths. Advance candidate versions together. |
| Native portability | `crates/tempo-fast-sys/build.rs` | Preserve ordinary compiler-path normalization, static GNU Windows runtimes and bounded CMake jobs. Verify actual executable imports after packaging. |
| Mac native inputs | `scripts/fetch-hamlib-unix.sh`, `scripts/macos-system-imports.txt` | Preserve bounded build-job override and reviewed symbol allowlist; an allowlist is not a substitute for auditing the produced bundle. |
| Packaging/acceptance | `scripts/xnexus`, `tauri.distribution.conf.json` | Preserve pinned assets/CLI, exact materialized bundle config, ownership/payload guards, resource bounds and independent artifact receipts. |
| CI and publishing | `.github/workflows`, `.xnexus-hooks` | Every inherited job stays restricted to the exact upstream repository; only reviewed fork validation may run here. Keep inherited release/deployment workflows disabled. |
| Attribution and maintenance | `NOTICE`, `README.md`, `AGENTS.md`, `docs/xlog` | Retain upstream licenses/history, clearly distinguish fork behavior, and update evidence/remaining gates without rewriting failed results as passes. |

For an exact current inventory, run `git diff --name-status <recorded-upstream-SHA> HEAD`.
The table groups changes rather than duplicating a generated file list. Review
new modifications outside these seams explicitly. Dependency locks and the five
application version files must remain coherent.

Use `scripts/xnexus/update-upstream.ps1` for a report-only fetch and
`-PrepareMerge` for an isolated review checkout. Run fork policy, SDK, relevant
UI/native checks and applicable upstream gates on the merged source. Package
only after promotion to a clean canonical checkout and a unique version/source
receipt. Source tests do not qualify GUI, installers or physical station work.

The later `3885df0` update is preserved on review branch
`codex/upstream-3885-alias-review`, commit `50a5f1840b32bda65a838118f11468a0163f05fc`.
Its serial-alias fix and 19 targeted passing tests are documented in
[the review](UPSTREAM_REVIEW_2026-11-14.md). That branch is not merged into the
accepted base; full incoming-update gates remain pending.
