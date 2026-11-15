# Candidate 1.24.1-xlog.5 — bounded pending needs lookup

This unsigned Windows development candidate [built, passed static audit and was
independently retained](WINDOWS_BUILD_1.24.1-xlog.5.md) at exact source
`75a59f786ee1f437e7ab14a147bb3ca0ceed1c49`. **Internal testing only:** no `.5`
application launch, installation or live paired receive result is claimed.
The user requires the original Nexus operating interface; the diagnostic
dashboard is not the final product. This `.5` candidate will not be installed
over the unchanged `.4`. TX remains required and unimplemented.

Exact pre-version source is
`371474c900fd4d18fa18f831b5005f5a0fb1fe2b`.
The accepted application runtime is
`9383d286c480fd94820cf63a6651ef6d6f717076`; subsequent changes add documentation
and acceptance/upgrade tooling. The version commit changed only the five
version manifests and this candidate record. Its full SHA was captured externally,
verified against the clean canonical branch and remote, and used unchanged in
the guarded package command, audit and retention receipts.

The upstream pin remains `c88740a94172b3e78ae587d62bd0a54394d1e85f`
([upstream provenance](UPSTREAM.json)). The coordinated xLog owner candidate is
`1.6.31`, exact source `a00697f5458c64cf806e4c9647fcd133fc30391b`. The owner
subsequently reported completed packaging, installation, private acceptance and
normal handoff;
new live paired qualification is not established by that result.

## Scope and source verification

The native needs lane preserves one bounded pending query group across temporary
misses so it can consume the asynchronous owner result. It keeps immutable query
IDs, original row/context fences, the six-invocation/six-second residence bound,
one-second HTTP cadence and original evidence deadlines. A cached response is
eligible only while its serialized remaining lifetime is positive. PCM and Stop
remain independent. No protocol TTL, SDK, decoder or TX authority is widened.
See [the focused source tests](NEEDS_PENDING_RETRY.md).

Two receive/display corrections are included: successful status polling clears
its obsolete polling alert without clearing action errors; selecting a band-map
row retains the exact owner observation rather than searching by nonunique call
and frequency. Neither change touches profile or draft persistence.

Current source evidence:

- The actual production native worker/SDK focused lane passed 23 tests after the
  fractional-millisecond cache correction. The separate virtual timing matrix
  covers 179/200 rows with independent owner, snapshot and query phases.
- The real owner QueryCache/Mailbox → SDK → generated FT4 decoder/worker chain
  passed on the integrated runtime. It validates synthetic owner classification
  and retry behavior, not installed station classification.
- All 59 companion UI tests passed. TypeScript project build and Vite production
  build passed under the measured 4 GiB frontend guard. Fourteen companion
  viewport/theme cases and fourteen production-bundle needs cases passed,
  including expiry/capability/negative controls. The final integrated receipt
  SHA-256 is
  `420A0C3DA71C5944A6A68C6B143930E714A26C6276FE2854028F779A6198100D`.
  All guarded runs retired naturally with zero owned processes. The browser
  owner and decoded rows were synthetic; no native app or station was exercised.
- Upgrade tooling `6f03fc9ebb55e125d7968c02f7a691fe92e610bb` has 36 passing inert
  controls, including preservation and partial-failure journaling. Acceptance
  tooling `34698f83f8f0fecf94ebec8db5a5126cc416f716` is integrated. Neither
  executes a real upgrade by itself.

The focused native final guard SHA-256 is
`85D3BEDCBE0391CC5E503DB65466A51A411216FB0DE50D76E5B71EAAB54D26CB`;
its source-manifest SHA-256 is
`0F688D1F917AF18E862E1644976875EBF3B79E7B75BC4DD1D56E4BA8AD3A0255`.
The frontend build receipt SHA-256 is
`3290EDE869C364F91F2D910D31FCEB00143C40EA4AC2B605A9CC38BC92FA9D83`;
the browser guard receipt SHA-256 is
`9D2A02E71D9FCC3FEEF4D64CA6BD540DE4B01BC9F03584BE7CFF534A6AE7F70C`.
The upstream full UI/Remote sweep was not repeated for this focused change.

The [actual cache/decoder chain](../../integrations/xlog-paired/NEEDS_CACHE_CHAIN.md)
made four queries and two classifications, preserving original timestamp/TTL
equality; its generated owner and headless worker were verified at Normal
priority. Guard SHA-256:
`03DFD4DC8FEFDF041F9622EBDA73DECF509F5AE82BE9371A1AAD1EB1DAA6326C`;
execution receipt SHA-256:
`B5329F4795C5B4394500E71305199428996864F37DFDB7A18BDFE7B25A232997`.

The Normal-priority owner diagnostic capture passed 360,000 ms with 14,388
metadata records, 14,382 publishable records, one startup discard and five
declared gaps, without a fatal result. Trace SHA-256:
`97EC6DB6B913159911D20F348D303275EA619F8C00D6772E15CBFB6DE53A48FA`.
Replay through the production Buffer also passed. This is native diagnostic and
replay evidence, separate from installed paired receive. Earlier BelowNormal
capture failures and all prior installed failures remain preserved in the
[clock incident record](RX_CLOCK_INCIDENT_2026-11-15.md).

## Completed package and remaining product gates

Packaging used the exact clean canonical version commit. Earlier 12 GiB/20–20
and 12 GiB/19–19 proposals were preserved; the one executed, separately reviewed
serial profile used an 11 GiB aggregate Job cap, 18/18 GiB physical/commit
admission, 6/6 GiB runtime reserves, Jobs1, BelowNormal and five-second bounded
natural drain. Correct notification classification distinguishes abnormal exit8
from process-memory9 and Job-memory10; each remained fail-closed. The unchanged
release profile completed with a 10,202,296,320-byte peak and natural root exit,
zero active descendants and no forced cleanup. See the exact build record.

Artifact/import/manifest/NSIS audit, actual loaded CLI provenance and independent
retention passed. No `.5` native or installation acceptance followed. Any internal
private synthetic receive/lifecycle test needs separate explicit release of its
fixture and process scope. Historical `.4` A→B→A storage evidence remains tied
to its source; reuse requires exact unchanged startup/storage/draft/capability/
installer source proof, not merely matching names.

The planned `.4` → `.5` upgrade is cancelled following the explicit interface
direction. The reviewed upgrade helper and its inert tests remain available for
a later, correctly scoped candidate; they authorize no `.5` dispatch. A future
upgrade still requires ordinary close/WebView retirement, exact old/new payload
and registration pins, profile/outbox/draft backup and preservation, no old
uninstaller or forced termination, and post-install verification before launch.

Live receive continuity and authoritative Needed/Worked results require new
paired observation with both corrected installed candidates. Previous `.30`/`.4`
decodes and retained-cluster observations did not qualify either full receive
continuity or decoded needs. TX remains unimplemented/disabled, and current
macOS/Linux package and installed qualification remain incomplete. This candidate
does not complete the full integration project or authorize live transmission.
