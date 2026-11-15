# Optional owner station identity — client validation

Base: `75a59f786ee1f437e7ab14a147bb3ca0ceed1c49`. This additive client field
uses the owning snapshot's session/workspace/freshness contract. It confers no
transmit authority and introduces no UI rendering or fallback station identity.
An older owner can omit it. Native `xlog_mode::ClientView` already serializes the
typed SDK Snapshot, so no parallel native DTO is introduced.

Validation requires schema 1, saved_station_settings, normal_logging, exact
workspace binding, nonnil UUID identity/workspace, positive safe worker counter,
safe revision counters, canonical literal call/grid, coherent current/incomplete/
unavailable states, and transmit_authority=false. An identity change on the same
base snapshot revision is still a protocol violation; this field is not exempted
from the base immutable snapshot check.

## Recorded checks

Final guarded run: `target/station-identity-check/attempt-201ce5409faf458bbe3b4e04aa8c690a/receipt.json`.
SHA256: `7A449DB2B57459D68CCEBE8DD68B990E8DBC3F6CFA2E2C0333B845B18E00BF89`.

- 85 SDK tests passed (two intentionally ignored opt-in fixtures).
- The opt-in actual owner projector replay separately ran and passed once. Its
  six owner-produced cases use synthetic saved settings, not production identity.
- Pinned GNU Rust/Clippy 1.93.1 all-targets lint passed with warnings denied.
- Only changed SDK files were formatted with pinned rustfmt; source manifests
  and tool/fixture hashes matched before and after the final run.
- The integrated frontend run reported by the cockpit agent passed TypeScript
  and Vite with the exact TS DTO file below. Native application/package and
  physical station acceptance are separate gates.

The final job had 2 GiB memory/one job/BelowNormal, a 300-second deadline, fresh
8 GiB physical + 6 GiB commit admission, 2 GiB runtime floors, and a three-second
natural descendant drain. It exited 0 naturally; root exited, active processes 0,
cleanup confirmed, peak job memory 515,174,400 bytes, no memory/abnormal flags.

Actual owner fixture:
`C:/Projects/OM0RX-xLog/.build/xnexus-identity-tests/native-20261115-100824/station-identity-fixtures.json`
SHA256 `3557063A96591AA695224780D2725393E356B7A689DA15789B6B33FC59678D9E`.
The ordinary unit fixtures are explicitly handwritten and are not relabeled as
owner-export evidence.

Final source SHA256:

| File | SHA256 |
|---|---|
| integrations/xlog/src/lib.rs | 31AAED1B696B68F4E8B83FCDBE3EEEBCD63AA1F9BE9A9F8F61A42C3FA75782AE |
| integrations/xlog/src/station_identity.rs | 22D03769D6976A09A64F02607B74E7D8AD168C7B5BF8521B1B3EA3A0E87A617A |
| integrations/xlog/src/station_identity/tests.rs | 65FF1A868653EE78EB5251B19ADCB2E3D0DF5A6720AA102BD4F3E20812BE4900 |
| ui/src/xlog/api.ts | 03883035A220403BAE98D5E719FF85DD9B62CD2B5C3A63FCA4396381163BF727 |

## Preserved earlier attempts

All are retained under the same ignored `target/station-identity-check` directory.
The first attempt passed tests/replay but reached the 300-second deadline during
Clippy dependency work. The next Clippy continuation selected the wrong rustup
subcommand toolchain and failed with incompatible cached metadata. The corrected
absolute cargo-clippy/driver invocation passed; pinned rustfmt then required the
final source-matched repeat above. These were not reclassified as complete passes.
No compiler cap or deadline was widened, no global rustup/PATH setting or cache
was changed, and no device, app GUI or production profile was opened.
