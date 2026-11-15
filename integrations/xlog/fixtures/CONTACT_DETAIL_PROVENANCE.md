# Contact-detail producer fixture

`contact-detail-swift.json` is the unchanged synthetic three-case export from
OM0RX-xLog's actual Swift projector tests, produced on 2026-11-15. It contains
`found`, `deleted`, and `not_found`; it contains no production logbook records.

- Producer source: OM0RX-xLog commit `635f754d59e0901180e8b17c065cd4d77f0c285b`,
  `Sources/XLogCore/CompanionContactDetail.swift`.
- Editable-core wire constraints: `Sources/XLogCore/LogbookContact.swift`,
  `CompanionQSO.validate` and its lossless retained-form normalization.
- Original export: `.build/xnexus-contact-detail-tests/tests-null-20261115-154018/contact-detail-fixtures.json` in the xLog workspace.
- Exact SHA-256: `B53A2BE67F986C41A908D3F2BA802AFA055D747AEF6C62EC67EF1A7E6443043F`.

The ordinary SDK test includes these bytes and validates all three cases with a
lossless JSON round trip, including explicit null frequency and Foundation's
uppercase editable-base UUID. This fixture establishes DTO compatibility only;
it does not establish HTTP/session freshness, edit authority, UI behavior, or
installed application qualification. No xLog implementation is copied here.

## Focused validation

The codec was validated against pre-change Nexus base
`e1799c3f1d7a9e50adcb2da0bf78ce6c5922db63` using pinned Rust
`1.93.1-x86_64-pc-windows-gnu`, offline locked dependencies and a fresh SDK test
artifact. All 15 named codec tests passed (14 synthetic tests and this three-case
producer replay), with zero ignored tests. Clippy passed for all SDK targets with
warnings denied. The other 100 SDK tests were filtered, not re-executed.

Retained local evidence is in the `contact-detail-codec` worktree at
`target/attempt-b78d58201ce94876aae9b14cf89aa428`:

- Final receipt SHA-256: `85F9CCF38E77F783AA3182B53D01DA620073FCE3861BE36DF071C08BB03AFD80`.
- Guard receipt SHA-256: `98CF1DE0DE9C422A5DB32E87533BEA40CC73C8E48137BA5F1968EEF1DF48C8CE`.
- Test log SHA-256: `454D650D24E1127AA65F6D7D59CC56E8ACC329C34BA8794C6348DE3D73E4E898`.
- Clippy log SHA-256: `D15BA064A0969E68928A1FFC6C7654FF6674A598593B30EAB3F9CF571A056EA4`.

The isolated 2 GiB / 300-second job used one build job and BelowNormal priority,
8/6 GiB physical/commit admission, and 2/2 GiB runtime reserves. Peak aggregate
memory was 632,881,152 bytes. Root and effective exit codes were zero, natural
drain was 37 ms, and final active process count was zero without forced cleanup.
All pinned source/tool inputs and 1,702 donor cache files were unchanged. This
paragraph records the completed run; it does not expand the qualification scope.
