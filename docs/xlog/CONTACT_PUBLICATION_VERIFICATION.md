# Contact changes and companion need status

The actual xLog worker mutation exports and production Rust classifier projection
passed a serialized acceptance matrix on 14 November 2026. This is source/test
evidence, not installed host, native UI or physical station acceptance.

**Scope correction, 15 November:** the replay supplied a handwritten retained
envelope with an extra `response.result` level, matching the erroneous consumer
instead of the actual ClusterSession producer. Its 23 exports establish mapping
of worker classifications, not retained-owner wiring. The newly confirmed
[reply-shape incident](RETAINED_CLUSTER_INCIDENT_2026-11-15.md) now records the real
producer-to-consumer failing-then-passing regression, 20 cluster/nine projection
and 127 bridge tests, plus corrected replay of all 23 publication/six classification
exports with unchanged inputs. Correction `4025ee6f` is published in candidate
1.6.30 source `8795606b`; package/install and normal handoff subsequently passed.
Fresh operator pairing and live propagation remain pending, as recorded in that incident.

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
envelope with `available=true` and `needs_invalid=false`. The test exercises
worker refresh-state mapping, but its extra nested RPC envelope bypasses the
actual retained producer shape. Separate host mutation-fence tests also did not
establish that missing producer-to-consumer connection.

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

## Broader M3 owner inventory

The xLog owner reviewed existing tests at
`82ba0df513e027bd82699db9236daab467d8d29b`. The following is source review,
not a new execution claim. It separates existing owner capabilities from fields
the current companion API actually exports.

| Area | Existing owner evidence | Companion boundary |
|---|---|---|
| Portable and special calls | `CountryFactsTests.portableCorpus`, `CTYResolverTests.portableOperatingPrefixPrecedence`, `CountryIntegrationTests.sharedHostSnapshot`, `LiveClusterHistoryTests.portableCallHistory` | Cluster indexes literal calls; history merges selected aliases. `/MM` and `/AM` have explicit no-entity facts. A companion comparison fixture remains needed; do not unify these policies. |
| Stored DXCC and corrections | `LiveClusterHistoryTests.pjStoredEntityOverride`, `effectiveCountryOverride`, `CTYCorrectionPreviewTests.isolatedPreview`, `CountryIntegrationTests.atomicConflicts` | Storage/correction evidence does not prove a DXCC-changing edit updates both old/new entity needs through the companion path. |
| Band/mode priorities | `ClusterClassificationTests.testNF3RPatternPreservesBandModeAndConfirmationRules`, `testEntityConfirmationPrefixAndExactCombinationPriority`, plus the executed matrices here | Actual New Band, New Mode and New Band+Mode worker exports and production projection now pass the extension below. |
| Confirmation sources | `ClusterClassificationTests.testConfirmationFlagsAndEqslOnlyExclusion`; `OperatingGoalTests.history`; `LocalAwardProgressTests.globalCandidatesAndPositiveWAC` | Paper-Y/LoTW-Y/eQSL-only worker exports and projection now pass. Award-specific evidence and X/V remain distinct owner policies. |
| Workspace replacement | Executed `PostCommitContactRefreshTests.workspaceFence`, `rejectedRestore`, `shutdownFence`; `CompanionContactTests.ownerHistoryAndReceiptUseWorkspaceIdentityAndUnixMilliseconds` | The 23 exports do not include replacement. Actual replacement export and native host cache-retirement evidence are separate remaining checks. |
| Satellite contacts | `LogbookTests.satelliteFieldsParticipateInSyncBaseline`; award-specific POTA and QRP DXCC tests | Current cluster classification and companion need keys have no satellite dimension; companion QSO/history DTOs omit satellite fields. Satellite-specific needs are unsupported by this contract. |
| Contest needs | Python duplicate/statistics and append-scoring tests; `PythonContestLaneTests.testCWOPSOpenOriginalPluginSerialDupeAndScoreThroughNativeBridge` | Current companion rows have no contest session, duplicate or multiplier fields. Normal-log need status must not be presented as contest qualification. |
| Goals and activation references | `LiveOperatingGoalMethodsTests.retainedProjection`, `LiveOpportunitiesMethodsTests.cachedEvidenceAndHistory`, `clusterProgrammeIdentity`, `islandExpeditions` | Owner functionality exists, but companion rows omit explicit goals/reference need results. A programme tag alone is not evidence that a park/island is needed. |

Satellite, contest, goal and reference needs require explicit additive owner/API
design, source/age/station identity and tests before UI claims are expanded.
They are incomplete integration scope, not failures to be hidden by synthesizing
an independent xNexus award engine. Normal worked/confirmed facts must remain
separate from sponsor eligibility and credit.

## Executed band/mode and confirmation extension

xLog test source `4cf9b8b47c8e85420a463cf1a8602acf7a2df624` adds six
independent private-workspace cases. Actual worker saves and fresh cluster ingress
produce New Band, New Mode, New Band+Mode, paper-Y, LoTW-Y and eQSL-only results.
The last case remains Worked, not Confirmed. No owner policy or runtime changed.

The first combined invocation exceeded its 60-second limit after five successive
workspace setups. That failed/partial evidence is retained. Six serialized
parameterized invocations preserve the same per-case deadline and passed with
all **42 tests in 30.737 seconds**, under the unchanged 2 GiB process-tree cap.
The six-case export directory is separate from both original and fresh 23-case
publication evidence. Native log SHA-256:
`0213f64d2c8108126e88e3b02718e9312f53dbaa2791911a264307299b5f1775`.

Checker source `9411c38c2f645b738bd2702c3dc1101e1672df57` passed all six
unchanged worker exports through the production Rust projector, asserting exact
row ID/count, call, band, mode, frequency, ADIF, status, score and need against an
independent matrix. All 23 fresh publication exports also passed, plus 11
production/harness tests. All 29 owner-original and copied hashes match before
and after replay. This is serialized boundary evidence, not native UI acceptance.

Retained receipt: xLog `.build/classification-projection-20261114/receipt.json`.
Six-case output SHA-256:
`adb7893f11ccd614e3e16e21f225a94af22c76a3bcef9b0b918db12b4d736a86`.
Publication regression output SHA-256:
`8166bfdd02461789623d9280995a053d3619210c11d6e97f8a0b3a261f803e23`.
