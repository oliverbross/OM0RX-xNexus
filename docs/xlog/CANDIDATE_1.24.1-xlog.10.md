# 1.24.1-xlog.10 automatic xLog testing candidate

Application feature source: e26446627 (full commit resolved in the build receipt).
This commit advances the five version surfaces and records the candidate.
The exact resulting clean version commit is bound in the package proposal and
build receipt before compilation; no different source may reuse this version.

Local xLog discovery replaces pairing-file import. Ordinary transport/audio
interruptions preserve the original interface, decoded history and previously
validated logbook badges. Real connection, route or needs revisions still take
effect. Receiver startup uses the remembered FT8/FT4 mode. The matching xLog
candidate supplies one remembered slice, RX DAX/channel and TX DAX configuration.
No separate pairing or RX/TX enablement is required.

Focused source qualification: five actual SDK/native continuity cases, seven
receiver-hook cases, and two connection-hook cases passed. The package build
provides the combined TypeScript and Windows native compilation. Broad upstream
CI and live RF operation are not claimed by these source checks.

Testing build: canonical C:/Projects/OM0RX-xNexus, pinned Rust1.93.1 Windows GNU,
radio/custom-protocol, jobs1. Release optimization remains3 and panic unwinding
is unchanged. This candidate overrides CARGO_PROFILE_RELEASE_LTO=false and
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 to avoid the previous long fat-LTO link.
The package receipt records the exact environment, source and tool hashes.
No upstream release, update feed or deployment is published by this candidate.
