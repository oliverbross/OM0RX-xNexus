# Original global rotor strip

Base `0829c6f5bb4223c27474cc0ef73af16602f0032b`, plus this isolated contribution;
no borrowed source overlays. The qualification below uses synthetic owner
observations and actual original UI components. No native app, live owner
request, radio or rotator was operated.

The original strip already had an adapter fallback that preserved Stop without
azimuth. This change does not claim to fix a missing fallback. It moves the actual
strip once to the original host's existing global status/header, leaving it
reachable on Spots. An explicit external cockpit flag suppresses its second
internal mount only for this host. Native/default cockpit placement is unchanged.

Owner rendering accepts typed nullable observations, never native settings,
satellite or declination DTOs. Its only timer retires displayed telemetry at the
existing snapshot deadline. No owner/native polling occurs inside this branch.
The original native branch remains separate and retains its readers and tracking
Stop semantics. Owner Stop uses the existing rotation-only label; it does not
claim to stop a Nexus satellite tracker.

`createOwnerRotor` retains the exact API, opaque native connection token, local
session, owner server/workspace and rotor ID. Each Stop closure captures this
immutable target. Replacement cannot retarget an older closure. The retained
target survives expiry, temporary transport failure and ordinary command pending;
connection intent, API/token/owner/workspace/session changes retire it. Never-seen
targets are unavailable. A later genuine target replaces the old one.

`useOwnerConnection.isCurrentConnection` checks actual synchronous epoch/API and
the latest accepted owner identity before React renders. Connection intent clears
it before awaiting; ordinary command admission does not. Stop checks this seam
immediately before the existing independent command call, with no intermediate
await. The native owner remains final route/capability admission authority.

Receipts retain their existing status display. No angle equality is a causal
completion acknowledgement, no command is retried, and Point/bearing is not added.
The separate pure control-observation helper is not wired by this slice.

Qualification on 2026-11-15:

| Gate | Result and evidence |
| --- | --- |
| Focused owner/hook/host | 24 passed: full binding/ABA, same-turn reconnect, stale/missing telemetry, independent Stop during stalled ordinary dispatch, poll failure and global Spots reachability. |
| Original native strip | 12 passed: existing satellite/dial/long-path controls. A separate source comparison verifies its entire 18,267-byte body unchanged apart from the function name and normalized line endings. |
| Catalog | All 376 hardcoded-string and 30 placeholder checks passed. No catalog budget or translations changed. |
| Full UI | Actual `tsc -b` and Vite production build passed, with 41 generated assets. |
| Private browser | All nine cases passed: six actual OriginalHost cases at laptop/floor/UHD200 in dark/light themes, and three original native RotorStrip positive controls. |

Browser checks use actual original CSS/viewport behavior and private contexts.
The global rotor group appears exactly once, stays outside the hidden cockpit on
Spots, and its Stop is visible and hit-testable before and after expiry. A
synthetic acknowledgement does not restore expired telemetry. Replacement routes,
unresolved reconnect retirement, the new exact connection target and capability
loss are checked. Native controls still stop satellite tracking before the
rotator; the owner branch makes zero native calls. All cases have zero forbidden
operations, external requests, page errors, console errors or warnings. There is
no global OS input, desktop helper or hardware interaction.

Evidence is retained under this worktree's ignored `target` directories:

* `rotor-focused-preparation/attempt-34a44229e56247eebd3a99fb3c1efe51`:
  receipt `71DCDBF392EA293185DBA2C3C7E1C1CF4F1FC019E9BC5899281B7E9835764180`;
  24/12/406 results, unchanged source manifest
  `869810BA4D8D61D5CBDC97D1E965BD0AA5F048F003A8AA1ACB83A0FF93D11CC9`.
* `rotor-types-preparation/attempt-c5d6b773702d43698aecaee71c509a3a`:
  receipt `E91A2335B2614C7AAB425299A2D6AAD9B8F6BCFC0D42CCD3133D31DBBAC837FC`;
  generated assets `FC5C3F788B08EEA7A814A0432CD0ECB84DAEE8E5B203F29BA4CA344BC319CE38`.
* `rotor-browser-preparation/attempt-7544c6f4d1ee4c529640a2cd4fe49a3b`:
  receipt `445EB6A34373CFF6A177B3AE7A15C0C35ABE9A282AB4C1544A5E6C7D59E1D99F`;
  nine-case result `B30B68EFED714AB7B432D6E0A357BA3DD94FC1DA4EF404A423294B35E6F3EE82`.

The final full-UI and browser runs verified all 2,594 input files unchanged:
`8DAA9C616F933D451596BA629757351311D0EA7E377CDBCFEDCD17D33B2295CE`.
All passed runs exited naturally with zero root/descendant processes, confirmed
cleanup and no memory/abnormal-exit notifications. Peak job memory was
914,100,224 bytes (focused), 2,623,180,800 (types/build), and 1,450,049,536
(browser). The private browser also exited normally with code zero.

Preserved earlier failures are not relabeled as passes:

* Focused `attempt-778a0f9a59f34787a436e1c853955d14`, receipt
  `EAE378E9BFF9E98B5BD523B8EA2A7143564173B59494D9C5A5B7D2026FF0205A`:
  23/24 passed; an unregistered `toBeDisabled` test matcher failed. The exact
  disabled-property assertion replaced it; the separate focused run above passed.
* Types `attempt-2951e4e8893c45d981c24b7f1856d049`, receipt
  `4C99311E621917402EBBB843D7114BD021B48BACED7A0601E77D493058A8F2F8`:
  two fixture assignments violated readonly `SnapshotLease.deadline`. New lease
  objects replaced those assignments without changing the scenarios/assertions.
  Full types/build and browser then passed. Runtime bytes were unchanged from
  the focused pass; the focused suite was not repeated for this typing correction.

Original failed test bytes, tool/source pins and receipts remain retained. Each
follow-up was separately reviewed; no deadline, memory cap, assertion or product
freshness rule was relaxed. The D378 private Job guard used fresh 8/6 GiB
physical/commit admission, 2/2 GiB floors, BelowNormal priority and a three-second
natural drain. Focused tests used 2 GiB/300 s; types/build 4 GiB/600 s; browser
2 GiB/600 s with the unchanged private browser runtime `45024029…`.

These are source/UI checks, not native installation, hardware Stop or movement
qualification. Point/bearing and physical outcome reconciliation remain separate.
