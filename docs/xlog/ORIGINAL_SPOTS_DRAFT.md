# Original Spots surface — implementation and validation

The actual Nexus `SpotsPanel` is now available through the original host's Spots
navigation with xLog-owned observations. It retains the original table, row
cells, search/filter controls and CSS. The existing native props and behavior
remain the default for CW, Phone and Connect callers. External rows are never
cast to native `SpotRow`, `NeedAlert` or `PropagationSnapshot`.

## Data and action boundary

Only the authenticated SDK snapshot supplies observations. The adapter preserves
stable spot ID, exact Hz/mode/source, optional country/band and the owner's need
status, score and reasons. Unknown assessments expose no score or inferred award
tags. A zero timestamp, a time later than the snapshot, or negative elapsed wall
time displays unknown age. Display age never grants a source lease.

Admission and retained actions require `cluster.read`, the current connection
token/session/owner/workspace, the original snapshot and its unchanged deadline.
Invalid monotonic clock readings cannot admit facts or selection. A 100 ms
repaint withdraws expired rows; an action checks freshness immediately.

External headings say Source and xLog assessment. Current-empty means no
observations supplied, not proof of a healthy cluster or no needed stations.
Unsupported native licence, reporter-locality, state and worked-date filters are
absent and cannot hide owner rows. Owner filter storage is separate. Explicit
capabilities fence native pounce setters, centroid reads, call-card/menu, post,
Spot me and pop-out before invocation. Row selection is read-only, carrying the
exact source row/Hz to the host callback; no tune, posting, retry or TX is added.
Country-directory coordinates do not become precise station positions. Map and
NeededPanel remain outside this slice.

The original host mounts neither App nor ConnectView. Its existing hidden
keep-alive wrapper preserves the cockpit draft/layout; inactive cockpit readers
pause, and page changes retire the waterfall picture. The FT navigation item
returns to the same mounted cockpit. RX ownership and global Stop remain above
both views. Browser checks exercised that Stop after source expiry as well.

## Exact validation composition

Base: `29bc76120525e575d1d1123c5892f956cb1ee224` on
`codex/original-cluster-surfaces`. This logical commit contains only the Spots
contribution. Validation additionally borrowed the already-reviewed shared fix
`6b40a539b941b5e3bf0e630b0a21481e83807bef`: four older owner labels in eight
catalogs/two CSVs, and an equivalent declaration for a history-reader helper.
Those eleven-file deltas were applied **unstaged after staging the Spots changes**
and are excluded from this commit. Parent integration must retain both changes.
Canonical source, installed applications, live owner and devices were untouched.

Evidence paths below are relative to this worktree. Source manifests include
all UI source and configuration (2,457 files for final type/browser gates), tool
hashes, the borrowed commit and staged/unstaged composition. Before/after runtime
source matched; only this validation document was finalized afterward.

| Gate | Result | Evidence / receipt SHA256 |
| --- | --- | --- |
| Owner mapper, actual panel and navigation | 29 passed: 20 source/lease, 8 panel, 1 host/navigation | `target/spots-focused-preparation/attempt-e7d186aaf1f14c2cae677007b7eea0a6/owner-spots.json` — `110DDA9D933558D8CCD5E4A688B8AFFDF0AE94D04C0B5581804B9E999CC48045` |
| Existing native Spots behavior | 50 passed across 8 suites | Same attempt, `native-spots.json` — `6746A5DAF35FE071C7D34ED04776EE04F4250B188A26F34823400E087AEC86BB` |
| Actual catalog guards | 406 passed: 376 hardcoded-string + 30 placeholder | `target/spots-catalog-followup/attempt-aa977dfea4894bf58dc07081422da756/receipt.json` — `DDFEC0744DB75773B18EC81B9A26DEAB6DBDA5BB142DCA71067DB5A6B4174AC5` |
| Full existing `tsc -b` and Vite production bundle | Passed; 41 generated files | `target/spots-types-preparation/attempt-5e6b898799bc44188a921ff3e4a58a99/receipt.json` — `A1310CCAE730ECD3405C8EFB566528826F4AC618A9512A55863B19721DFF4582` |
| Actual original-component private browser | 12 passed: 6 owner + 6 native | `target/spots-browser-preparation/attempt-8f50bfdae8ed4876ac4e5b8c4ed16e06/receipt.json` — `CCB477C67E948DD4ECCE96A119BCF590C0AC7283C55760636162A6167DBFCC12` |

Final type/browser source manifests matched
`7C24C15A08A5A5DDB895BAF144C29C9123D6A88A0245E62B4552C36CFA475388`.
Vite output inventory:
`673334DAF9149BEF095B2BE2E2095B9ABC4095558494C44C7DDBE6B5A96540CA`.
Browser result inventory:
`424893343CA03BC4800881BD0F672836A22FEADFC57A52B2B1578D9FFD424076`.

The browser used actual original components/styles/useViewport with explicitly
synthetic service answers. Cases covered LAPTOP 1366×768, FLOOR 780×438 CSS
(1366×768 at175%), UHD 1920×1080 CSS (3840×2160 at200%), dark/light, original
navigation/draft retention, exact ID/Hz selection, unknown-score/future-time
suppression, empty/capability-loss/expiry, reachable global RX Stop and native
row Work/spot-review defaults. No RF, PCM or real database/owner behavior is
claimed. No forbidden native method, mutation, external request, page error or
console error occurred. Existing native SpotDialog description warnings were
recorded separately. All owner cases had zero page-level horizontal overflow.

Root visually reviewed the original laptop table and passing floor dark/light
filter screenshots. Representative files under the passing `screenshots/`:
`0-owner-dark-current-initial.png`, `4-owner-dark-current-initial.png`,
`6-owner-light-filters-selected.png`, `10-owner-light-current-initial.png`,
`4-owner-dark-expired.png`, `4-owner-dark-stopped.png`.

All runs used private Job Objects with source/tool pins, fresh8/6 GiB physical/
commit admission,2/2 GiB floors, BelowNormal and3-second natural descendant drain.
Focused/catalog profiles used2 GiB/Node1024/300s; full types4 GiB/Node2560/600s;
browser2 GiB/Node1024/600s. Corrected D378 guard and explicitly pinned bundled
PowerShell were used. Successful type peak:2,651,508,736 bytes; browser peak:
1,592,041,472 bytes. Both root and browser exited normally with0, final active
processes0 and cleanup confirmed. Private headless CDP never used global desktop
input, cursor APIs or desktop-control helpers. No automatic retry or cap widening.

## Preserved failures and corrections

- First focused run:29+50 behavior tests passed; catalog395 passed/11 failed.
  Computed translation keys hid seven new labels/two existing headings from the
  literal extractor; five calls became literal without text/condition changes.
  The older shared label/helper failures were fixed separately in6b40.
  Receipt `62735D70D371607D83452AD55639EB7182028ECF5EA24AAF43CE39F8C25B97EF`.
- First full TypeScript run: two new test role queries used unsupported
  `exact:true` options. Removing only those options preserves the already-exact
  string-name matching and assertions. No product correction was needed.
  Attempt `8283c8b3324742fdaf5dc263344a9236`, receipt
  `1068A722CA8ED39E49B9681037A36B92FC9A8FFFA43451D98731820D2A13E5E0`.
- First browser attempt `5b73a323666a4a03a0c86002a7f14a2b`: a diagnostic assignment
  returned a DOM object to CDP's returnByValue serializer. Wrapping it in void
  fixed the probe only. Receipt
  `63412947FB7CC418584CECC24EA02C284D943CCFCE85A1B5F0BC92416E35F579`.
- Second browser attempt `578a249b132c466abb19cca825698c24`: a CSS suffix was
  accidentally appended to a JavaScript element expression; querySelector fixed
  that one probe line. Receipt
  `61FE1DB7C19B66C2BE6AC9F037B1C02D59CE1EA3213DEFACDF3E365B764A81EA`.
- A no-browser expression-check preparation initially expected one delegated
  expression, overlooking the wait delegate. The checker was corrected, then
  parsed/compiled71 actual generated expressions without executing them.
  Evidence `1DF38FA3DAA5F7ECB9DE6E20B31F220C3F80C0003D77FC74BDAA184E5D998AEE`.
  This was not a third failed browser run; all three actual browser attempts
  above (two failures and the final pass) are preserved.

Each corrected run required explicit separate review/release. Failed source,
receipts, logs and available screenshots remain in their unique attempt paths.
Product source and all geometry, lease, ID and Stop assertions stayed unchanged
through the browser corrections. No failure is relabeled a pass.

The ten Spots labels have eight translated catalogs. Chinese CSV additions were
converted with the existing converter/Node24.22.1; only their ten generated rows
were applied, preserving forty unrelated entries omitted by the older full CSV.
Full generated outputs are retained. Initial Node20 conversion failed before
writing because it cannot import TypeScript. Runtime English fallback is unchanged.

This is qualified source/UI integration, not an installed release or live-owner
acceptance. Actual owner-produced cluster JSON replay remains a separate gate;
these new UI fixtures are explicitly synthetic. No version or package was created.
