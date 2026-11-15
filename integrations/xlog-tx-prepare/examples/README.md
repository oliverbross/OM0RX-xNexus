# Offline prepared-period corpus

`export_prepared.rs` calls this crate's actual `prepare()` for CQ, grid, report,
R-report, RRR, RR73 and 73 in both FT8 and FT4. It does not contain another
encoder, run the sequencer, register an app command, contact xLog, or open an
audio/radio device. The explicit 12 kHz mono f32le/1000 Hz parameters and limits
are corpus inputs, not approval for an operating route or drive level.

Supply a fresh absolute output directory, the exact source baseline commit, and
the actual exporter and producer source SHA256 values. These caller-supplied
provenance values require external build verification: the exporter cannot
attest its own compiler or source bytes. The reviewed runner additionally binds
the dirty exporter source, toolchain, dependencies and compiled executable.
No uncommitted exporter is represented as being included in the baseline commit.

Each case has a JSON manifest and the exact prepared PCM bytes. `inventory.json`
is created last, without replacement, and is the completion marker. Failed or
partial output is retained; a directory without a complete validated inventory
is not a successful corpus. Reusing an existing output directory is refused.

The manifest describes the proposed owner offline validator's schema 1. This is
not a transmitted request, approval journal or stable TX wire contract. The
producer's `content_sha256` binds its documented local period identity. It does
**not** bind every manifest field. Therefore the inventory separately records
the exact manifest hash and a length-delimited manifest-plus-PCM body hash under
the expressly offline `xnexus-prepared-corpus-body-v1` domain.

`verify_corpus.py` independently checks all 14 intents, strict file/field sets,
manifest/PCM hashes, both digest encodings, finite bounded samples, original
lead-in, waveform bounds and full-period zero padding using Python's standard
library. It is not a second encoder and does not prove owner acceptance, message
decoding, scheduler correctness or RF operation.

The bounded offline check passed on 2026-11-15: fresh producer/exporter compiled
against source-equivalent, previously qualified GNU Rust 1.93.1 DSP libraries;
locked offline metadata and formatting passed; both exporter tests and all 14
independent corpus checks passed. No hardware, application, owner validator or
transmitter qualification is claimed.

## Preserved evidence

Baseline commit: `984fbaadd75cc5f9d53cb3d6af9ce44fe5ae285d`. The exporter was
uncommitted at validation; the corpus records its actual SHA separately:
`2BFD727DCEA7374D00503C618208EBFC40967BBC14803938687075E3134418DA`.
Producer SHA: `D7F2B5D325A958455FBEF86D5EAB262C0370E2AB1F616D40DCBD2DD406B6F53D`.
Independent checker SHA: `B3DD78D0C2FBC80618ADECD4DA19BF787D2E456C862485FA20FFD8A561F24EEF`.

The original local evidence remains in the `tx-prepared-corpus` worktree at
`target/attempt-45f5d3e0131c4c5ab0446cd760772e45`, including the raw `corpus/`
manifests and binary PCM (not committed to Git):

| Record | SHA256 |
| --- | --- |
| `receipt.json` | `6C7F2F608E8D4A9AC8B5356C2460FDA959D8E7177B532AFCC4D57DDF32966839` |
| `corpus/inventory.json` | `B77DAE43D212848E65CA7D3116AF47A361B35B42825C490B8C6B7B6FCA9BA481` |
| `independent-validation.json` | `44C7D15CFCD5C64DD66C4876EBC4C94CD3059468115EAB6F1D48F3C27680816E` |
| `compiled-artifacts.json` | `DC2DA79CFB6CD544612E79824B600FA9D4F66A6281E4C2F60C3AE0F1773F6D1C` |
| `export_prepared.exe` | `00160C009DDAE3427010D121CB2FC0E117F2CE951F168ABCCF2E0F7AF6D37694` |
| `metadata.json` | `661CD6E503FD417247BB524EE43FA2CD566232F1045C8AC179001ED8DB4F32AC` |

The before/after source inventories both hash to
`C0C4E7BAE6EE1F81287C5F085CB60D145654FCA374E79D28B58867B1D2E5E4B0`.
Only this evidence documentation changed afterward. The private Windows Job
allowed 2 GiB total descendant memory, BelowNormal, one compiler job, 300 seconds,
8/6 GiB physical/commit admission and 2/2 GiB runtime floors. Peak was 535,638,016
bytes; root exited 0 with zero descendants, natural drain and cleanup confirmed.
The preserved runner/tool/dependency pin inventory is alongside the receipt.
