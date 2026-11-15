# Original waterfall UI boundary

`spectrumApi.ts` reads only `xlog_rx_spectrum({after})`. A per-host reader binds
each request to the native current RX context, paired owner and local lifecycle.
Absent legacy `current_context` is unavailable, not an invented worker identity.
The reader validates the exact frame shape, finite bins, format/timing metadata
and full owner/worker/route/mode context before returning the original spectrum.

The display deadline is monotonic request START plus the original remaining
validity, capped at500 ms. Duplicate cursors cannot scroll or renew evidence.
Stop/re-pair/route/mode/epoch changes retire pending answers and retained images.
Callers synchronize context at intent and inspect the reset token on a timer;
the one-shot delivery reset flag alone is insufficient. Serialized frames are
snapshots, not revocable live references. No PCM, device, Engine, spectrum
generation or substitute bins are added in JavaScript.

33 focused tests passed, including exact Rust-produced JSON replay, stale or
foreign context, pending lifecycle changes, duplicate/expired evidence and
monotonic request-start bounds. Receipt in this worktree:
`target/spectrum-context-check/attempt-6c2cd35abb47419f98609f50c66e9be3/receipt.json`,
SHA256 `19933479B4731316762ABC18045F07449C4D7364955B811210CE99D2B8135BB6`.
The earlier sandbox esbuild-spawn failure remains preserved separately.
The approved2 GiB guarded run retired naturally with no remaining processes.

The exact frontend files also passed integrated TypeScript (`tsc -b`) and Vite
in `original-xlog-host/target/host-types-vite-28e94c378e0041df91f1cad0abf0755c`.
Receipt SHA256 `5679771FE25F3965136C987516ECD32212C74E1867FBEC623AC1B3CCB4B05A55`;
all2,589 inputs stayed unchanged, root and descendants retired normally.
The actual Rust fixture provenance is recorded in `RX_CURRENT_CONTEXT.md`.

Original-host browser integration is separate evidence; this helper commit does
not register a new app entry, package/install an application, qualify real
capture/hardware or complete TX. Unknown host UTC synchronization and RF delay
remain unknown.
