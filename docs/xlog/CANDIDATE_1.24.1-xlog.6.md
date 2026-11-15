# Candidate 1.24.1-xlog.6 — original Nexus interface for testing

This unsigned Windows candidate is being prepared for hands-on testing of the
original Nexus interface with xLog-owned services. It is not a completed product
or public release. TX remains unfinished and disabled; existing physical-control,
live receive and owner integration limits remain explicit.

Exact pre-version source: `44ba79060974e44263a39e3afefdf5cb49ca0ced`.
Upstream source remains `c88740a94172b3e78ae587d62bd0a54394d1e85f`, version 1.24.0
([provenance](UPSTREAM.json)). This candidate commit changes only the five version
manifests and this record. Its resulting full commit SHA must be recorded in the
external freeze receipt, verified against clean canonical main and origin/main,
and bound to the build supervisor before compilation. No self-referential source
SHA is guessed here.

The production companion entry renders `OriginalHost` and the original Nexus
components. The tested manual-create branch and existing owner controls are
combined at the pre-version source. The later SDK archive branch and offline
final-output prototype commits `d5336b15e` / `81aaf90c7` are excluded; no new
features or transmit work are added to this testing candidate. Installed `.4`
and retained internal-only `.5` artifacts remain separate and unchanged.

## Existing source qualification

The [combined original UI record](COMBINED_ORIGINAL_UI_QUALIFICATION.md) binds:

- Strict TypeScript and production Vite: receipt `3744E518C93D319991AEC201C39988910AFFD4390D81AB1D825FE5404A34755A`.
- Catalog checks, 440 passed: `F3850BCBF38CE9F76D88C416B7614BAA78DDB9079A51968411599646EF8F3F79`.
- Focused integration, 55 passed: `06EDFC2359BF220B9213DCD346B10BE4427366EE66EDC97ECEE9CC1845013DAA`.
- Original-component manual browser, three viewports/six layouts: `77AF569D440F118025D84734E668E9DBD64D360CC78BB7986FCA1B4FEB6CCDA9`.

The [combined native check](COMBINED_NATIVE_CHECK_2026-11-15.md), receipt
`99B894A6C66CC7EB83D1D75650989A36BB37780A611AC5765D94815150F2186F`,
compiled actual normal and test metadata on the qualified native source, with
zero test execution. Native Rust source is unchanged by the subsequent UI merge
and this version commit. Previously qualified unchanged source is not rerun as
an unrelated test campaign. These are source checks, not `.6` installed acceptance.

## Canonical package and provenance

Package only from clean `C:\Projects\OM0RX-xNexus`, using the existing
`target/xnexus-windows` cache, GNU Rust 1.93.1, pinned Tauri CLI 2.11.5 and the
existing release optimization profile. Build `radio,custom-protocol`, NSIS x64,
unsigned, with Cargo/CMake jobs 1 and Node heap 2560 MiB. Reuse the retained
libvpx, Hamlib, model and notice inputs; the isolated NoticeCargoHome is used only
for the existing offline notice check, never as the compiler Cargo home.

The proposed serial supervisor retains the successful `.5` policy: 11 GiB total
Job cap, fresh 18 GiB physical AND commit admission, 6/6 GiB runtime floors,
BelowNormal, five-second natural descendant drain and no automatic retry or cap
increase. A separately reviewed bound proposal and fresh admission are required
before dispatch. Actual npm/Tauri process/module provenance is recorded during
the build. The prior `.5` elapsed time is not a promised `.6` completion time.

Expected output is `target/xnexus-windows/x86_64-pc-windows-gnu/release/bundle/nsis/OM0RX-xNexus_1.24.1-xlog.6_x64-setup.exe`.
After natural build success, perform the existing static artifact audit and
non-overwriting independent retention, binding source/version, installer and
standalone/payload executable hashes. The build receipt, audit and retention
record establish what was produced. No installer, application, device or RF
launch is claimed by this source-freeze record.
