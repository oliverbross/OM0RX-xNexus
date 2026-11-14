# xLog native reference client

This native Rust client implements the [bridge contract](../../docs/xlog/PROTOCOL.md)
for the companion desktop: observation, owner-mediated controls, history and
durable logbook operations. It cannot open a database, radio, rotator or cluster.
Canonical xLog integration and paired application acceptance remain release gates.

The additive `audio_handle()` requires the owner's `audio.rx` capability and reads
selected-channel mono PCM on a separate native worker. It never starts capture or
opens an audio device. The xLog operator must explicitly map FLEX slice A to its
observed DAX channel and stable Windows input endpoint. There is no channel-1
default. Keep audio polling outside the ordinary observation and Stop mutexes.
Bridge disconnect/replacement/drop invalidates its audio handles and queued
batches. Receiver errors/disconnect/drop invalidate that receiver's batches.
The worker must also discard partial periods on idle expiry and fence late decoder
results against the desktop session generation.

Audio responses are bounded to 512 KiB, 32 chunks and 262144 decoded bytes, with
exact cursor, frame continuity, route, identity and freshness checks. A read error
invalidates the handle until explicit reconnection. Consumers must check
`AudioBatch::is_fresh()` before consuming queued data and discard partial periods
on `reset`, expiry or any error. Samples stay at the owner's native rate; the
downstream assembler must not fill gaps with generated silence.

`sample0_unix_us` refers to Windows endpoint capture. The reported uncertainty
bounds only the host clock correlation; UTC synchronization accuracy and remote
RF/DAX latency remain explicitly unknown. This receive-only extension grants no
transmission authority. Native worker/decoder/UI wiring and physical DAX receive
qualification are separate from these protocol tests.

Run synthetic tests from the repository root:

```
cargo test --manifest-path integrations/xlog/Cargo.toml --locked
cargo clippy --manifest-path integrations/xlog/Cargo.toml --locked --all-targets -- -D warnings
cargo fmt --manifest-path integrations/xlog/Cargo.toml --check
```

Once a qualified xLog bridge exists, its explicitly paired endpoint can be probed:

```
cargo run --manifest-path integrations/xlog/Cargo.toml --locked -- --endpoint http://127.0.0.1:PORT --token-file PATH
```

The token file must be outside the repository with user-only access. xLog will
own pairing/revocation; do not extract tokens from its private settings. Never
pass a token directly on a command line. Errors omit response bodies and secrets.
The probe prints aggregate counts only. The library does not persist snapshots.

Tests use only ephemeral loopback servers and synthetic data. A passing test is
not evidence of a real bridge, paired desktop, or hardware qualification.
