# Upstream update review — 14 November 2026

Disposition: do not promote this update yet. The current candidate remains
`c2c917b6f748366fbf6b3960876d9f98ba1854c9`, based on upstream
`c88740a94172b3e78ae587d62bd0a54394d1e85f`.

The isolated rehearsal merges upstream
`3885df0fe0d992a52f51ff5481025f860b2322f5` onto that exact candidate.
It has zero conflicts across 29 files (1,085 insertions, 140 deletions).
Eight fork-policy checks pass and all 44 inherited workflow jobs retain their
upstream-only repository guards. The pending merge is preserved uncommitted;
its retained patch SHA-256 is
`6582a067e9225ea43ac2778d84bde0ae6e215d3253517b1e695e4c77078a3c6c`.
These are merge and source-review results, not runtime qualification.

## Blocker: equivalent serial names can clear a stuck-key warning

In the proposed source, `crates/tempo-audio/src/control_line.rs:234` makes
`SharedControlLines::open_with` use `ports::same_port`. This correctly shares a
handle between macOS `/dev/tty.*` and `/dev/cu.*` names, or Windows `COM7` and
`\\.\COM7`. The new `held` method at line 257 still compares names only with
`eq_ignore_ascii_case`.

`CwStuck::proven_up` in `crates/tempo-audio/src/service.rs:20979` treats a false
`held` result as evidence that the port closed and released the key. If PTT opens
the first spelling and CW shares it through an alias, a failed CW key-up followed
by dropping the CW keyer can therefore clear the warning while PTT retains the
same open handle. The later-keyer recovery comparison at line 20982 also uses
the narrower spelling comparison.

The existing regression in
`crates/tempo-audio/src/service/tests/cw_shared_port.rs:496` uses identical
spellings and does not cover this case. Before promotion, make both ownership
checks use the same canonical-port equivalence as opening, and extend the test:

1. Open PTT with one spelling and the CW keyer with its alias.
2. Fail key-up, drop CW, retain PTT and tick the service; the warning must remain.
3. Release the last holder; the warning may then clear.
4. Cover both macOS twins and Windows prefixes, plus a recovered keyer using
   the alternate spelling.

This finding is from source review; that new regression has not been executed
and no patch has been applied to the retained merge. It affects the incoming
independent operating path. It does not establish a defect in the frozen
companion candidate, which does not initialize this local station engine.

## Other reviewed boundaries

- The companion branch in `src-tauri/src/lib.rs` still returns before local
  Engine/audio startup. Its native invoke allowlist excludes serial control,
  rigctld, settings-bundle restore and hosted streaming. Its UI entry imports
  the companion App and does not mount the independent LogConfirm component.
- The incoming rigctld retry is limited to transient process-spawn failures;
  it retains the nonexistent-rig probe and the settable/refusal gate.
- The stream helper extracts the existing relay-address predicate.
- LogConfirm retains its element/cleanup lifecycle, ignores its own DOM
  mutations and permits at most one scheduled animation frame.

No second concrete blocker was found in these reviewed changes. The review did
not run the full upstream suite, a desktop build, GUI or hardware tests. After
the serial fix, run the applicable upstream gates and companion regression
checks before merging, then reserve a new candidate version before packaging.
The current Windows and Mac build receipts remain tied to the frozen source.
