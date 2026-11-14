# Contact changes and companion need status

The actual xLog worker mutation and production Rust projection chain passed a
serialized acceptance matrix on 14 November 2026. This is source/test evidence,
not installed host, native UI or physical station acceptance.

## Owner mutations

xLog test-only source `095498efc3362f2a1a385e7b63c77c0253b1f6b9`, based on
`f155bd2d9732bf8fbb32e89a6153804f284d3891`, drives actual `LiveWorker` requests
against fresh private SQLite workspaces and an injected cluster transport.
It covers save, LoTW `Y` confirmation edit, deletion, restoration and previewed
ADIF import. It also covers DATA contacts with FT8/FT4 submodes: the existing
DIGI classification family and exact-mode history matrix remain distinct.

A deterministic gate holds publication after the database mutation commits.
The worker must retain the previous published revision while `refreshing`, then
publish `current` with the committed revision and contact count. Fresh synthetic
spots verify Worked, Confirmed and New Call. Increased callsign counts remove
retained spots; a missing row is explicitly checked rather than counted as a
successful classification assertion.

The corrected Windows Swift run passed 41 tests in two suites. Original exports
and source/cache provenance are retained under xLog's
`.build/w10-native-prereqs/contact-publication-evidence-20261114-190915`.
The native test log SHA-256 is
`5ff7430633f50164e1c53123824415c9dfdc90a7e8944d1d1d50c68a65e6823b`.
Earlier fixture-preflight/date-encoding failures remain retained; no runtime
changes were needed to pass these tests.

## Production companion projection

xLog checker source `e44364bc67fdbfcf2abf026a9f67b0415ade7a39` directly includes
the production `companion_projection.rs`; it does not duplicate the classifier.
It consumes the worker's unchanged result and supplies a fresh native observation
envelope with `available=true` and `needs_invalid=false`. Therefore the test
exercises the actual worker refresh-state gate rather than an invented native
invalidation flag. The separate host mutation fence has its own production tests.

All 23 exports passed an independent case matrix: 16 verified present-row needs
and seven verified deliberate absence. All retained rows during refresh projected
as Unknown with zero score and a refreshing reason; current target rows preserved
owner status text and score. Nine production projector tests and the harness's
negative controls also passed. Original/copy input hashes remained unchanged.

The accepted replay used Rust 1.99.1 MSVC, one build job, BelowNormal priority and
a 1 GiB process-tree cap; peak tree memory was 341,192,704 bytes. Output SHA-256:
`ca66a3a9e3dfa125da00069ade29336a819f49d20524789a2166e6fd9c2f91d7`.
Receipts are in xLog's `.build/publication-projection-20261114`.

## Remaining boundaries

This does not complete M3. Installed host/HTTP/UI propagation, decoded-call
batch lookup, broader award rules, entity overrides and activation references
still require their own evidence. The owner currently has differing confirmation
predicates (`X` versus `V`) and its recent-contact DTO omits submode while the
matrix retains it. These tests use LoTW `Y` and do not redefine that policy.

No production log, radio, rotator, installed application or package was changed.
The source changes are test-only and do not require an application version bump.
