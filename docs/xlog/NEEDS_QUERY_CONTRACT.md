# Decoded-station needs query contract

Agreed implementation target, 14 November 2026. **Not a shipped capability.**
Isolated owner/SDK work may proceed after source review; canonical promotion
requires paired native acceptance of the repaired profile construction and the
boundary tests below. This refines the sequencing in
[the design](DECODED_NEEDS_DESIGN.md); it does not waive native acceptance.

## Request and response

Optional capability: `logbook.classify`. Authenticated read-only
`POST /xlog/v1/needs/query`; request limit 16 KiB, response limit 64 KiB.
The strict request object contains only `queries`, an array of 1–32 objects with
`query_id`, `callsign`, `dial_frequency_hz`, and `mode`. IDs are unique, 1–128
ASCII alphanumeric/underscore/hyphen bytes. Calls are normalized uppercase,
bounded to 32 bytes and validated as calls without discarding portable suffixes.
Frequency is a positive exact JSON integer no greater than 1,000,000,000,000 Hz;
mode is FT8 or FT4. Client band, entity, contacts and scores are rejected.

Reply fields, all present:

- `protocol_version`, `server_instance`, `workspace_id`,
  `generated_at_unix_ms`, `max_age_ms` (2000).
- `readiness`: `ready`, `unavailable`, `refreshing`, `uncertain`, or `stale`.
- `authority`: null when non-ready; otherwise `logbook_revision`,
  `classification_revision`, `country_reference_revision`, `policy_revision`.
- `results`: the exact request-ID set, with arbitrary ordering and no duplicates.

Numeric revisions are exact integers from 0 through 9,007,199,254,740,990; zero
is permitted only with the owner's explicit ready/complete authority. ADIF is
null or an integer from 1 through 9999. Present band/country strings are nonempty,
bounded to 32/128 UTF-8 bytes without controls. Reasons are each nonempty and
bounded to 256 UTF-8 bytes without controls. These literals are not trimmed or
otherwise normalized; spaces are permitted. Country-reference identity is
the owner's opaque ASCII identity, bounded to 512 bytes and compared only for
equality. Its present form begins `xlog-country-facts-v3:` and may include entity
catalogue, fallback and CTY-data identities. Do not truncate or reinterpret it.
The bounded 64-byte policy identity is initially `xlog-call-dxcc-needs-v1`;
this versions the existing call/DXCC projection, not a new award policy. Both
identity strings are nonempty printable ASCII. Clients accept maximum ages from
1 through 2000 ms and at most 50 ms future clock skew, without extending a
monotonic lease beyond two seconds or renewing it from a repeated timestamp.

Each result contains `query_id`, `normalized_callsign`, `dial_frequency_hz`,
`decoder_mode`, `classification_mode` (`DIGI`), `band`, `country`, `adif`,
`country_resolution`, `call_status`, `dxcc_status` and
`need: {status, score, reason_code, reasons}`. Optional facts are null.
Known need scores are signed 32-bit integers; Unknown always has a null score.
Reasons are bounded to 30 strings. Known results have a null `reason_code`.
Call statuses are exactly `New Call`, `New Band`, `New Mode`, `New Band+Mode`,
`Worked`, `Confirmed`, or `Unknown`. DXCC statuses are exactly `ATNO`,
`C/New Band`, `C/New Mode`, `C/New Band+Mode`, their `W/` equivalents,
`Worked`, `Confirmed`, or `Unknown`. Nullable does not mean that required JSON
fields may be omitted. Missing/null snapshot metadata preserves legacy base
observations and invalidates only needs authority.

Non-ready envelopes have null authority and every row is Unknown with a null
score, `reason_code` equal to readiness, and empty reasons. They preserve query
identity/input echoes and DIGI normalization, but band/country/ADIF/resolution
are null and call/DXCC statuses are `Unknown`.

Ready authority may still yield per-query Unknown with `country_ambiguous`,
`country_unknown`, `known_no_entity`, or `unsupported_band`. Country resolution
uses the owner's `resolved`, `knownNoEntity`, `ambiguous`, `unknown`, `invalid`,
or `missingDate` values. `/MM` and `/AM` must never borrow a home entity or gain
ATNO. Their valid literal call evidence may remain visible, but combined need
is Unknown/null, DXCC status Unknown and country/ADIF null. This does not change
underlying history aliasing or confirmation policy.

For `country_ambiguous` and `country_unknown`, country/ADIF are null and DXCC
status is Unknown; valid literal call evidence may remain visible. Resolution
is `ambiguous` for the former, and `unknown`, `invalid` or `missingDate` for the
latter. `unsupported_band` requires null band, while resolved country facts may
remain visible. These combinations still have Unknown need and a null score.

Existing bridge `{error: string}` errors remain: 400 malformed batch, 401/403
authentication/authorization, 404 absent optional capability, 503 saturation or
retired identity. No mutation receipt or uncertain-write retry semantics apply.
No result may be returned under another server/workspace identity.

## Observation and dispatch

Snapshots optionally add `needs` metadata with the same readiness, authority,
generated time and maximum age, without results. Consumers invalidate facts on
the next observed readiness/revision change or expiry. This is bounded
observation latency, not instantaneous notification of external log edits.

SDK lookup uses a dedicated bounded handle/lane, shared parent revocation and
authority-generation lease. No HTTP call holds the main client or receive-state
lock. One pending batch and a one-second client deadline bound lookup work;
timeout/optional absence clears needs without delaying PCM or Stop. A successful
query must match the authority captured before dispatch and still current after
reply. Facts attach by immutable decode identity/context, never row position.

The owner reads only complete retained contact/reference projections. It does
not cold-load, start a worker, add/archive/forward spots, or write a contact or
station command. A monotonic mutation-fence generation detects intervening
writes even if pending state clears before return. No fence lock crosses I/O.

**Chosen owner transport: one mailbox on the existing worker Session.** The
generic worker request path can retire the worker after a blocked stdin write;
a new blocking writer is therefore unsuitable even with try-lock admission.
HTTP queries instead admit one queued job for the current worker generation.
Only an already-running, ordinary native cluster snapshot poll may carry it;
there is no new poll, wake-up, worker launch or query writer thread.

An expired queued job is removed before attachment. Once attached, the actual
poll owns the in-flight permit across HTTP expiry and companion re-pairing.
Matching stdout completion, including a late abandoned-poll reply, or proven
worker retirement releases it exactly once. Delivery additionally requires the
same live lease, workspace, mutation-fence stamp and unexpired owner authority.
Private query results are removed before ordinary snapshot emission. No needs
mutex spans poll I/O or classification. Renderer-origin snapshot requests cannot
inject the private query fields.

The ordinary poll is roughly one second, so the one-second client timeout is a
deadline, not a success guarantee. A missed cycle or load may return 503/Unknown.
The bounded batch adds bytes to that poll; it does not eliminate transport
pressure or fix existing ordinary-poll write failures. Required fake-process
tests must prove no added writer/retirement call or accumulating jobs under
blocked writes, repeated timeouts and re-pairing. Delayed replies, pre-write
admission failure, absent pollers and generation retirement need distinct tests.

## Acceptance

Require strict parser/body/count/ID/response-size checks, exact result-set and
authority validation, late replies/re-pairing/row trimming, and unchanged
classification policy parity against the complete real owner. Hold mutations,
workspace replacement and reference changes across dispatch; pending/uncertain
or changed authority must not restore facts. Measure saturation/deadline behavior
with concurrent PCM and independent Stop. Assert no archive, forwarding, contact
or hardware side effects. Native IPC/UI and physical reception remain distinct
acceptance levels; this contract alone establishes none of them.
