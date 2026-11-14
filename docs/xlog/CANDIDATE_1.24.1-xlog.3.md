# Candidate 1.24.1-xlog.3 — profile construction repair

This is a candidate reservation and build recipe, not a release or acceptance
claim. The exact source before version preparation is
`e52fffc26e1b42889996f015240d6e74506478e4`. Runtime correction
`907ee0369604e63481bfd98a369152c580571559` is its only application-code change
relative to the rejected `.2` source `c2c917b6f748366fbf6b3960876d9f98ba1854c9`.
The remaining integrated changes are reviewed tooling, synthetic acceptance
fixtures and qualification records. Upstream remains pinned to
`c88740a94172b3e78ae587d62bd0a54394d1e85f`; the proposed upstream update is not
included.

The repair prevents automatic window construction and applies the private
directory and Mac store identifier directly to the actual WebView builder.
It preserves the native outbox lock, companion-only entry, commands and window
options. The `.1`/`.2` shared-store failure and their original build receipts
remain preserved. No shared store is migrated, cleared or overwritten as part
of the repair.

## Build provenance

After this version commit passes review and is fast-forwarded to canonical
`main`, obtain its full clean commit with `git rev-parse HEAD`. Pass that exact
commit to both platform helpers; the resulting build receipts bind the final
version commit, tool versions, dependencies, resources and artifact hashes.
Do not substitute this document's pre-version source or package a worktree.

- Windows: `scripts/xnexus/build-windows.ps1 -ExpectedCommit <full-final-SHA>`;
  Rust `1.93.1-x86_64-pc-windows-gnu`, Tauri CLI 2.11.5, existing pinned native
  and model assets, unchanged optimized release profile and unsigned NSIS.
  Preserve the standalone and installer-embedded executable hashes separately;
  the audited Tauri NSIS marker transformation is expected.
- Mac: `bash scripts/xnexus/build-macos.sh <full-final-SHA> 1.24.1-xlog.3`;
  Rust 1.93.1, hash-verified official Tauri CLI 2.11.5 invoked by its absolute
  path, existing native/model inputs, macOS deployment floor 14.0 and the
  recorded release profile with debug information. Use the reviewed symbol
  policy; any new audit discrepancy must fail. Output remains unsigned and
  not notarized.

The Mac pre-version native all-targets check passed, together with four
construction unit tests and eight fork-policy checks. Windows native checking
of the same correction is being completed before canonical promotion. The
constructor tests do not establish actual WebView storage isolation.

## Required native gate

Follow [the two-profile acceptance procedure](PRIVATE_PROFILE_ACCEPTANCE.md):
fresh A, fresh B, then A again, with live identity/private-root verification
before every binding and ordinary Quit/process-only exit checks between runs.
Only after distinct drafts remain isolated may paired synthetic control, QSO
queue and RX acceptance resume. A failure preserves evidence and stops the
sequence; there is no default-profile fallback.

Windows UI automation still awaits desktop availability clarification. Linux
passed the exact `.2` SDK/UI contract, but full native desktop validation remains
pending for corrected source. Physical FLEX slice A/DAX reception, radio/rotor
operation, decoded-call need lookup and transmission remain separate unfinished
work. Nothing in this reservation closes those gates or publishes a release.
