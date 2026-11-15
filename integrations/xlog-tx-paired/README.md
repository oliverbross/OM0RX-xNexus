# Actual-owner offline sequence replay

This acceptance-only crate joins the actual Nexus `Station`, single-period
preparer and offline sequence adapter to four separately checked-out xLog owner
modules. It is not an application dependency and enables no transmit path.
There is no device, backend, live clock, network endpoint or persistent journal.
`FakeSink` retains the actual immutable `PreparedOperation`; only its explicitly
recorded fake effect advances Nexus. Starting, Permit and ACK alone do not.

The library is compiled normally, with library tests and doctests disabled.
The sole integration target runs 18 cases, without compiling the owner's separate
42 unit tests. Missing inputs fail rather than skip tests. See
[VALIDATION.md](VALIDATION.md) for exact qualified inputs and evidence boundaries.

## Reproduce

Use a qualified owner checkout whose four files match the SHA256 pins in
[build.rs](build.rs). Set `XNEXUS_XLOG_SOURCE_ROOT` to its absolute repository root.
The build script canonicalizes the root and files, verifies every hash, and
generates path imports into `OUT_DIR`; it does not copy owner implementation.
The pinned paths are `prototype/src-tauri/src/{digital_tx_owner,output_authority,prepared_period,prepared_period_adapter}.rs`.

Set `XNEXUS_OFFLINE_CORPUS` to the absolute directory containing the retained
14-case corpus: `inventory.json`, fourteen case JSON manifests and fourteen
`.f32le` payloads. Inventory SHA256 must be
`B77DAE43D212848E65CA7D3116AF47A361B35B42825C490B8C6B7B6FCA9BA481`.
Each case separately verifies its manifest and PCM hashes against that inventory.
These are synthetic generated waveforms, not captured station audio.

From the Nexus repository root, with Rust 1.93.1 and the existing native DSP
prerequisites available:

```powershell
$env:XNEXUS_XLOG_SOURCE_ROOT = '/absolute/path/to/qualified-xlog-checkout'
$env:XNEXUS_OFFLINE_CORPUS = '/absolute/path/to/retained-corpus'
cargo +1.93.1 test --locked --offline --manifest-path integrations/xlog-tx-paired/Cargo.toml --test replay -- --test-threads=1
```

The Cargo recipe is the reproducible package interface; a full Cargo build of
this new crate is a separate gate from the direct-rustc qualification documented
below. The crate has an isolated lock seeded from existing qualified dependency
versions; it does not update the application lock. Use the platform's normal DSP
build prerequisites rather than downloading or silently replacing dependencies.

## Assertions and upstream updates

Fourteen cases cover CQ, grid, report, roger-report, RRR, RR73 and final 73 in FT8
and FT4. Each compares actual freshly prepared PCM and content identity with the
retained corpus, passes the exact raw manifest and PCM through owner validation,
constructs the actual typed prepared operation and serialized Body, commits via
actual Owner/Permit to the fake sink, and verifies exactly-once Nexus advancement
and exact retained release identity. Same-ID conflicting bodies and new-ID reuse
of a consumed slot are rejected. Four negative cases cover corrupt PCM, corrupt
manifest geometry, whitespace-sensitive raw-body identity/substitution and a
changed context at the fake effect boundary.

After an upstream/owner update, qualify the changed owner modules independently
before reviewing and updating the four source pins. Never bypass a pin to get a
green result. A changed preparer/corpus requires a separately reviewed export
with the actual exporter in
[xlog-tx-prepare](../xlog-tx-prepare/examples/export_prepared.rs), full raw
manifest/PCM retention and explicit provenance. Do not rewrite the old corpus or
replace expected assertions with freshly generated answers. Keep failed receipts
and old input hashes. A changed protocol or operating format needs its own review.

This fixture uses caller-supplied synthetic times, an in-memory journal and a fake
effect record. It does not qualify physical presentation timing, hardware Stop,
watchdogs, durable restart recovery, live authority, DAX TX format or RF output.
