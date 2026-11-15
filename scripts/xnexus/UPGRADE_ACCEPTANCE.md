# Owned .4 to .5 upgrade

This helper is prepared tooling, not evidence that an upgrade has run. It does
not reserve a version, build a candidate, close an application or invoke an old
uninstaller. The original .4 installation helper and all historical receipts
remain unchanged.

The only supported transition is installed `1.24.1-xlog.4`, application source
`ae35fc6435f861e38029049299833afe57de4885`, to a separately reviewed, uniquely
versioned `.5` candidate. The current-user target, two shortcuts and default
companion profile are derived from the hash-pinned actual installation and normal
launch receipts. Named/custom roots and environment overrides require separate
review. No other application profiles are discovered or copied.

## Preparation after the candidate freeze

1. Build/audit/retain the new exact-source candidate. Run its integrated frontend
   gates and private synthetic paired retry/decoder acceptance, including ordinary
   lifecycle. Record the actual receipts, not planned tests. No live RF is implied.
2. Reuse the historical A → B → A storage result only when the helper's recorded
   Git object identities prove the startup, native storage, capability, draft and
   history boundaries unchanged. Receive-panel and station-map UI changes still
   require current candidate frontend/private acceptance. If a protected source
   changes, stop and review fresh profile-isolation qualification.
3. Create a fresh protected current-user/SYSTEM/Administrators-only evidence
   directory under
   `C:\Projects\OM0RX-xLog\.build\xnexus-upgrade-acceptance\upgrade-<32 lowercase hex>`.
   Do not place it in application data or the install target. Save `review.json`
   with `schema: 1`, the exact `newSource` and `newVersion`, and these two maps:

   - `inputs`: `oldInstallation`, `oldLaunch`, `oldAudit`, `newBuild`, `newAudit`,
     `newConfig`, `newNsis`, `newInstaller`, `qualification`. Each value is an
     object containing an absolute `path` and `sha256`. The three old receipt
     hashes are fixed in `upgrade-inputs.ps1`; new inputs must match the passing
     current artifact audit. Retain all files independently before target reuse.
   - `tools`: a SHA-256 for each of `execute-owned-upgrade.ps1`,
     `upgrade-inputs.ps1`, `upgrade-transaction.ps1`, `installer-transaction.ps1`,
     `installer-acceptance-lib.ps1`, `installer-observer.ps1`,
     `installer-observer.cs` in this reviewed helper directory.

4. The `qualification` input is a reviewed aggregate JSON record with schema
   `xnexus-4-to-5-qualification`, exact source/version, status `passed`,
   `privatePairedRetryDecoderPassed: true`, `normalLifecyclePassed: true`, the
   historical `profileBaselineReceiptSha256`
   `D5182423C6DB8275DF4D5AFA4A91326D2B8ED2D8D894E0C8F29B1AB2C64C9FB7`,
   and 2–16 unique actual receipt `{path, sha256}` objects. These statements need
   independent review of those receipts; setting booleans is not qualification.
5. Review the complete plan and pin its SHA-256. Run the entry point in a fresh
   `pwsh -NoProfile` process without `-Execute` first. The helper checks the whole
   import closure before sourcing it, the canonical clean candidate, retained
   source/config/template identities and all input hashes. No profile is read in
   this mode. Keep canonical source frozen through dispatch and post-verification.

## Explicit execution after release

After ordinary close of xNexus and retirement of its profile WebViews, invoke
`execute-owned-upgrade.ps1 -ReviewPlan <review.json> -ExpectedReviewSha256 <hash>
-Execute` in a fresh `pwsh -NoProfile` process. This is not an instruction to run
it before the reviewed candidate and package-owner release exist.

The helper refuses any live xNexus or unidentified/relevant WebView. xLog need
not be closed: it owns a different process and data root. The helper repeatedly
verifies the exact .4 payload, marker, generated uninstaller, full typed HKCU
registration, shortcuts, runtime and current profile state. It copies the actual
closed companion profile, including outbox, drafts, WebView files and empty
directories, into the protected evidence directory. File handles deny concurrent
writers during each copy; hashes and full inventories are checked again before
dispatch. Reparse/hard-linked inputs, oversized inventories and backups over
8 GiB stop the attempt rather than being omitted.

Only the new installer is dispatched, with `/S /D=<exact owned target>`. The
reviewed template disables default autorun, requires ordinary app closure,
replaces only enumerated owned payload files and preserves data. The observer
uses ordinary nonterminating Job membership: it never kills on timeout or closes
an application. This narrow upgrade expects one naturally retired, identity-bound
installer process with exit zero. Unexpected children or partial state stop
qualification without cleanup.

Afterward, verify the new payload/marker/uninstaller, same install directories,
same shortcut identities and all old unknown non-executable files. Only the
owned uninstall registration's version and integer estimated size may differ;
all value names/types and other values remain exact. Profile contents, unrelated
shortcuts, shell namespace and registered WebView runtime must remain unchanged.
The original backup is verified again and retained. No application is launched.

The claim journal prevents accidental replay. Failure after dispatch is recorded
as unknown installation state; there is no retry, rollback, uninstall, forced
termination or automatic restoration. Preserve the receipt and backup for a
separately reviewed recovery. Hash manifests contain filenames and hashes; the
backup itself contains private data and must never be committed or uploaded.

## Focused validation

`upgrade-transaction.test.ps1 -EvidenceRoot <fresh private test directory>` uses
inert installer/process/registry adapters and synthetic disposable files only.
It tests the positive transition, predispatch identity/state races, live-process
refusals, backup corruption, unknown-file retention, profile loss, typed registry
changes, shortcut changes, timeout/nonzero/unexpected-child results and partial
dispatch journaling. Actual backup copies include empty directories and verify
the unchanged synthetic source. Run only in the resource slot released for tests.
These checks do not establish actual installer or current-profile acceptance.

On 2026-11-15, 36 synthetic controls passed under the tracked corrected
notification guard: 1 GiB aggregate cap, BelowNormal, 8/6 GiB admission and
2/2 GiB runtime reserves. The child exited zero with no remaining members;
peak usage was 76,845,056 bytes. The private evidence is
`target/upgrade-tests-ed20fbdd9a0d49ab817805bd08890839` in the upgrade-helper
worktree. Its test receipt SHA-256 is
`65A40D6C98A4D7A9495B71834BB5E4E5B96D9613EE7D4836ADE8F341EC716DD9`
and guard receipt SHA-256 is
`A34EADFDAB722333B3FF82460259B03A73A1B92931FC351DF89D1F09C81F8E3F`.
Earlier sandbox ACL and nonexistent inert-target fixture failures are retained;
neither reached product dispatch. All four new PowerShell files parsed. No real
upgrade was performed, and the current .5 artifact pins are not yet available.
