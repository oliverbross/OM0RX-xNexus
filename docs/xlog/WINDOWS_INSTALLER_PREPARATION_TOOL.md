# Windows installer preparation tool

`scripts/xnexus/prepare-installer-acceptance.ps1` implements the read-only
preparation stage of [the acceptance plan](WINDOWS_INSTALLER_ACCEPTANCE_PLAN.md).
It is **not an installer executor**. Its `-Execute` switch fails before touching
evidence or starting a process. No application version is reserved by this tool.

Pass the exact completed build receipt, static audit receipt, independently
reviewed audit SHA256, version and full source commit. The canonical source must
still match. Supply an existing, current-user-owned evidence parent with ACL
inheritance disabled and access limited to that user, SYSTEM and Administrators.
The script checks this boundary; it never changes ACLs or creates parent folders.
Paths must be canonical local fixed-volume paths without reparse ancestors.

The default invocation creates only a nonce evidence directory. Its proposed
`app` child remains absent. It rehashes audited inputs, expands the actual
generated NSIS `File` instructions into exact destination/hash pairs, rejects
unknown extraction forms, inspects all twelve product/uninstall/Run registry
coordinates, checks process absence, hashes known profiles and shortcut roots,
and reads shortcut targets. Missing access, collisions and ambiguous data stop
preparation; they do not authorize cleanup. WebView2 must already meet the actual
generated minimum through the installer's first-choice machine registration.
This limited lane refuses fallback/provisioning rather than changing shared state.

The report records proposed silent arguments, the exact ownership-marker
contract and sentinel locations. Synthetic sentinels are **not created** by
preparation: they belong after the future verified owned install. Existing
profiles are read and hashed only. Reports and inert test fixtures are retained.

Execution remains a separate reviewed change. A non-terminating observer library
now has two passing finite-lived fixture cases: parent exit before child retirement,
and timeout preserving a live child until its ordinary completion. It is not wired
into an installation transaction. The build Job supervisor is intentionally
unsuitable because disposal terminates its children.
The installed uninstaller is generated at installation time and is absent from
the static 7-Zip extraction; the future transaction must record its identity
after the verified install and reverify it before ordinary `/S` invocation.
Immediate pre/post namespace, payload and retention comparisons must surround
that transaction. A preparation report does not reserve shared registration.

Run `pwsh -NoProfile -File scripts/xnexus/installer-acceptance.test.ps1` for inert
positive/negative controls. These test actual junction refusal, exact receipts,
registry inventory completeness, runtime minimums, destination mapping, marker
identity and retained-file hashes. They do not install/uninstall the product,
qualify native application startup, or exercise an NSIS child process.

`installer-observer.test.ps1` compiles only a tiny inert fixture. The observer uses
[atomic Job-list assignment](https://learn.microsoft.com/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)
at process creation, verifies actual zero limit flags and memory limit, and has
no termination API. It records live PID, creation time, executable/hash and full
arguments. Closing its handles leaves processes alive; timeout is unresolved,
never successful cleanup. Each fixture is finite and the test waits for ordinary
exit, then positively verifies no owned process remains.

[Windows Job membership](https://learn.microsoft.com/windows/win32/procthread/job-objects)
covers ordinary `CreateProcess` descendants; service/WMI launches are outside
that proof. Kernel total/active counts prevent root-only completion claims. If
a short-lived process escapes identity observation, the library reports a
historical identity gap even when the tree retired. Actual NSIS handoff and any
unexpected helper still require candidate-specific evidence and review. No
installer, application UI or registry mutation is exercised by these fixtures.
