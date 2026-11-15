# Windows candidate 1.24.1-xlog.4 — build and artifact evidence

Completed 15 November 2026 UTC. This is an unsigned development
candidate, not a qualified station application or published release. Canonical
`main` stayed clean at `ae35fc6435f861e38029049299833afe57de4885` throughout
packaging and audit. The [candidate record](CANDIDATE_1.24.1-xlog.4.md) binds
the pre-version implementation, paired owner and remaining acceptance.
No real xNexus application or installer was launched during these checks.

## Source, tools and bounded execution

The upstream pin is `c88740a94172b3e78ae587d62bd0a54394d1e85f`. The canonical
helper used Rust `1.93.1-x86_64-pc-windows-gnu`, Tauri CLI `2.11.5`, one
Cargo/CMake job and `NODE_OPTIONS=--max-old-space-size=2560`. Release optimization
remained fat LTO, one codegen unit and stripped debug information. Native/model,
lockfile, distribution configuration, template, guard and license hashes are
bound by the build receipts. The frontend passed; Rust reported the optimized
release build complete in 74m40s.

One reviewed external supervisor ran in fresh PowerShell, without changing
application or canonical supervisor sources. The existing Job-limit parameter
set a **12 GiB aggregate cap**. Two local threshold-helper overrides required
**20 GiB physical AND available commit admission**, with **6 GiB physical/commit
runtime reserves**. Assignment preceded resume; BelowNormal priority, no
breakaway, the 12-hour timeout and no automatic retry/enlargement were retained.
The copied guard allowed a bounded five-second natural drain after root exit,
with retained descendant identity/exit handles, deadline-before-success checks
and refusal of nonzero exits or incomplete inventories. Owner fixtures covered
short-success, nonzero and expired-drain children; 49 lightweight Nexus checks
verified the stronger policy binding. The actual run required no child exception.

| Supervisor input | SHA-256 |
|---|---|
| External wrapper | `9C6873974728A2D83319AE408BBDB489E602950D033B92777BB3BD1B739F3C4F` |
| Memory profile | `9CFE8BEDCAE963A6B61C684BF8D8D6C56E9B43C94F2693E844E2DAF522083E48` |
| Copied `Guard.ps1` | `245AAD31CE64434FB9E005F2DFF202E5E345B8A3C4E439E18639052BF2542735` |
| Copied `NativeJob.cs` | `88E6C18FE1E21DEC7B95D64D6813CB2D06AF50F5C6F6108BB99748AF097B8A5F` |

Admission observed 22,668,156,928 physical bytes and 21,958,385,664 available
commit bytes. Peak owned-job memory was **10,072,150,016 bytes (9.38 GiB)**;
minimum host availability was 14,471,200,768 physical bytes and 11,463,974,912
commit bytes. The previous 4 GiB allowance would not have contained this build.
The run ended at `2026-11-15T00:38:54.0728642Z`: root exit 0, zero active
members, confirmed cleanup, no forced live cleanup and a 23ms final drain.

Actual CLI evidence captured node PID 14276, creation time, exact `.4`
distribution-config command and three-member ancestry to owned root PID 5740.
The loaded module was the canonical target's
`@tauri-apps/cli-win32-x64-msvc/cli.win32-x64-msvc.node`, SHA-256
`CF1426421FAFFEE1460CDE828A97D4DDEC387F276549280CC8530F7EA04124F8`.
The process receipt hash is
`ADE6F13E97C131804160C755668ECC29187B87B7472501D809203E9699354E04`.
This proves actual use, rather than merely a cached tool's presence.

## Artifacts, audit and retention

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Standalone executable | 114846348 | `44C2BE04218E643266A44A87185DB4969ABAB711B05EB5FC4E90EAFFE7C88ED8` |
| NSIS-embedded executable | 114846348 | `B8BBD58EF45DA82B06D9514E43A3DB3B151F0D60506EB45CFD153BFD4486A9BE` |
| `OM0RX-xNexus_1.24.1-xlog.4_x64-setup.exe` | 269004954 | `13F3173193FA17EC34685978C1E9218EF69D4A96ED57FE0B1E360C56F46C3A31` |

The static audit passed at `2026-11-15T00:41:58.9325201Z`: product/version,
Common-Controls v6, imports without builder-only GNU DLLs, actual CLI provenance,
exact generated NSIS/distribution inputs, embedded assets/licenses and unchanged
artifact hashes. Standalone and embedded executables differ only at the supported
three-byte `UNK`→`NSS` bundle marker (offset 86597024). The embedded offline
WebView2 installer matches SHA-256
`D3CBE8FEDB104282D745FFF106993EFEAD51AE94F5595A4E75804446A974F9C3`
and has a valid Microsoft Corporation Authenticode signature. The xNexus
application and installer themselves are **not signed**.

Independent retention completed at `2026-11-15T00:42:51.0085709Z` under:

```text
C:\Projects\OM0RX-xLog\.build\xnexus-worktrees\release\target\retained-candidates\1.24.1-xlog.4
```

The `installer`, `standalone` and `installer-payload` directories preserve the
three artifacts. All 60 copied files (540,569,234 bytes) passed size and
before/after SHA-256 checks, including build/distribution/audit receipts,
generated NSIS, actual supervisor, CLI/process/memory evidence and warning fixture.
The retained standalone application and installer each have only their own
hardlink path. All 13 retained `.2` entries were reverified unchanged before
and after copying; their rejected-isolation status remains unchanged.

| Retained receipt | SHA-256 |
|---|---|
| `retention-receipt.json` | `E52719399000A6E8CE112A60FD8C7AB9A85642315BD587BC45F211303DA2E1B6` |
| `provenance/1.24.1-xlog.4.json` | `F35A89AF8E38600906C48600D45AF60AB3DF4E7D5D5CDE0FB3DA61B0868E7BD4` |
| `audit/receipt.json` | `0786AFF5B3F7AB70E302280F6535B5B7B2FA9F028075D04DF0D05EEA0AE7A9AB` |
| `guard/guard.json` | `4923C869DB241445E2FE7587F427ECDE971729A81F2A1901403D202559494821` |

Original evidence remains at
`C:\Projects\OM0RX-xLog\.build\xnexus-windows-guard-evidence\20261114T2317295554205Z-3597017b7b654b058f042c240390cc42`.
The external proposal and its earlier superseded proposal are also preserved.

## Warning qualification and remaining gates

NSIS emitted two warning 6000 messages: the fork removed WiX migration state but
left `$WixMode = 0` wrappers around two shortcut early-return conditions.
[`=` is integer comparison](https://nsis.sourceforge.io/LogicLib), not string
equality. An inert fixture copied both exact generated branches and ran them
with the pinned compiler/LogicLib. All four UpdateMode/NoShortcutMode combinations
matched the intended WiX-free branch: `00` permits creation; `01`, `10`, `11`
return early. Explicit empty and nonnumeric operands also compared equal to zero.
The fixture reproduced both warnings, exited 0 with zero owned processes and no
forced cleanup, and peaked at 2,658,304 bytes under a 256 MiB Job. It wrote only
fixture results, without real shortcuts, registry entries or product dispatch.
Its retained `guard/shortcut-warning-fixture/receipt.json` has SHA-256
`4792B6BC4C4683C2F3296BA467FCF6292DD04B176CC624C7A62D477E3D46FE0B`.

The warning is harmless **on this tested toolchain** for these branches. Remove
the obsolete wrappers and add a regression invariant in a future source change;
changed packaged source requires a new version. This fixture does not qualify
a real installation or another NSIS toolchain.

[Subsequent disposable installer acceptance](WINDOWS_INSTALLER_1.24.1-xlog.4.md)
passed, followed by permanent installation and native A→B→A private
profile/lifecycle acceptance. Subsequent normal installed launch passed and left
xNexus open, still unpaired; paired UI acceptance remains pending. Updated Linux/macOS
checks and packages, physical DAX reception, clock/route qualification, signing
and distribution remain separate. Digital TX is neither implemented nor enabled.
The xLog owner now reports 1.6.28 packages, installation and normal handoff passed;
documentation is `92d6547` over application `eb3cf702`. The unchanged xNexus `.4`
paired live with 1.6.27, then encountered its owner clock error. The correction
is installed in 1.6.28, but the new manual receive outcome remains pending. See
[the incident evidence](RX_CLOCK_INCIDENT_2026-11-15.md). No successful physical
decoding or live station qualification is implied.
