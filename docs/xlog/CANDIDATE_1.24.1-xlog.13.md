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
by static artifact audit and independent retention. No broad CI campaign,
installed acceptance, native geometry, live RF success or public release is
claimed here. No additional feature or transmit setup is introduced.
