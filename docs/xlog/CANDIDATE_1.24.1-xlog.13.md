# 1.24.1-xlog.13 explicit operator TX testing candidate

Pre-version integrated source: `d290d5bfd510e66d9ec99ea47d82c84a5f7481c7`.
It contains UI feature `4074910495377a08283db8f14f6c3917bf2970c6` and
native feature `aa400d8da2f2e17a8aabd391ce2b534150fd2a12` (canonical cherry-pick
`d290d5bfd510e66d9ec99ea47d82c84a5f7481c7`). Upstream provenance remains
`c88740a94172b3e78ae587d62bd0a54394d1e85f`.

This commit changes only the five version surfaces and adds this record.
The external package proposal must bind the resulting exact clean commit before
building from canonical `C:/Projects/OM0RX-xNexus`; no self-commit hash is guessed.
Matching owner candidate: xLog `1.6.54`, source
`2085c521dec5901babb6b9d5699eb5edb44e90c4`, including the owner needs-query
correction. Its packaging and installation are separate receipts.

Original Nexus TX controls dispatch explicit operator actions while paired and
capable, independently of cached status expiry. Native dispatch reads current
authenticated owner context. Receive consumption cannot cancel an enable while
the SDK retires the previous lease and publishes its replacement. Work may
select a retained message after a stream repair on the same physical station;
it does not replay that old decoded period or retarget pending output. Genuine
station/source changes and independent exact Stop/release handling remain.

Focused qualification before this version-only commit:

- 18 frontend cases passed; result SHA256 `8877D20C151EA9B9840C4BA8E86B9FF84B47AB6205852AAA7536851E78B4FFC8`.
- Three actual native-module/fake-owner cases passed: retained Work and physical
  replacement negatives, enable-publication race, and explicit intent after
  fresh-unavailable or expired cached context. Receipt SHA256
  `DAAE9EC9330B56BA9A8989128A3B07B15DC1052F24F80FAD6B390917BE3C50C6`;
  guard `34E61E3FFFF68584269EA8687FD168B9ED2D9846D8273CA38185C0F8B9D35EA3`.
  Fresh SDK/native compilation, 2 GiB/300 seconds, natural exit/final zero;
  source/tool/dependency pins unchanged. The prior three-case pass is retained.

Reuse the qualified canonical GNU 1.93.1 cache with release optimization 3,
unchanged panic unwinding, jobs 1, `CARGO_PROFILE_RELEASE_LTO=false` and
`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`. The unchanged calibrated supervisor
has a 7 GiB aggregate cap, 14 GiB physical and 13.25 GiB commit admission,
6 GiB physical/commit runtime floors and five-second natural drain. The `.12`
measured peak was 6,311,280,640 bytes; this is history, not a guarantee.

The actual package supplies final TypeScript/native integration checks, followed
by static artifact audit and independent retention. At this source-only
checkpoint, no broad CI campaign, installed acceptance, native geometry, live RF success or public release is
claimed. The completed build and installed observations are recorded below.

## Completed build, installation and bounded live observation

The canonical package built exact `326f6d658e5c2ba18f4072563d986e33363756a8`
with the unchanged profile above. TypeScript/Vite and native compilation passed;
optimized native compilation took 51m51s. The package guard retired naturally
at 2026-11-17 01:47:03 UTC: root/effective exit 0, final active 0, no forced
cleanup or resource flags. Peak aggregate memory was 6,310,494,208 bytes;
minimum available commit/physical memory was 7,915,892,736/13,792,772,096 bytes.
The two known NSIS `WixMode` warnings remain recorded, without a template edit.

Static artifact audit passed without launching the application. Independent,
non-overwriting retention completed at 01:50:46 UTC: 49 files, 739,156,867 bytes,
under `.build/xnexus-worktrees/release/target/retained-candidates/1.24.1-xlog.13`
in the xLog workspace. Earlier candidates and failed/held evidence remain intact.
The installer is 285,493,470 bytes; both executable variants are 181,239,099 bytes.
Their bundle markers differ, so the installed-payload hash is the installation
identity, not the standalone hash.

| Evidence/artifact | SHA-256 |
|---|---|
| Package receipt | `C25B5C9B239A4CB5F3DDA5C4B0E013A9E5C3FC5DA578C6EAAC7BB33454B10785` |
| Natural build guard | `921487B359FC0E4096262C3824C074202647F099004F97932C2F45FB60899CF6` |
| Static audit receipt | `E067D8B3102E7A2301B50831EB3EB3B4B4AE769FDA8B28DE0C8AA25A27BA211A` |
| Independent retention receipt | `CA32E7BEF4BEF808674862FCF6C935D1D1405A37DC8AFD55C38568A45A49CAA4` |
| Installer | `224AA3ED080E823145F4D2ABF102CBD93CFF5511C6FDD6C88E3692DCA8B255CD` |
| Installed payload executable | `BDACE8CE7125641DC0F2225F6AAE2D1EE7D506DD74CFC5BF6F67C0ACA8A169AB` |
| Standalone executable | `4C4DD3BFED86E2210D24A650B57335BF95487760A89E31951AA26FF6146F3F2D` |
| Actual owned CLI/module receipt | `20EFE45FF279EEFF2F682C501B8CA8BFAD412ADF1B2EA15DA2A91F59D7D951CE` |
| Lossless CLI audit view | `9998F9B5A972D6845272A27B1AA52131E97091369F780C7464AB0B968A97EC0D` |

The current-user installers for xLog 1.6.54 and Nexus `.13` each exited 0;
installed versions, registry locations and executable hashes were verified by
01:53:15 UTC. The installation workflow forced no closure, launched no
application and issued no radio command. Installation
receipt SHA-256:
`3574B8F404B6FD5F56B0AA64B519C51902FC8AE88F39DF607BB0A6929D7724DC`.
The receipt is at `.build/releases/1.6.54/install-20261117-025212/installation.json`
in the xLog workspace. This later installation does not rewrite the retained
build/audit receipts' truthful uninstalled-at-creation state.

Both applications were subsequently opened normally. At 01:58:11 UTC, process
inspection confirmed one established Nexus-to-owner loopback connection;
`paired-connection.json` SHA-256:
`4CD0535ECF7050C3E6CE978E959365D9353B476C5FB0D6769F9A586B26BB74F3`.
No credentials were recorded and no control command was issued.

With Nexus running, four read-only observations from 01:56:59 to 01:57:01 UTC
queried the same 32-call batch. The first two returned HTTP 503 pending; the next
two returned HTTP 200 ready, each with 19 needed and 13 worked results, zero
unknown. All four station snapshots reported needs metadata ready, and all four
TX contexts reported ready with no refusal reason. The bounded observation is
`active-nexus-needs32-and-tx-context.json` in the same installation directory,
SHA-256 `B50B10E805270BBD3142A21727AFCC93D38FE5CE1B1A9741A584F5F0AADA2753`.

This proves installed identity, an active local connection and successful owner
classification responses under the running Nexus load during that interval.
It does not establish what the native GUI displayed, a successful checkbox or
Work gesture, physical PTT/audio output, on-air FT8/FT4 operation, or a long soak.
Those operator observations remain pending; no GUI automation or RF command was
used for this checkpoint. The broader project and platform qualification remain
incomplete, and this is not a public release.
