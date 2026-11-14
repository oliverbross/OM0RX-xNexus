# Proposed Windows distribution hardening

This is tested source for a **future candidate**, based on
`d206b880216543af2142e600ea6b8e5ac6bdd1b2`. It does not change the frozen
`1.24.1-xlog.3` candidate or qualify any installer for distribution. The wrapper
activation below is prepared source for the next reviewed version; no package
has been launched with it. The earlier
[installer review](WINDOWS_INSTALLER_REVIEW.md) remains the record of the
inherited behavior.

## Activation and provenance

The opt-in [distribution overlay](../../src-tauri/tauri.distribution.conf.json)
selects the fork template and ownership include, and bundles its MIT notice.
The ordinary `tauri.conf.json` is unchanged. The prepared Windows wrapper now
materializes and passes the explicit reviewed merged config as described below.
Using it requires a new user-visible candidate version, exact source provenance,
and fresh package/static/install qualification. Do not apply it silently to .3.

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

The marker is written only after payload/uninstaller creation, explicit checks
of every enumerated destination, and a fresh path/content scan. The previous
marker is invalidated immediately before payload replacement so an interrupted
upgrade cannot retain an old completeness claim. A failed or partial install may require manual review; there
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
and exact file hashes; commit `cdb1b84ef5b1e3d15071fd66548bfaa7ee02035c`
has these baseline bytes (the payload-completion follow-up below supersedes the
generated template hash):

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
to the reviewed generated source. Only the guard and marker probes execute. They are
silent disposable test programs, do not install a product or mutate registry
state, and leave their fixtures/receipts intact. Cancellation tests exercise
pre-operation admission and sentinel retention, **not native wizard Cancel**.

Still required after source review and a new version: guarded actual bundling
with the overlay; artifact/static scope recheck; fresh private installer UI,
upgrade, cancellation and standalone uninstall qualification; running-app
refusal; verification that profiles/queues and xLog survive every operation;
and signing/distribution approval. No installation or hardware claim follows
from these source tests.

## Payload-completion review follow-up

The [NSIS AllowSkipFiles reference](https://nsis.sourceforge.io/Reference/AllowSkipFiles)
confirms that extraction can otherwise offer Ignore, allowing continuation with
the error flag set. The fork now compiles payload extraction with
`AllowSkipFiles off` and `SetOverwrite on`, checks errors immediately after each
main/resource/external-binary extraction and `WriteUninstaller`, and aborts on
failure. The marker writer calls `XNValidateCompletePayload`: it requires the
main executable, uninstaller, every rendered resource and external binary to
exist as inspectable regular singly linked files before scanning the tree and
writing the marker. This checks presence/regularity; it is not a cryptographic
post-install byte verification. An extraction failure cannot reach the writer.

The complete template compiled again. Six additional actual NSIS tests execute
the **exact marker functions extracted from that rendered template**, with inert
payloads: complete positive control; missing main executable, uninstaller,
resource and external binary; and a failed upgrade with an existing marker.
All missing-payload cases abort with exit 91 and no marker, including the old
upgrade marker; the complete case writes one. Queue sentinels remain byte-identical.
No application, product installer or registry-writing section runs. Nine policy
tests pass, including new negative controls for skippable extraction and bypassed
payload validation. First qualification receipts above remain preserved.

Follow-up receipt:
`target/nsis-template/5b370151-149e-491f-906c-717f8f50c809/receipt.json`.
Generated template SHA-256:
`22ed402c99eaa283f0e5c30ab5d469852f70bfcfd1e5aabd6413970aa2836975`.
Extracted marker harness SHA-256:
`4547d72573706f65ae5f3c6f65a89f61a61fc1ac1ac8e1ddbfae9402506d704d`.
The original 26 admission cases use unchanged guard bytes. Native wizard
extraction-error, Retry/Cancel, upgrade and uninstall qualification remain pending.

## Prepared build activation and resource provenance

The Windows wrapper now runs the pinned-template and installer-policy gates,
then [distribution-preflight.mjs](../../scripts/xnexus/distribution-preflight.mjs)
materializes the complete base-plus-overlay config into the candidate provenance
directory. It passes that exact file to the pinned Tauri CLI with `--config`.
The merge preserves all **11 existing resource mappings** and adds only the
installer MIT notice (12 total). It also preserves every application/build
setting. Unexpected overlay keys, alternate hooks, foreign identity, additional
Windows platform-config files and inherited `TAURI_CONFIG` are refused for review.

The receipt records the effective config hash, complete base/effective resource
maps, and hashes of both config sources, template, guard, MIT notice, original
upstream template and generator. Inputs and effective config are checked again
before the CLI and after packaging. The generated NSIS source hash is retained.
The existing static audit still verifies standalone/payload marker differences,
application versions/manifest/imports, signed Microsoft offline WebView2 input,
models, Hamlib and legal notices. For this distribution policy it additionally
checks configuration/source/generated-script hashes and the embedded MIT notice.

`-Jobs 1` is now the default and is recorded for Cargo and CMake; `-Jobs 2` remains
an explicit supported ceiling. The release optimization/debug profile is unchanged.
Concurrency is not a per-process memory bound: launch admission, a private Job
Object cap and process telemetry belong to the separately reviewed outer resource
guard. No empirical peak-RAM claim is made from the old .2 receipts, which record
process identities, priority and pauses but not reliable peak memory samples.

Five merge-policy tests pass, including resource-loss, hidden-build-command,
alternate-hook and foreign-product negative controls. A source-only preflight
produced the expected 11-to-12 resource map; both PowerShell scripts parse. This
does not execute Cargo, Tauri, the product, or an installer. Actual merged-config
bundling and subsequent static/installation acceptance remain pending.

### Pinned CLI relative-path proof

The generated config may live in `target/xnexus-provenance`: its location does
not become the base for resource, template, hook or frontend paths. This was
verified by reading the exact `tauri-cli-v2.11.5` sources, without starting a build:

- [`ConfigValue`, CLI lib.rs lines 54–87](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.5/crates/tauri-cli/src/lib.rs#L54)
  reads an argument file into a JSON value and retains no source directory.
  Git blob `81b45dd31cf21513e97a6451b770489b3cfab484`.
- [`load_config`, helpers/config.rs lines 154–216](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.5/crates/tauri-cli/src/helpers/config.rs#L154)
  loads the original `tauri_dir/tauri.conf.json`, merges the supplied values,
  and deserializes while temporarily in that original config's parent.
  Git blob `75fb6b1ac6050755e7a6921288d07cbf26bee3e5`.
- [`build::setup`, build.rs lines 158 and 198–204](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.5/crates/tauri-cli/src/build.rs#L158)
  sets the process directory to `dirs.tauri` before resolving `frontendDist`.
  Git blob `2819bc3fcd6d06fa46883d0451db549e34c1e686`. The standalone
  [`bundle` command also sets that directory at line 139](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.5/crates/tauri-cli/src/bundle.rs#L139),
  blob `4c128ff482f175d9ee3404b88f66d6f0ea861821`.
- The pinned [`NSIS bundler`](https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.11.5/crates/tauri-bundler/src/bundle/windows/nsis/mod.rs)
  canonicalizes `installerHooks` at lines 397–402, reads the custom template
  at 628–630, and joins resource paths to the current directory at 785/808.
  Git blob `67563de153a9ca6905042a7861f65c322a5b3957`.

The wrapper additionally enters the original `src-tauri` directory before the
CLI call. Relative paths therefore preserve their original project meaning;
rewriting them relative to the provenance directory would be incorrect.
