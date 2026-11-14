# Owner-authoritative needs for decoded stations

Status: proposed bounded extension, not implemented. Reviewed against xNexus
`e3ae1534ade5c34dd4e2d3a81bfbe3a72be15d5c` (worker runtime introduced by
`531a6ec50`) and xLog owner `5efffc1a23af68157e3c564eb2aa345256bea90a`.
This document changes no runtime or candidate version. Implement only after
paired native UI acceptance; physical FLEX/DAX and timing qualification remain
separate gates. The frozen `1.24.1-xlog.1` core candidate has no RX extension.

## Existing authority and reuse

Current decoded rows are observations, not needed-station classifications.
The owner already maintains the complete contact projection in
`LiveClusterMethods.swift`, including contact owner/revision, refresh state and
country-reference identity. `ClusterSessionService.classify` delegates to
`ClusterClassificationIndex.classify(call, band, mode, adif)` only when a contact
snapshot exists. `SpotLogic.score` supplies the current call/DXCC band/mode
policy. Factor a read-only query from these functions; do not call `addSpot`,
which also archives/forwards spots and updates operating displays.

Do not reuse the worker's synthetic `cluster_classify`/`IntegrationMethods.classify`
endpoint: its caller-supplied contact sample is not the live complete logbook.
Do not export a partial log or maintain an independent xNexus award database.
The current policy covers call/DXCC band/mode needs; additional award or operator
priorities require a separately reviewed owner policy extension.

## Proposed additive contract

Advertise an optional `logbook.classify` capability and an authenticated read-only
`POST /xlog/v1/needs/query`. Bound request bytes and accept at most 32 unique
queries, each with `query_id`, `callsign`, `dial_frequency_hz`, and decoder
`mode` (`FT8` or `FT4`). These are proposed names, not a shipped protocol.

The owner derives band, country and DXCC from its own references. Each response
contains the query ID, normalized call/band/mode, optional country/ADIF,
`call_status`, `dxcc_status`, and a need result with status, score and reasons.
Bind results to immutable authority: workspace ID, server instance, logbook
revision, country-reference revision and classification/policy revision. Include
generation time and a bounded maximum age. Use explicit unknown results rather
than a zero score when authority cannot be established.

Do not accept client scores, ADIF entities or contact lists. Use the immutable RX
route's dial frequency and the selected decoder mode, rather than the owner's
hardware `DIGU` mode. Use an explicit RX source with no synthetic RBN bonus or
unrelated spot flags.

Before and after any actor await, verify contact projection owner/revision,
workspace/generation, country reference and readiness. Reuse the owner
`NeedsFence`: pending or uncertain writes, stale/refreshing projection and
changed authority produce unknown. Add optional snapshot authority/readiness
metadata so consumers invalidate facts immediately when a write or reference
change occurs; TTL alone and the presence of cluster spots are insufficient.

## Native attachment and UI

Parse messages in native code using `tempo_core::message::Msg::parse` and its
sender accessor. Initially accept explicit, strictly validated calls (including
portable forms) from recognized message forms. Ignore free text, unresolved
hashes and bracketed calls. The loose callsign predicate alone can mistake
reports such as `599`/`5NN` for calls. Do not strip portable suffixes or borrow a
hash resolution from another receive stream.

Before asynchronous lookup, give each row a stable decode ID and immutable
workspace/server/capture identity, owner epoch, mapping generation, worker epoch,
period and row ordinal. Attach returned facts by ID and matching authority,
never by array position; rows can be trimmed while queries are in flight.

Use a separate bounded lookup lane that cannot block PCM polling, assembly,
decoding or emergency Stop. Deduplicate by call, owner-derived band, decoder
mode and authority. Limit to one pending batch with bounded cache/TTL. Discard
late results after route/session/mode change, authority change or row removal.

Show a need badge, score/reasons and an explicit inspect-call/history action.
Unknown/stale rows must not enter a Needed filter. Never inject decoded rows
into the cluster archive, auto-log a contact, tune a radio or transmit.

## Required acceptance

- Mixed needed/worked/unknown results against the actual complete owner log;
  portable calls, unresolved hashes, report-like strings and free text.
- Missing/stale projection, reference or logbook revision changes, pending and
  uncertain writes, receipt reconciliation and re-pairing across workspaces.
- Reordered responses, bounded row trimming, route/mode/session changes and late
  replies cannot attach facts to another row or restore stale classification.
- Saturation, deadlines and failed queries preserve receive and Stop latency.
- No archive/forwarding side effects or radio/logbook writes from classification.
- Native IPC and paired UI acceptance, followed by separately recorded physical
  station acceptance; generated-audio tests do not prove RF timing or routing.
