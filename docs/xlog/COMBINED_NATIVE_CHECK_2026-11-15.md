# Combined native-shell check — 15 November 2026

Exact clean source `a0a804d7786b3820bf72f3378b1df9daf3bbe8ad` passed the Windows
GNU native-shell normal-and-test metadata check. This documentation successor
has the same runtime bytes as `3487e4e2e7148398c9085e19ed17f5e04e314653`, including
the [independent bearing lane](BEARING_NATIVE_DRAFT.md) and
[actual decode-period preservation](NATIVE_DECODE_PERIOD_VALIDATION.md).
The original contact-reader/editor host and current SDK are included.

This was `cargo check --all-targets`, not a linked application build. Both actual
`src-tauri/src/lib.rs` / `tempo_lib` artifacts were reported **fresh=false**,
one normal and one test-profile artifact. Cargo completed in 4m07s. It executed
zero tests and launched no app, frontend build, GUI, installer, device or network
endpoint. The separate focused 32 bearing/contact and 44 decoder/worker runtime
tests remain their own evidence, not a combined test count from this check.

## Exact invocation and retained evidence

```text
cargo check --locked --offline --manifest-path src-tauri/Cargo.toml --features radio -j1 --message-format=json-render-diagnostics --all-targets
```

GNU Rust 1.93.1, explicit pinned Program Files PowerShell, ucrt64-first PATH plus
explicit Git, the existing Cargo home offline, Ninja and one CMake/Cargo job were
used. The child script is byte-identical to the prior successful 2103 check
(`9706AA3F4341CF51F269DDC6E5AC26821AD1B3E7C9BEC31D3FEFAE71CB1697AD`).
No debug/optimization profile or custom-protocol feature was changed. Unused
receive helper/compatibility API and newly retained native fields produced
warnings; there were no compilation errors.

Evidence is retained under the native-decode-period worktree:
`target/combined-native-check-4b8386d257b94cc3a91476d48e42c117`.

| Evidence | SHA256 |
|---|---|
| Final receipt | `99B894A6C66CC7EB83D1D75650989A36BB37780A611AC5765D94815150F2186F` |
| Guard | `476A98D536C01E076689E21176C4FE2E4A99E0868F596E753F9F5699685D4164` |
| Actual normal/test artifact checks | `26D69F369A6E4202B85A6E77273CF25D00F546A3D70092CC60625A0A8392D811` |
| Cargo artifact inventory | `C7FAF7BB44DB5CA5E043B7E685DA64EF951C4C41DDF67974A7B6C429C7237E61` |
| Compiler log | `59125FB18CFB032A2F8A4B9085ECCC6C005726E491F20CABA9EFA8CB72D844AE` |
| Reviewed input manifest | `03DC2FD71D0163556E0198698E84035C8D9FE981FF96A008E1C4BE3A703E7767` |
| Successful staging receipt | `CE3B8E3D52B8C06F21EA3D87B5FB1FBB477C2E41D486EFE16021481B171821B1` |
| Narrow checkout-byte proof | `065043C9E18B39067172CE7F82DD628D9BF8CB3BE4E8992A2564BCE2E55D7DA5` |

## Resource and source preservation

This new integrated-source attempt used an explicitly reviewed **5 GiB** aggregate
Job cap, 1800-second deadline, fresh 8/6 GiB physical/commit admission, unchanged
2/2 GiB runtime floors, BelowNormal, jobs 1 and five-second natural drain. It was
not a retry of a failed 4 GiB check: the prior 2103 check had passed, with only about
30.8 MiB between its measured peak and that cap. Both receipt and real supervisor
invocation used the new 5 GiB limit; no automatic retry or enlargement occurred.

The guarded root and descendants exited naturally with process/effective 0,
final active 0, cleanup confirmed, no force or memory/abnormal-exit notifications
and 18 ms drain. Peak job memory was 4290822144 bytes. Minimum available physical
memory was 17070788608 bytes and commit 14875430912 bytes. Guard completion was
19:15:01.6385994Z; final preservation checks finished 19:15:11.2972239Z.

The isolated rx-desktop checkout moved from exact qualified 2103 to exact a0a804
using an ordinary non-forced detached switch. Before staging, all 5295 old source pins were verified. After staging and
compilation, all 5338 new source files and 1523 immutable assets/evidence plus
tools were verified; the old source manifests were retained. The target remained the same mutable incremental Cargo cache and VPX path;
root/desktop locks, desktop manifest, UI lock and Git attributes were unchanged.
Actual source recompiled; old cached application metadata was not accepted as
new evidence. Canonical source and retained release artifacts were untouched.

Canonical raw and checkout manifests were kept distinct. Only the same three
previously reviewed documentation files (AGENTS.md, BACKLOG.md, UPSTREAM.json)
had exact CRLF/Git-LF byte differences, with each pair's hashes, Git blob and
attributes recorded. There was no runtime, binary or test-fixture normalization
exception. All ten historical source copies and five moved files, the original
checkout failure, windres/sandbox/CMake failures and prior test-memory failures
remain retained. Staging made no moves, deletions, resets or forced checkout.
The initial preparation refusal when documentation advanced 3487 to a0a804 was
also preserved before source binding was explicitly updated.

This check qualifies integrated native command/type/serialization compilation.
It does not qualify a new package, linked native runtime, installed original UI,
paired hardware, owner-mediated live TX, physical Stop/timing, or RF. Installed
`.4` and internal-only `.5` artifacts are unchanged; no version was reserved.
