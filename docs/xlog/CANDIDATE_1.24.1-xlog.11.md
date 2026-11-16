# 1.24.1-xlog.11 automatic xLog testing candidate

Application feature source: `ad52decef` (full hash bound in the package receipt).
This commit advances the five version surfaces; the full clean version commit
is recorded in the package proposal before compilation. Candidate10 was never
packaged: final source review found stream IDs were still mistaken for a real
radio-route change.

Local discovery replaces manual pairing. The matching xLog remembers the
chosen slice, RX DAX/channel and TX DAX, and starts the connection and audio.
Nexus retains its original interface and FT transmit controls.

A DAX clock repair or capture restart preserves decoded history, validated
logbook display facts, the current QSO and its Log operation identity. A real
physical route or source namespace change still takes effect. Work and output
continue to use current native authority; old decoded input is not replayed.
Stop keeps the exact original pending operation.

Focused checks passed: five SDK/native continuity cases, seven receiver-hook
cases, two connection-hook cases, then two actual stream-repair regressions and
one extended receiver-hook regression. The combined package provides final
TypeScript and Windows native compilation. Broad upstream CI and live RF
operation are not claimed by these source checks.

Build from canonical C:/Projects/OM0RX-xNexus with pinned Rust1.93.1 Windows GNU,
radio/custom-protocol, jobs1. Testing profile: release optimization3, unchanged
panic unwinding, CARGO_PROFILE_RELEASE_LTO=false and
CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16. Exact environment, source, tools and
artifacts belong to the package receipt. No upstream publication/update feed
is triggered by this local testing candidate.
