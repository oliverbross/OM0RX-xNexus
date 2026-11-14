# Windows installer coexistence review

Reviewed 2026-11-14. **`1.24.1-xlog.3` is a local candidate, not distribution-qualified.**
This is a read-only review of generated NSIS source and application paths, not an
installation, upgrade, uninstall, registry, or native UI acceptance result.
No installer was executed and no registry value was changed during this review.

## Evidence

The generated installer belongs to rejected candidate `1.24.1-xlog.2`, source
`c2c917b6f748366fbf6b3960876d9f98ba1854c9`. Its files under canonical
`target/xnexus-windows/x86_64-pc-windows-gnu/release/nsis/x64/` are:

| File | SHA-256 |
| --- | --- |
| `installer.nsi` | `3C136B8AF6CFA469C42AA799DC39197F5D0ABAE5B14F5B77016B558FF7582800` |
| `utils.nsh` | `FD766CAF19DD635793D14E7A4247C454FBF7A179D50B80947845E0ABB30CF35F` |

Tauri CLI is `2.11.5`, NSIS `3.11`. The `.2` to `.3` configuration diff changes
only the version; `.3` source is `d206b880216543af2142e600ea6b8e5ac6bdd1b2`.
The separate explicit WebView storage fix does not change these installer rules.
Recheck the generated `.3` script before claiming the same artifact boundaries.
The `.2` artifacts remain rejected for private-profile isolation despite their
successful static package audit.

## Boundaries found in source

| Surface | Generated behavior |
| --- | --- |
| Installation | Current user, user privilege, fresh default `%LOCALAPPDATA%\OM0RX-xNexus` (`installer.nsi:39,100–102,518`). xLog uses a different product directory. |
| Retained location | `HKCU\Software\OM0RX\OM0RX-xNexus` default value overrides the fresh location on reinstall (`951–954`). Ordinary uninstall retains it. |
| Registry identity | Uninstall key `HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\OM0RX-xNexus`; shell AppID `sk.om0rx.xnexus`. xLog's product and `com.oliverbross.xlog.integration` identifier differ. |
| Protocols | File-association and deep-link sections are empty (`701–705,854–858`). |
| Shortcuts | Current-user Start menu and optional desktop `OM0RX-xNexus.lnk`; removal checks the shortcut target (`876–902`). |
| Uninstall files | Explicit application/resource filenames, then nonrecursive directory removal (`815–874`). There is no recursive removal of `$INSTDIR`. |
| Shared runtime | WebView2 is installed only if the checked registry version is absent; no minimum version is configured (`550–640`). xNexus uninstall does not remove WebView2. |
| Registry cleanup | Only fork product keys and its autostart value; `Software\OM0RX` is removed only if empty (`905–937`). |

## Findings and acceptance limits

1. **The clear-data label promises more than it deletes.** The initially unchecked
   checkbox removes only `%APPDATA%\sk.om0rx.xnexus` and
   `%LOCALAPPDATA%\sk.om0rx.xnexus`, and is ignored for `/UPDATE` (`924–937`).
   The fixed `.3` source selects `%APPDATA%\om0rx-xnexus\companion` for its
   WebView profile and durable QSO queue. Named profiles and explicit roots
   also remain. See [profile naming and base](../../src-tauri/src/lib.rs) at
   `config_base` / `profile_dir_name`, and [companion root selection](../../src-tauri/src/xlog_mode.rs)
   at `data_root`. This is a retention/expectation defect, not deletion of xLog data.
2. **A shared custom installation directory is unsafe.** The directory page and
   retained-location restore have no ownership check. Selecting xLog's existing
   installation directory overwrites generic `WebView2Loader.dll` and
   `uninstall.exe` (`699,708`); xNexus uninstall deletes them (`852,862`). Distinct
   default paths do not protect this supported custom-directory choice.
3. **Finish can launch the normal profile.** Run and desktop-shortcut checkboxes
   default to checked: NSIS `Contrib/Modern UI 2/Pages/Finish.nsh:379–389` and
   `installer.nsi:404–414`. Run passes no private-root argument. Silent/passive
   launch requires `/R`, with optional `/ARGS` (`772–782`). Acceptance must prevent
   this implicit launch and use the separately verified private launch procedure.
4. **Process closure is by name, not instance ownership.** `utils.nsh:34–44`
   finds current-user `OM0RX-xNexus.exe` processes and can force-kill them after an
   interactive confirmation, or automatically in silent/passive mode. This can
   affect another private xNexus instance. It does not select xLog's differently
   named host/worker. Close xNexus normally before installer qualification; do not
   use this mechanism to bypass an unsaved-work guard.

No default xLog path/registry collision was found. Actual install/uninstall
qualification remains pending, including retained-location handling in a fresh
disposable account or VM. These findings do not authorize production installation.

## Proposed safeguards — review before implementation

- Add fork-owned preinstall/preuninstall checks: allow a new empty directory or a
  positively identified existing xNexus installation; reject foreign executables,
  unexpected nonempty directories, and ambiguous ownership. Verify canonical
  absolute paths and reparse-point behavior. Never infer ownership from the folder
  name alone. Preserve unrelated files; no recursive wildcard cleanup.
- Validate both the selected and retained locations **before** executing an old
  uninstaller. The current preinstall hook runs after the maintenance page, so it
  alone cannot protect that earlier action. Review a small directory/maintenance
  callback or template override alongside the hooks.
- Make the removal UI explicit: fixed bundle-cache cleanup keeps companion
  drafts, queues, named profiles and custom roots. Do not silently broaden deletion
  to those roots. Any later full-profile removal needs separate selection,
  ownership checks and an explicit warning about unsubmitted QSOs.
- Test empty/owned/foreign directories, a shared xLog directory, retained-location
  overrides, unrelated sentinel files, junctions, cancellation, and pending queue
  preservation before considering distribution qualification.

These are proposals only. Frozen `.3` runtime, configuration and installer behavior
were not changed by this review.
