# Selected-contact frontend reader

The original contact card can consume the native by-ID answer through
`contactReads.read`. The transport invokes only `xlog_contact_detail_read`, with
the current native connection token and exact lowercase contact UUID. It does
not search a history page, open a local log, use browser HTTP or retry a failed
read. Host mounting and supported editing are subsequent integration steps.

The native SDK remains the complete DTO and producer-policy validator. This
frontend boundary checks IPC identity, selected UUID, capability, bounded lease
and envelope shape. It preserves explicit nulls, raw facts, revision-zero
read-only contacts and the actual producer's edit base. An admitted answer is
copied and frozen, including its nested edit base and original deadline.

The deadline starts before invocation. Repeated producer timestamps retain the
earlier deadline across card closure, refresh and temporary availability loss.
High-water marks and worker-change refusal reset only for a distinct valid
connection identity. Invalid, throwing or backward monotonic time retires facts
and cannot later revive them. A changed selection supersedes a pending reply;
busy requests never create an IPC backlog. Connection intent and close must
invalidate the reader synchronously. Callers must also check the connection's
current read authority when handling events; a retained result is not permission
to enqueue or submit a correction.

## Focused evidence

The final focused run passed 90 cases: 47 selected-contact cases, 35 existing
history-reader regressions and eight actual Swift history producer replays.
The contact cases replay all three checked-in Swift detail states and cover
scope mismatch, expiry in transit, repeated timestamps, revision regression,
worker replacement, availability loss and recovery, A–B–A selection, clock
failure, immutable data and explicit failures. Receipt:
`E9829BD5961F3AA350A37C449EAE3240C820649EF523FEF9A5A5D1C1E1834E1E`.
The bounded job exited naturally with zero remaining processes and unchanged
source/tool inputs. Its earlier 85-case run remains retained separately.

Seven private browser cases passed using the real original ContactCard, styles,
native-only IPC adapter and this reader: three owner viewports, three native
positive controls and one static-footer negative control. The checked producer
bytes retain SHA-256
`B53A2BE67F986C41A908D3F2BA802AFA055D747AEF6C62EC67EF1A7E6443043F`.
The first browser attempt correctly refused found revision 1 after the retained
deleted/not-found revision 2. That failed attempt is preserved. The corrected
fixture starts a distinct synthetic paired session when replaying an independent
older producer record; it changes neither producer bytes nor product admission.
Geometry, hit-testing, immutable multiline display and ordinary Cancel checks
remain unchanged. Final receipt:
`BA4226DEEAA8BF289F829101884DF7AD51171B821A9EC36799473E66F6BCB7EC`;
results: `68D2B08E5295A7B76DF970C8DC6DF525FF1D999A5F2FEADD95AE96BA3947AEA9`.
Browser and job exited naturally with zero remaining processes; no external
requests or native app/desktop input were used. Existing dialog-description
warnings remain recorded. Representative original-card screenshots were
visually reviewed.

The final full `tsc -b` and Vite build passed, producing 41 inventoried assets.
Receipt: `B08D0B998A16C2204298F0A54CD5F79F6F76E6816AF6AB0ABB35CC0323613B1E`;
guard: `A40561EE480D6F8CA7867E4FB1C52755FDDE1BA290A6F9A6E91BE1AE9D56C929`.
It exited naturally with zero remaining processes, unchanged inputs and a
2,624,598,016-byte peak inside the 4 GiB limit. The earlier build before the
availability-recovery correction remains retained separately.
These are source and isolated frontend checks, not a live paired endpoint,
installed application, QSO mutation or station qualification. No version or
package changes accompany this source slice.
