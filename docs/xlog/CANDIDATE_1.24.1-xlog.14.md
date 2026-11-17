# 1.24.1-xlog.14 needs and TX prepare recovery candidate

Pre-version integrated source: `3eaa766e365a7a2d3782327873d7a33cbc5db0b0`.
Upstream provenance remains `c88740a94172b3e78ae587d62bd0a54394d1e85f`.
This commit changes only the five version surfaces and adds this record. The
external package proposal binds the resulting exact clean commit before building
from canonical `C:/Projects/OM0RX-xNexus`; no self-commit hash is guessed.
Paired owner: xLog `1.6.55`, exact source
`a8541523bbf3d3dc792c712408ee08a55a7a6d1f`. Its build and installation are separate.

The installed `.13` failed functional acceptance despite its successful build and
bounded owner API observations. Actual read-only CDP showed 200 Unknown receive
rows; a temporary needs-client destructor was retiring the shared authority.
Operator Work also failed: an earlier attempt retained an operation with no owner
receipt, and a later trace showed owner TX readiness alternating with an erroneous
one-second telemetry-age refusal. That later user attempt sent no prepare/commit
HTTP. These failures remain evidence, not successful on-air qualification.

This candidate includes the needs-client lifetime correction, visible native TX
failure reasons, and native/SDK prepare recovery from feature
`1fe4e5fe886c1a2be18c43cef15b85bf9766393b` (canonical `3eaa766e`). A fresh,
authenticated matching owner context admits the immutable body after construction;
existing original-lease semantics and once-only dispatch remain. Only proven
never-dispatched local plans are cleared. Dispatched transport uncertainty keeps
its exact release obligation and first failure reason. Work refreshes its current
coordinator before applying parity/action. Owner `.55` removes the demonstrated
incorrect envelope-age gate while retaining actual radio validity.

Focused qualification is deliberately limited: needs lifecycle red/green and ten
UI reason cases passed; actual SDK/native fake-owner preparation has four passing
cases, receipt `12D4D525318909E619CE8BB8D8F05545A7C8F6F2F8744DBD70D3BD39D1767D78`,
guard `807C6C516A300FFC3BB557FCB28108CA4CDF63001743EA799E1ABA7867C4B32B`.
Root/final exit was zero, natural drain 22 ms, all input hashes unchanged. The
first fourth-case failure and two mechanical old-behavior red variants are
preserved. [The SDK validation record](OWNER_TX_CLIENT.md) describes the exact
scope and redundant byte-identical RX-library rebuild; no full-suite claim is made.
Owner's nine focused route cases passed separately. No automated check issued RF.

Reuse the same qualified canonical GNU 1.93.1 cache, optimization 3, panic
unwinding, jobs 1, `CARGO_PROFILE_RELEASE_LTO=false` and
`CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`. The unchanged supervisor has a 7 GiB
aggregate cap, 14 GiB physical / 13.25 GiB commit admission, 6/6 GiB runtime
floors and a five-second natural drain. The prior `.13` peak 6,310,494,208 bytes
is historical evidence, not a new resource guarantee.

At this checkpoint `.14` has not been built or installed. The actual canonical
package supplies TypeScript/native integration checks, followed by current static
artifact audit and independent non-overwriting retention. Installation and normal
operator verification remain pending. No live TX, on-air success, complete product
acceptance or public release is claimed; earlier candidates and receipts remain.

## Package, installation and pending live acceptance

Canonical Nexus `.14` built exact source
`2dbbdb2d23718437f6f51473eeb32a54b497bcd3`; paired Owner `.55` uses
`a8541523bbf3d3dc792c712408ee08a55a7a6d1f`. TypeScript/Vite and optimized native
compilation passed; native compilation took 49m20s. The unchanged 7 GiB job cap,
14/13.25 GiB admission, 6/6 GiB runtime floors, jobs1/noLTO/codegen16 profile
retired naturally at 2026-11-17 06:07:42 UTC: root/effective0, final active0,
17ms drain, no forced cleanup or resource flags. Peak was 6,309,142,528 bytes.
The earlier no-child admission hold and unused 13.0 GiB proposal remain preserved;
the successful attempt used the original profile. Two known NSIS WixMode warnings
were retained without a template change.

Static artifact audit passed without launching the application. Independent,
non-overwriting retention contains 49 files / 739,198,633 bytes at
`.build/xnexus-worktrees/release/target/retained-candidates/1.24.1-xlog.14`
in the xLog workspace. Prior `.13` receipt and file metadata remained intact.

| Artifact/evidence | SHA256 |
| --- | --- |
| Installer, 285,518,141 bytes | `0F29A2230C276EAD5AEEFB4156E2B413BFE205F7D4502992D7D669FF4504CDC2` |
| Installed payload executable, 181,252,864 bytes | `8BB137D7D00BFE90DB32766B2E26B30A803A37409E8DC1E1A4F19596A207654A` |
| Standalone executable (different bundle marker) | `268946C30785D6CDFD97587B8CF10B4AD24E6C0124B4A4C5A83E2980F4BCB5EF` |
| Build receipt | `D5448A814AE45CD9102433C9573F36485CADEB03BC5F9753D8772E3A9DDA6DF7` |
| Natural build guard | `2FA4D319FDDF431E757A15CF8AFBB361021AEA2F3773D26E20D6BD608BA24FA7` |
| Static audit | `2A0CCF8136A5E8B0D51361CDE61342CD05FFD24572C4B85CAFF27262E3291B24` |
| Retention receipt | `962535FBDA4A4A228C1DBA43531130AC2AEA24E1AC63700EEBE140460B259C2D` |
| Original owned CLI/module evidence | `8765212D005EFB6DCD66DC52371E1D389D0CFEB21ADB35CE9856106AAE32BB3C` |
| Lossless CLI audit alias | `5941C8AB7842DCF3BFAAB9CAE97B532A6BD47F3EEF3523B7B987D98A7C41ACC7` |

Both current-user installers exited0 during 06:12:17–06:13:10 UTC, with initial
and final application process counts0. No forced closure, app autorun or radio
command occurred during installation. Registry/version and installed executable
hashes matched the expected payloads. Installation receipt:
`.build/releases/1.6.55/install-20261117-071217/installation.json`, SHA256
`F7FB7CE59B2B491BB6A3307BF293B3E32D11356EC160DA85C6845B001E48B64F`.
This later installation does not rewrite the retained artifacts' uninstalled-at-
creation receipts.

Root subsequently verified normal production launches: Owner18636/Worker23320,
Nexus24064/browser6896, with per-process read-only CDP9784 and no proxy or mouse
helper. Nexus paired automatically. At that observation the owner reported
`radio.connected=false`: FT waited for the saved FLEX slot and RX waited with
zero rows. Native/DOM classification and operator Work acceptance therefore
remain pending radio connection; the operator was asked only to use normal radio
Connect. No on-air success or complete functional acceptance is claimed.

## Live classification observation after operator Connect

On 2026-11-17, the operator connected FLEX. Nexus24064/browser6896 remained;
Owner restarted as21396/worker12212 and automatic re-pair required no new file.
At06:59:38 UTC native polling showed radio1 connected at10.136MHz DIGU; RX had
56 rows:28 needed,24 worked,4 unknown, with rendered NewBand/Confirmed/Worked/NewCall.
At07:02:03 UTC RX had200 rows:96 needed,92 worked,12 unknown; unknown reasons were
only `native_not_literal_sender` / `country_ambiguous`. Actual rendered labels
were populated. At07:04:20 UTC history remained200 rows and reception continued.
Evidence: `.build/nexus14-runtime-diagnostic/` process verification after user
Connect, read-status/read-poll and trace JSONs in the xLog workspace.
Native FT remained available in FT8, armedfalse/txcount0. The passive120-second
trace contained only the initial off event: no operator Work had occurred.
Classification/display acceptance is observed; TX/on-air acceptance remains
pending the operator's double-click. Installed application source remains `2dbbdb2d23718437f6f51473eeb32a54b497bcd3`.
