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

Execution remains a separate reviewed change. It needs an ordinary process
observer that proves the installer and NSIS temporary uninstaller handoff have
retired, preserves partial/hung state, and never force-kills. The build Job
supervisor is intentionally unsuitable because disposal terminates its children.
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
