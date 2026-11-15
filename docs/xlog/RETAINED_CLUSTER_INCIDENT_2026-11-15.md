# Retained cluster reply shape defect — 15 November 2026

A read-only installed observation showed fresh station/rotator state but zero
Needed/cluster rows. Its private receipt is retained under the xLog workspace at
`.build/xnexus-native-acceptance/installed-station-observation-20261115.json`,
SHA-256 `4F5461F850DCC43D6C708D2899FAC6C2453059BEB344386489F033B67203A1F3`.
An empty display alone is not proof of a defect: owner filters can legitimately
produce no rows. The following source mismatch independently establishes one.

In xLog, `ClusterSession::accept_at_generation` accepts the worker RPC's `result`
payload and stores that payload as `Projection.response`. `projection_value()`
emits it without another wrapper: retained data is at `response.snapshot` and
`response.needs`. The companion host/projector instead read
`response.result.snapshot` and `response.result.needs`. This extra level yields
empty spots, unavailable needs metadata and missing contact-mutation fence
revisions, including when the actual retained owner payload is nonempty.
The native cluster poller is independent of renderer tab selection.

The affected `companion_host.rs`, `companion_projection.rs` and `cluster_session.rs`
are unchanged between installed 1.6.28 source
`eb3cf7024f7355c16346282646c7d2d62fb6bb7c` and frozen 1.6.29 source
`1ff361bfa4f0d1f92c1f1f0323f9dcd166f1da44`. Ordinary worker RPC `result` reads are
correct and must not be globally removed.

Handwritten projector/publication envelopes repeated the extra nesting and
masked the producer-consumer mismatch. The earlier 23 Swift-export replay tests
establish classification mapping within that supplied envelope, **not native
retained-owner wiring**. See [the corrected evidence scope](CONTACT_PUBLICATION_VERIFICATION.md).
Those passed historical tests remain preserved; they are not live propagation proof.

Ignored review copies and a proposed actual-ClusterSession regression are at
`.build/xnexus-cluster-shape-review/REVIEW.md`. Source review and patch preparation
have completed. The actual producer regression reproduced the failure and passed
after the correction. The published implementation is
`4025ee6f057e811343a506ea131bba9acd1ea312`. The test feeds a nonempty synthetic worker reply through
the real producer, metadata/projector and mutation-fence readers.

The baseline test exited 101 with missing classification/logbook revisions:
`None/None` instead of `Some(5)/Some(40)`. Baseline log SHA-256:
`2B396BB7AFB54C1AE2C68A6F0BC9661FA57EF8B988DECD620BB5F31F1F8DDB76`.
Its expected-failure driver exited 0, but a sole retained VCTIP helper caused
guard exit 125; cleanup confirmed zero processes. Preserve those distinct results.

The green test passed at 05:18:22 UTC with test/guard exits 0, zero final processes,
39 ms natural drain, no forced termination and peak 2,711,261,184 bytes. Evidence:
`.build/xnexus-cluster-shape-review/green-20261115-051709/tests.json` and `guard.json`;
test log SHA-256 `15AC26EDE4CE44565B9E839682F5F121AA0235F114A8839349F9DE590FC02966`.
This is actual producer-to-consumer regression evidence, not installed propagation.

Related native suites passed 20 cluster and nine projection tests. The bridge
suites passed 116 + 11 tests, and replay passed all 23 publication and six
classification cases with original input hashes unchanged. Evidence under
`.build/xnexus-cluster-shape-review/affected-20261115-051856` and
`bridge-20261115-052003` reports exit 0, zero final processes, natural drains of
84 ms and 28 ms respectively, and no force. The first bridge attempt at
`bridge-20261115-051736` passed its checks/root exit but retained a sole VCTIP
helper, producing guard exit 125 before confirmed cleanup to zero. That failure
remains preserved; the same-source cached rerun completed cleanly.

The frozen 1.6.29 optimized native compile completed in 18m31s with root exit 0.
Its sole retained VCTIP helper caused lifecycle exit 125; cleanup confirmed zero
processes. The candidate is superseded and was never packaged or installed.
The actual baseline, corrected regression, related tests and export replay are
now recorded above. Clean, published xLog 1.6.30
candidate `8795606bdbb5ac029287518a826f189d8873d117` combines receive recovery and
cluster-shape corrections; its parent is implementation `4025ee6f`. The exact
remote main tip and that ancestry were verified. It was subsequently packaged
and installed at 05:56:28 UTC on 15 November, replacing 1.6.28. No xNexus runtime
edit is part of this record. Live cluster/needs and receive qualification remain open.

## Installed 1.6.30 checkpoint

Package evidence is retained under the xLog workspace at
`.build/releases/1.6.30/package-run-20261115-052240-0067855e`.
Worker, Portable and Installer completed naturally with exit 0 and zero final
processes. Host compilation passed in 15m16s/root exit 0; the exact signed sole
VCTIP helper caused guard exit 125 and cleanup to zero. The separately reviewed
Host qualification preserves that lifecycle failure rather than relabelling it.

The current-user installer is 60,813,293 bytes, SHA-256
`9cfd48da1af7d0c954ea3617f80af9d2c03d075ee21bf043635b9b06fcfa6f0a`.
Payload manifest SHA-256:
`7079d12518f7e618e6645d0c2eb2e0af9d2ad6ea8b080daf43dddc86377fd231`.
The installation verified 3,254 payload files and preserved all 11 profile files.
Receipt `.build/releases/1.6.30/acceptance/installed-verification.json` has SHA-256
`FF04FF58DC2BBCC1C10BBEAD163ECA6D03BF8FFBB37537C61724260EFE33443C`.
The private candidate and installed marked launches/ordinary closes passed.
Installed marked receipt SHA-256:
`6BD2AF46B483ACBED1A140848B889EDB3481C2C18EBFB7874B03519112BB3915`.
Normal installed handoff then passed 36.5624911 seconds/13 samples with host
PID 6540 and worker PID 24672, diagnostics off and stable FLEX/DCU connections;
xLog was left open. Receipt `normal-handoff.json` in that acceptance directory
has SHA-256 `DCB5CF2266801907212F077B7A2976F9E734F124CF53100D627DF68283963D05`.
The operator has been asked to complete new 1.6.30 pairing/sharing/FT8 receiving.
That confirmation and live cluster/RX/needs qualification remain pending.
