# Receiver history and classification continuity

The installed `.9` source coupled passive display to short audio/query leases.
`State::reset` cleared decoded rows and retired the FT coordinator on ordinary
audio expiry, continuity gaps and decoder backlog. Both SDK pollers also
disconnected their reader/session on temporary transport failure. Needs display
expired within two seconds despite retaining up to 200 rows and querying only
32 distinct keys per second. Faster refresh alone could not keep those facts
visible.

This change separates partial DSP validity from decoded history. Temporary
audio gaps discard partial samples, queued old decode epochs and spectrum,
while retaining the bounded row history and original Station/QSO. A different
route, namespace, mode or explicit Stop still retires it. Work and new decoded
periods retain their existing live admission checks; no expired audio grants
control authority.

Needs gains a display-only revision/session fence. It retains validated owner
facts across missing/expired metadata and temporary reads, while a changed
logbook/classification/country/policy revision, session revocation or explicit
pending-write fence withdraws them. `remaining_valid_ms` still reaches zero;
the live SDK APIs and control deadlines are unchanged. Native `display_valid`
is only for passive badges. The receiver hook no longer hides those badges on
a temporary station observation error.

The optional production `autoStart` hook starts RX using remembered FT8/FT4
mode after the connected owner becomes ready. A not-ready Start is retried by
the existing status poll; an ordinary Stop remains stopped for that session.
Connection discovery and owner capture configuration are separate changes.

## Focused validation — 16 November 2026

Base: `7416e3490f00bb759a4c0511c8ffa246e4509f38`, isolated
`codex/receiver-continuity`. No version or application profile changed.

Five selected tests passed using freshly compiled SDK and actual native
worker/FT modules: station/audio transient failure and recovery followed by
authentication retirement; expired passive facts followed by revision/write
invalidation; recurring DSP resets preserving rows/facts; genuine route change;
and retained QSO with stale Work rejection and full retirement. This was direct
GNU 1.93.1 compilation, not a full Tauri build or a complete test-suite run.

Private evidence under this worktree's `target/continuity-attempt-67ce16736df34d9bae0d52f2b71970cb`:

- Receipt SHA256 `CE4160645008B10013B6544504C135FBE7CD4930DFDED32F995933D9FC8BAF94`.
- Guard SHA256 `90B3334361CD5870D439601DE5B8A6D05DF860F0A38293999AF2AF89E98C50EF`.
- 2 GiB / 300 seconds, one job, BelowNormal, 8/6 GiB admission,
  2/2 GiB runtime floors, five-second natural drain.
- Root 4420 exited zero; final owned processes zero; no forced cleanup or
  resource flags. Peak 979,951,616 bytes. All pinned inputs unchanged.

Seven actual receiver-hook tests then passed serially with fake native APIs,
including passive display through stale observations, invalidation, remembered
mode, not-ready startup retry and manual Stop. Private evidence is
`target/receiver-hooks-prep/receipt.json`, SHA256
`EBC5227DE872E6755D865CC6E01DCD1345870AAA6DE8328E17A362835991D7BA`;
results SHA256 `55D3B3C6A4B706D3E06DAB25BA10268E0DDB150876EF6085F7F59E4655D5F7CB`.
The 1 GiB / 120-second job retired naturally with zero owned processes and
327,774,208-byte peak. Source pins remained unchanged.

After these gates, only three obsolete assertions in unselected worker tests
were reconciled with the new passive-history semantics; runtime bytes were not
changed. Those tests were not additionally executed. No app, browser, hardware,
PTT, package or installation was launched for this qualification.
