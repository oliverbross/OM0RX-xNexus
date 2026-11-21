# 1.25.0-xlog.21: FlexRadio band changes reach the radio through SmartSDR CAT

Source: `main` at `615d2ebb3`. That commit merges `xnexus/flex-smartsdr-cat-default`
(`ff481cf8c`) onto `6f1717bba` (1.25.0-xlog.20).
Upstream base unchanged: kd9taw/Nexus `39b0e946f` (1.25.0, see `UPSTREAM.json`). Identity
unchanged, so an installed xlog.20 keeps its logbook and settings.

## What changed for the operator

A FlexRadio set to Hamlib's native SmartSDR backend (model 23005, "SmartSDR Slice A") drove the
slice at internal index 0, not the slice lettered A. When slice A sat at another index, Nexus
read and set nothing, and the radio stayed on its band. On Windows, every radio profile that uses
23005 now moves once, at load, to FlexRadio 6xxx through SmartSDR CAT (Hamlib 2036,
`127.0.0.1:5002`). SmartSDR CAT serves the slice by its letter. If the old address held the
radio's real IP and the profile had none saved, that IP is kept as the radio's address. A marker
(`flexSmartsdrCatMoved`) stops the move from running twice, and a settings form can't clear it.
Nothing changes off Windows, where SmartSDR CAT does not run.

The radio-model short list no longer offers 23005. It stays in the extended list, so a saved
profile still shows its name.

## Where it is

| Part | Files |
|---|---|
| One-time move and marker | `crates/tempo-app/src/settings.rs` (`move_flex_to_smartsdr_cat`, called from `load_at`) |
| Marker kept across Apply | `crates/tempo-app/src/engine.rs` (`apply_settings_inner`) |
| Model lists | `crates/tempo-audio/src/rigmodels.rs` |

## Verification (before packaging)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` (merged `main`) | pass |
| `cargo test -p tempo-app --lib flex` (merged `main`) | 14/14 |
| `cargo test -p tempo-audio --lib rigmodels` (merged `main`) | 36/36 |
| tempo-app `settings` module (branch) | 329/329 |
| tempo-audio lib (branch) | 1003 passed, 4 failed; the same 4 fail on untouched `e37218c6d` (pstrotator, port_prober, two rigctld_proc tests), so the cause is this machine |
| Live | the operator's installed Nexus on 2036 at `127.0.0.1:5002` connected and followed SmartSDR (7.158 MHz); this was set by hand before the build |
