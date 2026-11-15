# Local one-period preparation — unwired prototype

Base: `75a59f786ee1f437e7ab14a147bb3ca0ceed1c49`. The pure preparation
adapter passed its six offline reference/boundary tests using the recorded,
source-equivalent Nexus DSP libraries. Locked offline manifest resolution and
pinned rustfmt also passed. See [VALIDATION.md](VALIDATION.md) for exact scope,
source hashes and preserved failed attempts. No audio device, PTT, HTTP request,
app GUI or production operation was used. There is no desktop dependency,
Tauri registration, bridge endpoint or capability.

This small standalone crate reuses original Nexus code rather than copying a
modem or creating a second station owner. `modes::wsjt77` public refusal/tones
functions are pure and have existing vendored golden coverage. Their `None`
case is refused here: no stateful fallback or `encode_later` memo work. The
adapter calls `tx_mode(...).gen_wave()` with validated FT8/FT4 tones, never
`Mode::encode()`. Engine, sequencer, audio output and xLog implementation code
are absent. `modes` still has transitive modem build dependencies; an unwired
crate does not imply a dependency-free build.

## Deliberately provisional local format

Every caller must supply a Format and Limits; none implements Default. Only
12,000 Hz, mono IEEE float32 little-endian is supported. This is NOT an approved
xLog output route or wire schema. The existing generators use fixed samples per
symbol (FT8 1920, FT4 576), even at another passed sample rate. Other rates are
therefore rejected instead of changing symbol timing or silently resampling.
S16 conversion is deliberately unsupported. A future approved owner format may
require a separately reviewed explicit conversion before content approval.

The output preserves Nexus's existing lead-in (FT8 0.5 s, FT4 0.3 s) and every
generated sample. It appends explicit zero samples to exactly 15 s/7.5 s; these
are protocol periods, not RX decoder frame lengths. No normalization, clipping,
truncation, inferred drive, passband compliance, RF timing or authority is claimed.
Caller offset/amplitude limits constrain content only. Format, message, offset,
mode and final bytes are bound by a versioned local content digest; a second
digest identifies the exact padded PCM. Private fields and read-only slices
prevent later mutation from invalidating that identity. Float/libm output can
differ across platforms; the digest identifies actual bytes, not a universal
cross-platform waveform hash or proof of the asserted message.

Text support is deliberately narrower than Nexus's packer. The packer's success
does not prove text fidelity: free text keeps only 13 characters, each token can
be shortened, and unsupported free-text characters can become spaces. This
initial slice therefore accepts only uppercase, single-space, three-token
standard type-1 messages: `CQ CALL GRID`, or `CALL CALL GRID/REPORT/SIGNOFF`.
Calls must be literal standard base calls (3–6 characters, no slash/hash); grids
are four characters. Reports are exactly `-30` through `-01`, `+00` through
`+49`, optionally prefixed `R`; signoffs are `RRR`, `RR73`, `73`. Public Nexus
`chkcall_ok` and packed type-1 bits are checked in addition to this narrow syntax.
No lowercase/space normalization is implicit. Free text (even valid short
text), compound/portable calls, directed CQ, hashes, contests and telemetry are
explicitly unsupported pending a separate canonical-text/preview contract.
The 37-byte guard alone is not claimed to prevent truncation.

Reject non-ASCII/NUL/control/blank/over-37-byte text before the truncating packer,
unsupported mode/rate/channel/encoding, encoder refusal/stateful fallback,
invalid tones, nonfinite samples/offset/limits, missing/shifted lead-in,
incomplete/oversize waveform, amplitude over policy and size arithmetic overflow.
Fixed FT8/FT4 sizes bound generation before it allocates the waveform.

## Offline validation boundary

Six test groups cover actual pure tones against the vendored test-only oracle,
sample-by-sample golden waveform tolerance, exact lead-in/trailing padding,
refusals and actual packer truncation/character-mapping collisions, the narrow
message forms and normalization rejection, explicit format/limits, corrupt-generator output and
digests over immutable final bytes. Vendored calls occur only in tests, with
generated content and no transport/device. The final run compiled the actual
adapter with pinned GNU Rust 1.93.1 and existing qualified DSP rlibs; it did not
compile all transitive Cargo dependencies. A generated lockfile and locked
metadata check establish manifest resolution separately. A clean Cargo build,
Clippy and platform qualification remain future gates before wiring this crate
into an application. Existing upstream tests are supporting source coverage,
not additional newly executed evidence.

## Next integration boundary

The familiar original Nexus FT8/FT4 cockpit should consume a future backend
adapter, not a new companion dashboard. This content module cannot arm it.
Owner commissioning, complete route/operation binding, single-use journal,
slot scheduler, independent release transport/watchdog and observed release
remain separate owner work. Sequencing can reuse `tempo_core::qso::Station`,
but `after_tx()` explicitly sets `keyed_on_air`: preparation or an accepted HTTP
reply must never call it. No automatic repeat, QSO logging or live TX follows
from these offline tests.
