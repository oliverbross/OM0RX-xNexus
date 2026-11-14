# Windows installer preparation tool

`scripts/xnexus/prepare-installer-acceptance.ps1` implements the read-only
preparation stage of [the acceptance plan](WINDOWS_INSTALLER_ACCEPTANCE_PLAN.md).
It is **not an installer executor**. Its `-Execute` switch redirects the operator
to the separate execution entry point without touching evidence or starting a
process. No application version is reserved by either tool.

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

`execute-installer-acceptance.ps1` implements the separate transaction. Default
mode validates the exact reviewed preparation and audit hashes, then returns
without installation or evidence writes. Actual execution requires all three
arguments below plus explicit `-Execute`, after package-owner release:

- `-PreparedReceipt`: the retained `acceptance-<nonce>/preparation.json`.
- `-ExpectedPreparationSha256`: the independently reviewed preparation hash.
- `-ExpectedAuditSha256`: the independently reviewed candidate audit hash.

The executor has no injectable production adapters or test-bypass argument. It
creates a single-use claim and journal, repeats the complete real preparation,
compares it with the reviewed state, and rechecks namespace, profiles, shortcuts,
runtime and artifact identity immediately before dispatch. The target remains
the original reviewed absent `app` child; the repeat preparation's unused target
is never installed. A failed transaction cannot automatically resume or retry.
The original nonce directory's owner and allowed ACL principals are rechecked
before its claim is created and before each dispatch; creating a fresh sibling
preflight does not substitute for checking the retained transaction directory.

It invokes only `/S /NS /D=<exact target>` and, following a verified owned install,
the generated ordinary `uninstall.exe /S`. Exact payload destination hashes,
marker bytes, generated uninstaller identity, current-user registration and
protected state must match before sentinel creation and again before uninstall.
Successful uninstall requires removed owned payload/registration, preserved
unknown and external synthetic sentinels, unchanged protected state and a retired
process tree. Any intermediate failure retains its journal and partial state;
there is no rollback, force termination, recursive delete or application launch.
Receipts distinguish `installedVerified`/`uninstalledVerified` from dispatch.
After either process dispatch, state is explicitly unknown until its complete
postconditions pass; a thrown partial operation never implies no mutation occurred.
Owned product/uninstall registry snapshots retain every bounded value and its
native type, and refuse subkeys. Changing `REG_SZ` to `REG_EXPAND_SZ` with identical
text therefore invalidates the retained registration fence.

A non-terminating observer library has three passing finite-lived fixture cases:
parent exit before child retirement, timeout preserving a live child until its
ordinary completion, and root exit zero followed by child exit one. The last case
proves the transaction rejects a failed child even when the root succeeded.
The build Job supervisor is intentionally unsuitable because
disposal terminates its children.
The installed uninstaller is generated at installation time and is absent from
the static 7-Zip extraction; the future transaction must record its identity
after the verified install and reverify it before ordinary `/S` invocation.
The executor performs those checks. A preparation report does not reserve shared
registration. The actual candidate NSIS temporary-child filename, arguments and
byte identity remain runtime gates: unrecognized helpers or historical identity
gaps stop the transaction rather than extending an allowlist automatically.

Run `pwsh -NoProfile -File scripts/xnexus/installer-acceptance.test.ps1` for inert
positive/negative controls. These test actual junction refusal, exact receipts,
registry inventory completeness, runtime minimums, destination mapping, marker
identity and retained-file hashes. They do not install/uninstall the product,
qualify native application startup, or exercise an NSIS child process.

`installer-transaction.test.ps1` covers 24 injected inert transaction cases,
including success, namespace and artifact refusal, partial/failed installation,
timeout, missing process identity, unexpected helper, changed payload or marker,
replaced registration/uninstaller, and retained profile/sentinel loss. These
controls prove orchestration ordering and refusal, not actual NSIS behavior.
Two additional cases invoke the actual executor's non-mutating default and
wrong-review-hash refusal. Seven more exercise the actual typed registry
projection and ACL descriptor validator, for 33 checks in that suite.

`installer-observer.test.ps1` compiles only a tiny inert fixture. The observer uses
[atomic Job-list assignment](https://learn.microsoft.com/windows/win32/api/processthreadsapi/nf-processthreadsapi-updateprocthreadattribute)
at process creation, verifies actual zero limit flags and memory limit, and has
no termination API. It records live PID, creation time, executable/hash and full
arguments. Query/synchronize handles retain each observed member's exact creation
identity and exit code after PID retirement; every member must exit zero for
transaction success. Closing its handles leaves processes alive; timeout is unresolved,
never successful cleanup. Each fixture is finite and the test waits for ordinary
exit, then positively verifies no owned process remains.

[Windows Job membership](https://learn.microsoft.com/windows/win32/procthread/job-objects)
covers ordinary `CreateProcess` descendants; service/WMI launches are outside
that proof. Kernel total/active counts prevent root-only completion claims. If
a short-lived process escapes identity observation, the library reports a
historical identity gap even when the tree retired. Actual NSIS handoff and any
unexpected helper still require candidate-specific evidence and review. No
installer, application UI or registry mutation is exercised by these fixtures.

The executor checks the exact fork `sk.om0rx.xnexus` AppID in the generated source
and retained helper, plus its specific shell registry metadata. It does not read
global Recent histories or claim they remained byte-identical. The ordinary NSIS
destination-list cleanup effect remains part of the candidate-specific runtime
boundary. No real installer acceptance has been performed during tool development.
