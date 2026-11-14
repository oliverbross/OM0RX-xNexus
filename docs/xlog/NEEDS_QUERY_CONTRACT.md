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

Numeric revisions are exact safe JSON integers. Country-reference identity is
the owner's opaque ASCII identity, bounded to 512 bytes and compared only for
equality. Its present form begins `xlog-country-facts-v3:` and may include entity
catalogue, fallback and CTY-data identities. Do not truncate or reinterpret it.
The bounded 64-byte policy identity is initially `xlog-call-dxcc-needs-v1`;
this versions the existing call/DXCC projection, not a new award policy.

Each result contains `query_id`, `normalized_callsign`, `dial_frequency_hz`,
`decoder_mode`, `classification_mode` (`DIGI`), `band`, `country`, `adif`,
`country_resolution`, `call_status`, `dxcc_status` and
`need: {status, score, reason_code, reasons}`. Optional facts are null.
Known need scores are signed 32-bit integers; Unknown always has a null score.
Reasons are bounded to 30 strings. Known results have a null `reason_code`.

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

**Owner transport design remains a review gate:** the existing generic worker
request path can retire the worker after a blocked stdin write. Merely adding
a non-retiring read classification is insufficient. The needs path must have
bounded admission, no lookup-triggered retirement, and a permit retained by the
actual job until completion even after its HTTP deadline. Re-pairing must not
allow abandoned jobs to accumulate. A blocked-writer fixture must establish
this behavior before promotion; a one-second success guarantee is not claimed.

## Acceptance

Require strict parser/body/count/ID/response-size checks, exact result-set and
authority validation, late replies/re-pairing/row trimming, and unchanged
classification policy parity against the complete real owner. Hold mutations,
workspace replacement and reference changes across dispatch; pending/uncertain
or changed authority must not restore facts. Measure saturation/deadline behavior
with concurrent PCM and independent Stop. Assert no archive, forwarding, contact
or hardware side effects. Native IPC/UI and physical reception remain distinct
acceptance levels; this contract alone establishes none of them.
