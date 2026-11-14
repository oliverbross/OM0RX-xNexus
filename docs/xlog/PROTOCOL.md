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

Logbook create/update, audio and TX remain unavailable until their owner-side
contracts are implemented and validated; no control capability implies them.
