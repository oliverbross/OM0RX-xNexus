# Prepared process-only acceptance launcher

`start-candidate.ps1` is a prepared, process-only Windows acceptance helper.
Positive native launch behavior is not yet qualified. Run
`test-launcher-guards.ps1` for isolated guard checks; their preserved evidence
goes under the repository's ignored `target/xnexus-launcher-guards` directory.
The actual source accepts `--xlog-data-dir=<absolute root>` and places WebView
storage below `<root>\webview`. Default startup is companion mode; the launcher
supplies no independent-mode argument.

Required parameters are `-Executable`, `-ExpectedSha256`, `-ExpectedSource`,
`-ExpectedVersion`, `-PackageReceipt`, and `-OwnerLaunchReceipt`. Use values from
the completed canonical Windows packaging receipt and an existing positively
verified synthetic owner's `launch.json`. `-ValidateOnly` runs preflight without
creating a profile or starting a process. It cannot qualify a package whose
receipt still says `started` or whose executable/installer hashes differ.

The launcher checks every path component for reparse points, the canonical
executable/receipt locations, package and installer hashes, the private TEMP
fixture marker and ACL, and the live owner PID/executable/full arguments and
creation time. STOP or a missing/replaced owner aborts. Inherited WebView path or
argument overrides are refused. It creates a fresh child profile and preserves
stdout/stderr and a per-attempt receipt. Candidate and WebView verification is
process-only, with WebView user-data-dir constrained to the private `webview`
directory or an actual descendant. It never invokes a GUI binding or kills a
process. A failure is a hard stop, with evidence retained.

Initial missing CIM metadata can wait within the 20-second deadline only while
the exact launch handle remains alive. An observed mismatch, exited handle or
loss of previously verified metadata stops immediately. The direct WebView
browser is identified by its executable, ancestry, creation time, full arguments
and private storage coordinate, and must persist across checks. Final verification
reads a fresh descendant tree: normally exited utilities are skipped, while every
still-live WebView must prove its private root. Unknown or foreign live coordinates
and a missing/replaced persistent browser remain hard stops.

After a successful `verified-process-isolation` receipt, the supervising task
must still verify those live process coordinates immediately before any GUI/AX
binding. This launcher receipt is point-in-time evidence, not permission to bind
an app that subsequently exited. Do not bind to discover whether it survived.

For draft/outbox persistence acceptance, use ordinary Quit in the verified
disposable application, respect any unsaved-work guard, and inspect processes.
Then invoke the same launcher/parameters with `-PriorLaunchReceipt` pointing to
that successful attempt's `receipt.json`. Reuse is accepted only for the same
launcher hash, candidate hash/source/version, marked profile and still-live owner
coordinates, after all recorded candidate/WebView PIDs and any process naming
that profile have exited. The previous receipt remains intact; a new receipt is
written. Never use this route after a panic, uncertain exit or forced termination;
those require stopping the sequence and reviewing the preserved evidence.

Validation performed without an application launch: PowerShell parser passed;
24 function/AST guard checks passed, including initial missing/mismatched metadata,
persistent browser versus transient utility lifecycle, newly foreign live children,
exact arguments, WebView roots, reparse paths, hashes, owner replacement/exit and STOP. A separate
real `-ValidateOnly` invocation rejected a noncanonical executable before reading
owner or package state. Retained test evidence is under the ignored
`target/xnexus-launcher-guards` directory. Positive native/WebView behavior remains untested until the
supervising task reviews and uses the launcher with the completed package.
