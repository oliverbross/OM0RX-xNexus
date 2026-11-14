# xLog native reference client

This is a read-only Rust client for the **draft** [bridge contract](../../docs/xlog/PROTOCOL.md).
It is not yet wired into Nexus's desktop and the inspected xLog build does not
implement this endpoint. It cannot open a database, radio, rotator or cluster.

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
