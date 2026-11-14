# Companion profile isolation failure — 14 November 2026

The `.1` and `.2` Mac candidates are not qualified for separate-profile use.
The `.2` Windows candidate must not be launched or installed for acceptance until
the shared construction defect is corrected. Preserve the artifacts and failed
acceptance evidence; do not rewrite their source or version receipts.

## Observed failure

The exact Mac `.2` binary from source
`c2c917b6f748366fbf6b3960876d9f98ba1854c9` passed its supplemental artifact audit.
It then launched explicitly with a newly created private profile. Its executable
hash, live PID, complete arguments and distinct computed store UUID were verified
before native observation. The fresh profile nevertheless displayed the synthetic
`K1ABC` draft entered during the earlier `.1` acceptance sequence.

Acceptance stopped before selecting a pairing file. No token was read, QSO
submitted or station command sent. The observed file dialog was cancelled, the
application exited through ordinary Quit, and the synthetic owner exited through
its own STOP marker. Process-only checks confirmed both exits. No further
application binding or fallback launch was performed.

The earlier same-profile draft persistence observation remains factual, but it
did not prove isolation between different profiles. Its acceptance claim is
superseded on that point. The `.2` artifact audit establishes file properties,
not correct native storage behavior.

## Source diagnosis and correction boundary

The locked desktop dependencies are Tauri/tauri-runtime 2.11.2 and Wry 0.55.1.
`xlog_mode::run` assigns `WindowConfig.data_directory` and, on Mac,
`WindowConfig.data_store_identifier`. In the locked tauri-runtime source,
`From<&WindowConfig> for WebviewAttributes` omits both fields and leaves their
defaults as `None`. Tauri's configuration-based window creation consumes that
conversion. Computing distinct identifiers alone therefore does not establish
that the native WebView received them. The omitted directory also affects the
Windows/Linux construction path; no Windows xNexus native launch was attempted.

The correction must prevent automatic creation of the configured companion
window and create it explicitly with the WebView builder's storage setters,
retaining its label, window options, companion-only URL and restricted commands.
Keep the existing profile lock and unsupported-macOS rejection before any WebView
creation. Do not upgrade dependencies or silently migrate/delete the shared
store as part of this repair.

## Required regression evidence

- The actual window-construction boundary receives the explicit private data
  directory and, on supported Mac, the deterministic store identifier.
- Two fresh private roots start with empty independent drafts. A distinctive
  draft saved in root A is absent from B; returning to A restores it.
- Native process identity and complete private-root arguments are verified
  before every binding. On Windows, live WebView descendants must also identify
  the intended private user-data directory.
- Each launch ends by ordinary Quit with process-only exit verification. Failed
  launches are retained and never replaced by a default-profile launch.
- Only after isolation passes may paired synthetic controls, QSO queue and RX
  tests resume. Physical station acceptance remains separate.

Any newly packaged correction requires a new visible candidate version and an
exact clean-source build receipt. Existing `.1`/`.2` artifacts are retained as
failed-isolation candidates and are not distribution-ready.
