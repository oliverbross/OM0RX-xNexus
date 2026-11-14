# Proposed Windows distribution hardening

This is tested source for a **future candidate**, based on
`d206b880216543af2142e600ea6b8e5ac6bdd1b2`. It does not change the frozen
`1.24.1-xlog.3` candidate, activate the new template in the packaging wrapper,
or qualify any installer for distribution. The earlier
[installer review](WINDOWS_INSTALLER_REVIEW.md) remains the record of the
inherited behavior.

## Activation and provenance

The opt-in [distribution overlay](../../src-tauri/tauri.distribution.conf.json)
selects the fork template and ownership include, and bundles its MIT notice.
The ordinary `tauri.conf.json` and Windows build wrapper are unchanged. After
review, activation requires an explicit overlay argument in the guarded build
recipe, a new user-visible candidate version, exact source provenance, and
fresh package/static/install qualification. Do not apply it silently to .3.

The [generator](../../scripts/xnexus/build-nsis-template.mjs) transforms one
hash-pinned upstream file with exact single-occurrence seams. Both the original
and generated template are committed; edits belong in the generator. The source
is Tauri CLI `tauri-cli-v2.11.5`,
[`crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi`](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.5/crates/tauri-bundler/src/bundle/windows/nsis/installer.nsi),
Git blob `d372e3c391770cf231db974422a1e4f8adaac3a6`, SHA-256
`20f4ecc730defb71f1342eaeaec4021df13be3d843abba0effe88ea5835fa079`.
Its MIT copyright/permission notice is retained in
[LICENSE-MIT.tauri](../../scripts/xnexus/nsis/LICENSE-MIT.tauri) and `NOTICE`.
Review a new upstream template before changing the pin; do not regenerate over
unknown seams. This is a deliberate small fork of upstream installer policy.

## Admission and lifecycle

The [shared NSIS functions](../../scripts/xnexus/nsis/ownership.nsh) admit an
absent or empty new directory, or an owned existing directory. Ownership means
the exact fork product, format, and canonical root in `.om0rx-xnexus-owner.ini`,
plus regular main executable and uninstaller files. The directory name alone
is never evidence of ownership. Nonempty unmarked locations are refused.

Admission requires a canonical absolute local path, rejects drive roots,
noncanonical aliases and trailing dot/space components, checks every existing
ancestor for reparse points, and inspects owned contents without following
reparse points. Files must be regular, inspectable and singly linked. Another
program's executable is refused; the only allowed executable paths are the
fork main binary, uninstaller, and four explicitly named bundled Hamlib tools.
Unknown ordinary files are allowed and preserved. Directory recursion is
bounded to 32 levels. These checks prevent accidental mixed installations;
they are **not** a hostile same-user filesystem race/security boundary.

The retained registry root and selected root are validated before maintenance
UI, in `.onInit`, in the early section before WebView2 provisioning, on leaving
the directory page, and again immediately before installation writes. An
existing retained root must match the selected root. Relocation requires a
separate ordinary uninstall first. `SetOutPath` errors abort before payload
extraction rather than falling through to a previous output directory.

Upgrades replace explicitly enumerated payload files in the owned root. There
is **no previous-uninstaller execution, registry-supplied product command,
WiX migration, or inherited binary-name deletion**. The retained version must
be parseable and no newer than the candidate. Unknown and obsolete prior
product files remain unless a future reviewed ownership manifest enumerates
them safely. This avoids uninstall-then-install losing the ownership marker
while unrelated files remain in the directory.

The marker is written only after payload/uninstaller creation and a fresh path
and content scan. A failed or partial install may require manual review; there
is no rollback or adoption of unmarked contents. Standalone uninstall validates
ownership and ordinary closure before deletes, removes only its known payload,
marker and fork registry entries, and uses nonrecursive directory removal.
Unknown files remain; after removing the marker that residual folder will not
be silently adopted on a later install.

Both install and uninstall use the pinned process plugin's current-user lookup
and refuse a live `OM0RX-xNexus.exe`. They never invoke the inherited force-kill
macro. Normal closure must preserve any unsaved work. The Finish page's Run
checkbox starts unchecked. Manual Run and an explicit unattended `/R` remain
opt-in. The desktop-shortcut choice keeps upstream behavior.

Uninstall has a truthful retention notice and no clear-data checkbox: all
browser stores, companion drafts, durable queued QSOs, default, named and
custom profiles remain. No recursive AppData removal is performed. The
inherited shared WebView2 provisioning behavior remains; its runtime is not
removed by the uninstaller. Default product/registry/shortcut identities remain
separate from xLog, as detailed in the earlier review.

## Executed source checks, 14 November 2026

- The actual NSIS shared guard harness passed **26 cases**: absent/empty/owned
  admission; foreign xLog and mismatched markers; mixed executables; root,
  ancestor and nested junctions; hardlinks; drive/relative/noncanonical/device
  paths; trailing dot/space aliases; retained location and relocation refusal;
  ownership-required uninstall admission; cancellation before any operation;
  and current-user process refusal followed by ordinary test-process closure.
  Foreign, unrelated-file and pending-queue sentinel hashes were unchanged.
- **7 template policy tests** passed, including negative controls that restore
  force-kill, recursive cleanup, old-uninstaller execution, default Run, or
  remove pre-write admission. Generation matched the pinned transformation.
- The complete generated template compiled with NSIS 3.11, the real pinned
  plugin and Tauri support includes, an inert text payload and the MIT notice.
  The resulting compile fixture was **never executed**. This covers actual
  NSIS syntax for install/uninstall callbacks and resource paths, not native UI.

The guard receipt is retained under
`target/nsis-guards/15ba81a401b3452b988ebc58599a0490/receipt.json` and the full
template compile receipt under
`target/nsis-template/e775c7cd-b84c-40eb-8076-447d8e58b886/receipt.json` in the
isolated hardening checkout. Receipts record the precommit dirty-source context
and exact file hashes; the committed files have these same bytes:

| Component | SHA-256 |
| --- | --- |
| Generated template | `b8e251fc5dbef94a976bff6bb84640573329c1417325e4b29cdf812dabd56490` |
| Shared guard | `a4b45b1c038ebdfe86548e45c8eac1b726bb35ee8e4434ac9be98613a2aaeda4` |
| Harness source | `6df7d54a7c0f6b2251de1272fa2dc0dc0e5ac36c2c613cba1dd979a8c772ced1` |
| NSIS compiler | `42850802704ecb11163f7e0329d35ee54bd288953200d4966e226d572848cfc5` |
| nsis_tauri_utils 0.5.3 | `5ba143b5db4a87d32d6e7802e033330aae56cbceabe0d1e3ba41948385ad4709` |

Reproduce the source checks from the repository root:

```powershell
node scripts/xnexus/build-nsis-template.mjs --check
node scripts/xnexus/nsis-policy.test.mjs
pwsh -NoProfile -File scripts/xnexus/test-nsis-guards.ps1
npm install --prefix target/nsis-template-tools --ignore-scripts --no-audit --no-fund handlebars@4.7.8
node scripts/xnexus/test-nsis-template.mjs <preserved-Tauri-2.11.5-NSIS-support-directory>
```

The support directory supplies `utils.nsh`, `FileAssociation.nsh`, and
`English.nsh`; all are hashed in the compile receipt and `utils.nsh` is pinned
to the reviewed generated source. Only the guard probes execute. They are
silent disposable test programs, do not install a product or mutate registry
state, and leave their fixtures/receipts intact. Cancellation tests exercise
pre-operation admission and sentinel retention, **not native wizard Cancel**.

Still required after source review and a new version: guarded actual bundling
with the overlay; artifact/static scope recheck; fresh private installer UI,
upgrade, cancellation and standalone uninstall qualification; running-app
refusal; verification that profiles/queues and xLog survive every operation;
and signing/distribution approval. No installation or hardware claim follows
from these source tests.
