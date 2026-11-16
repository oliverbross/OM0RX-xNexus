# 1.24.1-xlog.12 dedicated TX freshness testing candidate

Pre-version feature source: `5e38607cc7f20b511bf6c9100789fa0e2ff0944c`,
based on installed `.11` source `9c49a33d6fde31e8d93f0721f5cec95e2a14a14d`.
This commit advances the five version surfaces only and adds this record.
The exact resulting clean commit must be bound in the external package proposal
before building from canonical `C:/Projects/OM0RX-xNexus`.

The original FT controls use the dedicated, current xLog TX context independently
of the ordinary station-snapshot deadline. Expired TX authority still disables
new TX actions; Stop remains available independently. Native dispatch validation
is unchanged. Obsolete instructions to enable a separate TX service, confirm a
route or import a pairing file are removed from these refusal messages.

One focused mounted-host regression passed: expired station snapshot with fresh
TX context permits the enable gesture; subsequent TX-context expiry disables
new transmit actions while retaining Stop. The guard exited naturally with zero
remaining processes; result SHA256
`FAAE440E3EAFEBA7599C3DC656FD4BD96A4A2F1F4F08A3220055B8977D264A8A`.
Seven other tests in that file were unselected, not rerun.

A separate read-only probe linked the exact packaged `.11` SDK and accepted
three current owner TX contexts without issuing enable, heartbeat, prepare,
commit or any device action. Receipt SHA256
`EE090891E6977490D56686C8D0B67908251BFE4420E1FC47C43752D331559E35`.
The first probe compile lacked the host proc-macro dependency path and failed
before any probe execution; that evidence remains preserved.

The live owner also returned unavailable needs metadata with no classification
authority. This Nexus change does not invent logbook facts or repair the owner
query service; the matching owner correction is tracked separately.

Reuse the qualified Windows GNU 1.93.1 canonical cache and testing profile:
release optimization 3, unchanged panic unwinding, jobs 1,
`CARGO_PROFILE_RELEASE_LTO=false`, `CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16`.
The selected supervisor retains its 7 GiB cap, 14 GiB physical/commit admission and
6 GiB runtime reserves. Package compilation supplies final TypeScript/native
checks. No broad upstream CI, native UI geometry, live TX/RF success, installation
or public release is claimed by this source checkpoint.
