# Private WebView profile qualification

The `1.24.1-xlog.1` and `1.24.1-xlog.2` candidates are **not qualified for use**. Mac native acceptance found that a newly created `.2` profile displayed the synthetic draft saved during `.1` testing. Both candidate processes had positively verified executable hashes and distinct absolute `--xlog-data-dir` arguments. A read-only examination after normal shutdown located the draft in the fork's default WebKit store. No pairing file was selected, no QSO was submitted, and no hardware was accessed.

The locked Tauri runtime 2.11.2 conversion from `WindowConfig` to `WebviewAttributes` omits both `data_directory` and `data_store_identifier`. Config-only assertions therefore did not establish isolation on either platform. The fix preserves the main window's options and companion URL, suppresses automatic configured windows, and calls the actual builder's storage setters before explicit creation in setup. Native outbox ownership is still acquired before any WebView creation. Missing configuration, unsupported macOS, and construction errors do not fall back to a shared profile.

The construction tests execute the production helper against a recording builder whose config conversion deliberately omits storage, reproducing the framework defect. They test setter ordering, distinct coordinates and failure propagation. The real adapter must also pass pinned native compilation. These are construction guards, **not proof of native storage isolation**.

## Required native A → B → A test

Use a newly versioned, reviewed candidate with completed artifact receipts. Keep its exact executable hash, source and version fixed throughout. Do not reuse or rebuild either failed candidate version.

1. Prepare two different marked private profile roots, A and B. Preserve their paths, permissions and markers. Never use production state or delete existing WebKit/WebView data.
2. Explicitly launch the exact candidate with `--xlog-data-dir=<absolute A>`. Before any GUI binding or AX observation, prove its live PID, executable and complete arguments. If a synthetic owner is used, independently prove its matching fixture coordinates too. A returned launch PID or expected path alone is insufficient.
3. Confirm that A starts with an empty draft. Enter a uniquely marked synthetic callsign/comment using freshly observed UI controls. Keep it unqueued and unpaired for this isolation test. Do not write localStorage directly or inject a fixture draft behind the UI.
4. Ordinary Quit, respect any unsaved guard, then verify process exit without another application binding. On Windows, also prove the relevant persistent WebView browser and its descendants exited before reusing A.
5. Launch the same executable explicitly with fresh root B, and repeat process proof before UI access. B must start empty and must not contain A's draft or any draft from the previously shared default store. Enter a different synthetic draft and quit normally with exit proof.
6. Relaunch the same executable with the exact A arguments and a new positive process proof. A's original draft must return unchanged; B's draft must not appear. Quit normally again and retain all receipts.

On Windows, the persistent browser process must additionally show the intended private `--user-data-dir` (documented descendant paths are allowed). Reject unknown or foreign live coordinates before UI observation. On macOS, record the deterministic root-derived store identifier, but do not treat that computed value as proof that WKWebView used it: the observed cross-profile draft test is required. Scoped read-only storage inspection may support the evidence; internal WebKit directory layout is not a portable contract.

If the candidate exits unexpectedly, process identity is ambiguous, storage crosses profiles, or an unsaved guard cannot be resolved from the current observed dialog, stop the UI sequence. Preserve the evidence. Never obtain a new binding to discover whether an exited app survived, launch without private arguments, use a production fallback, force quit, or send blind Escape.

Only after this gate passes may paired synthetic acceptance resume: needed-station display, fake radio/rotor operations, durable native queue and fake-owner receipts, receive start/stop with paced silence, and ordinary shutdown. Synthetic owners do not qualify the actual xLog database, physical station, DAX timing or decoding reception. macOS 14 runtime compatibility remains a separate gate from an audit on a newer Mac.

Existing default-store data and the `.1`/`.2` artifacts remain preserved. This fix does not silently migrate or erase a shared draft whose intended profile cannot be established.
