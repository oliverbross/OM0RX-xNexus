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
No mutation method exists in this first client.

Response budget: 2 MiB, 4096 spots, nonempty bounded identifiers, finite radio and
rotor observations, 32 reasons per spot, 256 bytes per reason. Request timeout 3s,
connect timeout 1s. Unknown additive fields accepted for compatible v1 evolution;
unknown need status refused rather than interpreted as unworked. HTTP error bodies
are not surfaced because they can contain private station data.

Later revisions add log cursor streams, needs batches and mutation receipts with
request_id + expected workspace/server/revision. Those are design intentions, not
advertised capabilities. TX and rotor/tune writes are absent in this first slice.
