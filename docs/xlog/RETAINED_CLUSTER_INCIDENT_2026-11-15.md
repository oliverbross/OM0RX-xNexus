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
have completed, but no canonical correction or compiled failing-then-passing
regression is claimed yet. The planned test feeds a nonempty synthetic worker
reply through the real producer, metadata/projector and mutation-fence readers.

The frozen 1.6.29 build continues, but installation and owner closure are held.
Installed 1.6.28 remains unchanged. A planned 1.6.30 candidate will combine the
reviewed receive-recovery and cluster-shape corrections after required tests;
no new package completion or fix deployment is claimed. No xNexus runtime edit
is part of this record. Live cluster/needs and receive qualification remain open.
