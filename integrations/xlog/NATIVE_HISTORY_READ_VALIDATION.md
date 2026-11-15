# Native history read-lane validation

15 November 2026. This record concerns native source qualification, not a new
package, installed application or live station result. The SDK and producer
replays are recorded separately in [HISTORY_READ_VALIDATION.md](HISTORY_READ_VALIDATION.md).

The native lane retains one independent `HistoryClient` per connection, outside
the ordinary station-client mutex. Connection retirement revokes its atomic
fence immediately, even while HTTP is pending or a response retains the old
connection. Reads check the exact connection token and generation before and
after HTTP. Serialization checks both the retained lease and revocation fence
before and after writing the payload; the frontend must still reject late IPC
using its request-start deadline and connection/session token. Busy, unsupported
and expired reads are unavailable, never an invented empty contact list. Legacy
`xlog_history` remains unchanged, and these reads do not depend on RX context.

## Full native check

The frozen validation source is base
`7e36f9e7a6e792f8f845ed1e2ea4aef09635e38f` plus the reviewed SDK/API and native
history deltas. The 5,226-file manifest also retains the existing identity,
waterfall and current-context implementation. Source, tools and 1,495 retained
evidence/asset files were verified before and after the run.

Pinned GNU Rust 1.93.1 `cargo check --locked --offline --features radio
--all-targets -j1` passed. Both actual `tempo_lib` normal and test targets reported
`fresh=false`; cached dependency results alone were not counted as application
qualification. The 4 GiB Job used one Cargo job, BelowNormal priority, an
1,800-second deadline, 8/6 GiB physical/commit admission, 2/2 GiB runtime floors
and five-second natural drain. Peak Job memory was 3,667,984,384 bytes. Root and
effective exit were zero, all owned processes retired, and no forced cleanup or
resource notification occurred.

Evidence paths are relative to the xLog workspace under
`.build/xnexus-worktrees/history-integrated/target/history-full-native/`.

| Evidence | SHA-256 |
| --- | --- |
| `check-5f81b15292134020b5f0022673eadbb8/receipt.json` | `69E860819DF1EDBED291B1AEEA3755842BB77D7BBD5385585D26E0BF602EF364` |
| `check-5f81b15292134020b5f0022673eadbb8/guard/guard.json` | `C9388FE0FCFBA9DE5D629EB4BB28F13F659B1026188DB1A44E26EE0E0E595E55` |
| `check-5f81b15292134020b5f0022673eadbb8/checks.json` | `4878A6B2E5C824C869C308C793C5B6069F0BC1F5B1C63E2AFD6FF89D9F42FDC2` |

## Selected native tests

The actual Tauri library-test target compiled and linked afresh (`fresh=false`).
The runner copied only that executable into its evidence directory, embedded
the pinned Common-Controls manifest into the copy, and preserved the original
Cargo executable. It enumerated exactly four connection/private-root tests and
eight passive-history tests before execution. All 12 passed in 1.21 seconds,
with zero ignored tests and 1,991 unrelated tests filtered out. The tests include
the native connection generation/retirement seam, retained-response serialization
after disconnect, a retained SDK handle across revision changes, authentication
revocation, actual lease expiry, unavailable/unsupported reads and a stalled read
that does not block station polling or atomic retirement.

The separately reviewed test profile used an 8 GiB aggregate Job cap, 14/14 GiB
physical/commit admission and 4/4 GiB runtime floors, preserving two GiB of
initial margin in both budgets. Cargo remained at one job and BelowNormal,
with the same 1,800-second deadline, five-second natural drain, pinned toolchain
and test-debug settings. Compilation took 5 minutes 39 seconds. Peak Job memory
was 5,430,247,424 bytes; minimum host physical/commit availability was
16,842,567,680 / 13,961,457,664 bytes. Root/effective exit and final active process
count were zero; natural drain took 43 ms, with no memory/abnormal notification
or forced cleanup. All source and retained-evidence pins remained unchanged.

| Evidence | SHA-256 |
| --- | --- |
| `test-fc12ec40e352451db9cb05679503c006/receipt.json` | `0CD14FFFFD904CBFA34A2D56F969ED9725E502C5ADE9FA149AAAFBF33968FC90` |
| `test-fc12ec40e352451db9cb05679503c006/guard/guard.json` | `88301DCBA8187AB26795291628B85668521908C2B41F9226A0A62C4E41E86B49` |
| `test-fc12ec40e352451db9cb05679503c006/checks.json` | `C737E0CFB98477A79CDFD4818FA44CF8005C5EDB9F29F3A9CFF0C78E27AD056B` |
| `test-fc12ec40e352451db9cb05679503c006/tests.log` | `804BC332185D0D1BE8CB35572F28C98531BCB9C1FDA89B1968F1D11DDBD830F4` |

Original generated test executable SHA-256:
`D0C2A8CDA1A9F9EC09772C623825EC296CCC8B4AC217CE511BF52B0DAA5AC8BD`.
Manifest-prepared test copy SHA-256:
`E91D36BDACB6AEC4E178D254C2FEB2F9A6F4A433E64ADA9C4DCAB13EF0EEEA5D`.

## Preserved environment failures

The first all-target check stopped in `windres` preprocessing. An exact-resource
probe showed failure with Rust's tool directory before MSYS2 in `PATH`, and
success with MSYS2 first. Merely adding MSYS2's Unix tools did not fix it. The
corrected environment retained absolute Rust tool pins and added pinned Git for
the final source checks; no application source workaround was introduced.

The first linked-test attempt stalled in CMake's Fortran ABI/Ninja probe before
any selected tests executed. After verified zero CPU/child/file progress, only
the explicitly identified owned Ninja/CMake processes were stopped for
diagnosis. Its failed receipt remains
`test-6abf48716bd84b2f9c65f35ad96ab4e0/receipt.json`, SHA-256
`1A6711ADAE14081292F005AC7B991978A163B55F81723FE4B2F04B78267D1B0E`.
The wrapper then retired with exit 101 and zero owned processes. Natural wrapper
cleanup does not erase those explicit diagnostic process stops or qualify tests.

A separate identical tiny Fortran/C/C++ tool probe timed out inside the execution
sandbox but passed outside it in 7.2 seconds with the same source, tools, path,
1 GiB cap and 120-second deadline. The sandbox-specific mechanism is unknown.
The comparison record is `../history-cmake-probe/permission-comparison.json`,
SHA-256 `AB23AE4FB7B01DDF6A9AD5ABA2DAABFAA62FF821F52B764615A28081EEE4059E`.
This is tool diagnosis only; it is not application-test evidence.

The subsequent unchanged application Test attempt outside the sandbox passed
the prior tool stall, then reached the 4 GiB aggregate Job limit during native
test compilation. The guard observed a job-memory notification and stopped its
owned process tree; final active processes were zero and all source/retained
pins were unchanged. No selected tests executed. The failed receipt is
`test-532dbcd300084a0f9d86060c7677d237/receipt.json`, SHA-256
`6D15F6761EA5DA861F84187084235A95A915F139CC2D7280D64FA30F6344EFC5`;
the guard hash is
`7F3C01B7AE6B3945B73CA7108D6686711A41929D215E4C9740B2C6805B3C3C8F`.
Observed peak Job memory was 4,301,172,736 bytes. This failed attempt does not
establish a linked-test pass; the later separately approved 8 GiB result above
is the successful linked-test evidence.

## Scope limits

The debug native gates leave `custom-protocol` disabled. They do not qualify
frontend embedding, packaging, installation or application startup. No GUI,
production database, hardware device or RF operation is used. Current original
UI integration and eventual native lifecycle acceptance remain separate gates.
