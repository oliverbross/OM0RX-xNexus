# xLog companion protocol draft 1

Status: client/reference contract only; not yet implemented by released xLog.
The Rust client in integrations/xlog is executable contract evidence, not proof
that a live xLog endpoint exists. Both sides must agree before freezing v1.

Native HTTP/1.1 at an explicitly paired `http://127.0.0.1:<port>` address.
No redirects, proxies, cookies, browser origins, network hostnames or generic RPC.
Every request has Authorization: Bearer <pairing token>. Pairing/revocation is
xLog-owned and not in this prototype. Tokens are not passed on the CLI or logged.
The reference client receives the token as a constructor argument; its read-only
probe reads it from an explicitly supplied local file.

GET /xlog/v1/hello returns protocol_version=1, server_instance, workspace_id,
capabilities (initially snapshot.read), application="OM0RX-xLog". Unsupported
major or missing capability fails closed. New server/workspace requires re-pairing
or explicit reconnection; it is never silently mixed with prior observations.

GET /xlog/v1/snapshot returns:
- protocol_version, server_instance, workspace_id, revision (monotonic u64)
- generated_at_unix_ms (wall-clock audit), max_age_ms (1..10000)
- station: {connected, radio_id?, frequency_hz?, mode?, transmitting?}
- rotor: {connected, rotor_id?, azimuth_deg?, moving?}
- spots: bounded list of {id, callsign, frequency_hz, mode, observed_at_unix_ms,
  source, need: {status: needed|worked|unknown, score, reasons: [string]}}

These are materialized complete snapshots, not incremental events. Equal revision
is allowed only for identical content; older revisions fail. A repeated snapshot
does not reset its age: age uses generated time at receipt plus monotonic elapsed
time. Max_age is capped client-side; a future timestamp beyond 2 seconds is refused.
Freshness expires even when a TCP connection remains open. Any failed poll clears
actionable state; stale content can be displayed only as explicitly unavailable.
Read/control methods require explicit capabilities and cannot transmit.

Response budget: 2 MiB, 4096 spots, nonempty bounded identifiers, finite radio and
rotor observations, 32 reasons per spot, 256 bytes per reason. Request timeout 3s,
connect timeout 1s. Unknown additive fields accepted for compatible v1 evolution;
unknown need status refused rather than interpreted as unworked. HTTP error bodies
are not surfaced because they can contain private station data.

## Explicit receive/control commands

`POST /xlog/v1/command` accepts an envelope with `request_id` (UUID),
`server_instance`, `workspace_id`, `expected_revision`, and `command`.
Command kinds and required capabilities are identical:

- `station.tune`: radio_id, frequency_hz, mode.
- `rotor.point`: rotor_id, azimuth_deg (0 inclusive to 360 exclusive).
- `rotor.stop`: rotor_id. Revision 0 is deliberately permitted for Stop; it still
  requires valid pairing, server/workspace and stable rotor identity.

The native client requires a fresh snapshot and matching connected device for
ordinary controls; tuning also requires positively observed non-transmitting
state. xLog repeats admission against its current owner state. No local CAT or
rotor connection is opened. Server capabilities alone determine availability.

Receipts include request_id, server_instance, workspace_id, status and optional
bounded message. Status is `accepted`, `applied`, `observed`, `rejected` or
`uncertain`; an ACK must never be labeled physical observation. A lost/malformed
reply after dispatch returns outcome unknown and must not cause an automatic
repeat. `GET /xlog/v1/receipt/{UUID}` returns a matching receipt or HTTP 404.

`BridgeClient::stop_handle` creates an independent StopClient at pairing time.
The desktop keeps it outside the ordinary poll mutex; Stop neither waits for a
fresh poll nor takes its queue slot. Revocation/workspace changes still refuse
the action at the host. Desktop disconnect drops this handle.

## Explicit selected-call history

`logbook.read` grants `GET /xlog/v1/history?callsign=CALL`. Calls are uppercase
ASCII callsigns; portable `/` is URL-encoded. No browser-side token is involved.
The identity-bound response carries protocol_version, server_instance,
workspace_id, callsign, generated_at_unix_ms, at most 25 contacts and at most
256 band/mode cells. Contacts contain the QSO draft fields plus nullable
contact_revision. Matrix cells contain band, mode, worked and confirmed booleans
from xLog. Missing revision makes an existing contact read-only. Confirmation
is never inferred from worked status. This read is explicit and separate from
the fast station polling path; it is not a full logbook export.

Optional spot metadata preserves country, numeric ADIF entity, latitude,
longitude, band, call_status and dxcc_status. Missing metadata is null; no
coordinates or entity facts are invented locally.

## Durable logbook operations

`logbook.write` explicitly grants `logbook.create {qso}` and
`logbook.update {qso, expected_contact_revision}`. A QSO contains contact_id
(UUID), callsign, band, mode, started_at_unix_ms, optional frequency_hz, rst_sent,
rst_received, grid and comment. The operation request UUID is distinct from the
contact UUID. The owner must atomically commit the contact and its durable
operation receipt; repeated identical operations return the original receipt,
and reused IDs with different payloads are rejected. Update is revision fenced.
The workspace identity persists over normal owner restarts and changes on restore.

The native `outbox::Outbox` stores immutable queue records and syncs an attempt
marker before network dispatch. Its single-writer file lock prevents concurrent
desktop senders. Lost replies and process interruption leave `NeedsReceipt`;
ordinary submission then performs receipt lookup only. Missing receipts require
a separate explicit retry action, which checks receipt absence again and sends
the same operation UUID and payload. Only applied/observed/rejected receipts are
terminal; accepted and uncertain are never displayed as saved. Foreign workspace
records cannot be submitted. Corrupt records and partial temporary writes are
preserved for diagnosis. Queue files remain private to the xNexus profile.

Client tests use synthetic owners; actual availability requires an xLog build
advertising the durable contract. Audio and TX remain separate ungranted
capabilities; no receive/control or logbook capability implies them.
