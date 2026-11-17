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
