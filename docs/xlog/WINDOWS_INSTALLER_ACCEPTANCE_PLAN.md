# Windows silent installer acceptance plan

**Prepared, not executed.** This plan requires a new uniquely versioned Windows
artifact, its completed static audit and explicit package-owner release. It does
not authorize use of rejected `.1`/`.2` artifacts or change the current candidate.
The existing project authorization covers disposable qualification; a production
namespace collision stops this procedure rather than authorizing replacement.
Native application GUI/lifecycle and physical station acceptance remain separate.

## Source and observed prerequisites

This review used Nexus `1b4ddbcbdcf763e791e2062e3092d650d65f0c15`, including the
[generated NSIS template](../../scripts/xnexus/nsis/installer.nsi),
[ownership functions](../../scripts/xnexus/nsis/ownership.nsh) and reviewed
[distribution hardening](WINDOWS_DISTRIBUTION_HARDENING.md). The eventual
candidate's generated script and hashes must match its own audited provenance.

The current configuration is `currentUser`, with default path
`$LOCALAPPDATA\OM0RX-xNexus`. A custom `/D` isolates files, **not registration**:
installation writes the fork's shared current-user product and uninstall keys,
may save installer language, and can provision the shared WebView2 runtime.
Uninstall removes product-wide registration and the product's Run value; it does
not qualify as isolated solely because the installation directory is disposable.

Read-only inspection on 14 November 2026 found no fork product/uninstall/Run
entries in either registry view of HKCU or HKLM and no matching xNexus shortcuts
in the inspected user/common Desktop and Start Menu roots. Machine WebView2
registration reported `157.0.4341.46`. These are historical observations only:
repeat the full preflight immediately before execution. No registry or installed
state was changed during this review.

## Required namespace and artifact preflight

1. Require exact version/source `V`/`C`, the completed unsigned/uninstalled build
   receipt and passing [static audit](../../scripts/xnexus/audit-windows.ps1).
   Rehash the installer and standalone/payload artifacts. Verify the actual
   generated template uses the reviewed guard, no force-kill and no default Run.
   Keep `.2` retention and all failed evidence independent of the target.
2. Inspect both 32-bit and 64-bit views of HKCU and HKLM for
   `Software\OM0RX\OM0RX-xNexus`,
   `Software\Microsoft\Windows\CurrentVersion\Uninstall\OM0RX-xNexus`, and
   value `OM0RX-xNexus` under
   `Software\Microsoft\Windows\CurrentVersion\Run`. Inspect matching shortcuts,
   default install location and known xNexus profiles without changing them.
   Record bounded relevant values/absence and existing file hashes. Refuse an
   inaccessible/ambiguous namespace or any existing installation/registration
   that this test could overwrite or unregister. Do not clear keys to proceed.
3. Positively inspect process identity: any live `OM0RX-xNexus.exe` stops the
   sequence. Do not close/kill an unrelated application or inspect a GUI binding
   to find out whether it is alive. The installer itself also refuses a live
   current-user process of this name; that is not a substitute for preflight.
4. Confirm registered WebView2 already satisfies the generated installer policy.
   If absent, unreadable or an update is required, stop this limited acceptance
   lane: runtime provisioning affects shared state and needs a separately
   reviewed execution plan. Keep the signed cached Microsoft input and hash.
5. Choose a fresh nonce directory under the private evidence tree, with an absent
   or empty `app` child. Verify the resolved path and every ancestor are local,
   canonical and non-reparse. Do not pre-seed the install target with sentinels:
   the ownership guard correctly rejects a new nonempty unowned directory.

If existing fork registration/shortcuts make current-user isolation impossible,
use the existing disposable inert NSIS harness or a separately provisioned test
Windows account/VM. The inert harness validates guard semantics, not installation
of the product. Do not migrate or overwrite a production installation to obtain
an acceptance result.

## Actual silent install and uninstall sequence

Use only the already audited installer, with a recorded exact process tree and
bounded execution. The reviewed flags are `/S /NS /D=<absolute-empty-target>`;
`/D` must be last. Choose a target without spaces to remove quoting ambiguity.
Do not pass `/R`, `/ARGS`, `/UPDATE` or `/P`. `/S` skips the Finish page and
`.onInstSuccess` runs the application only with explicit `/R`; `/NS` suppresses
new shortcuts. Existing-shortcut migration precedes `/NS` checks, which is
another reason the fresh namespace gate is mandatory.

After completion, require root and any installer descendants to have exited,
not merely a successful launcher return. Record exit status, process evidence,
file/registry changes and absence of any product launch. Verify main payload,
WebView loader, all resources/licenses, uninstaller and ownership marker against
the audited package. Marker product/format/root and registered version/location
must identify only this exact installation. Confirm no unexpected shortcut, autorun or protocol
registration; the current base config declares no application associations or
deep links. Do not start the installed application in this acceptance lane.

Only after the successful owned install, add ordinary unknown-file sentinels
inside its root and a nested directory. Put synthetic draft/queue sentinels in
separate disposable profile roots outside the installation. Use clearly labeled
inert data; this checks byte retention, not real queue recovery. Preserve hashes
of any preexisting default/named profile state without modifying it.

Immediately before uninstall, reverify artifact identity, marker/root, the exact
retained registration and zero live product processes. Any replacement or
intervening production state stops the test: product-key deletion is not scoped
by the custom `/D` location. Invoke the installed ordinary `uninstall.exe /S`.
Do not use force-kill, clear-data actions, direct recursive deletion or special
uninstaller-copy bypasses. Follow its ordinary temporary-process handoff and
wait for the verified uninstall process tree to retire.

Required postconditions:

- Enumerated product payload, marker, uninstaller and this test's fork
  registration are removed; no unrelated registry entries or shortcuts changed.
- Unknown-file sentinels survive. Their residual root is unmarked and must not
  be silently adopted by another install. Do not delete it as part of the test.
- External synthetic profiles/drafts/queues and all observed preexisting profile
  bytes remain unchanged. Shared WebView2 remains installed.
- No app process or unexpected helper remains, and no production xLog process,
  profile, hardware or registration was touched.

A failed, hung or partial installation retains evidence and stops further steps;
do not run cleanup/uninstall against an unmarked or unverified root to make the
test appear clean. Normal successful silent install/uninstall does not establish
interactive dialog correctness, a version upgrade, app startup/storage behavior,
signing trust, other Windows accounts or operating-system versions.
