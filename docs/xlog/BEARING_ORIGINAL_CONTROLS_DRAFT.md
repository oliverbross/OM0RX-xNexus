# Original rotor bearing controls — qualified frontend slice

Base: `29ad2ed5afc1b2887fe9a71b538b7208b102400c`. The focused, TypeScript and original-component browser gates below passed. No physical rotor request, native GUI, package or hardware validation is implied.

The existing global original RotorStrip gains an explicit selected-call bearing read and its original short/long-path button presentation. Reads use only `xlog_bearing_read`. Point uses the existing admitted `observedPoint` command path with the exact owner angle and captured rotor target; no local resolver, reciprocal calculation, automatic move or native-service fallback exists. Stop retains its separate exact-target path. Dispatch receipt and later noncausal observed target state are displayed separately.

The reader validates the native response and strict nine-key owner envelope, including full saved-station binding and origin grid. Its deadline is the minimum of the captured snapshot deadline and IPC-request-start plus the native remaining lease. Same evidence cannot extend that deadline. Selection, source, identity, capability, synchronous poll failure and clock failure retire facts. A unique object identity fences late replies; a captured result identity also rejects an old Read closure after selection retirement before React updates its input. Explicit retirement preserves historical Point settlement while retiring observation authority.

`ui/src/xlog/fixtures/owner-bearing-swift.json` is the unchanged qualified actual Swift corpus, SHA-256 `307BD860A42BC5A3191FA0F421BFCA32AB0C27CC44DF0843840AE0F57B43FE8D`. Test wrappers add explicitly synthetic native envelopes, snapshot clocks and injected Bridge methods. They do not claim a live owner or rotor.

Focused gate: 49 new reader/control/hook/component/IPC cases, 67 existing Point/connection cases, 16 original rotor/navigation cases, and 409 selected catalog assertions passed. Source and tools are pinned in `target/bearing-controls-preparation`. The runner used one worker, a 2 GiB private Job, 300 seconds, Node heap 1024 MiB, fresh 8/6 GiB physical/commit admission, 2/2 GiB floors and three-second natural descendant drain. These are selected assertions, not the entire upstream UI suite.

## Catalog source preparation

Eighteen bearing labels were added to English and all eight present translation catalogs; the four maintained CSV files receive only those eighteen rows. Existing offline converters ran in a bounded source-generation job: `target/bearing-catalog-generation/attempt-866806d726944455b269e31188022200/receipt.json`, SHA-256 `8982E96641891DFB313FF8F45F59B9C1F2B2D21014617DCDC5BDCD81FE7234CC`. It retired naturally with zero children. This was not a test run.

Whole conversion exposed existing CSV/catalog drift affecting unrelated editor, Spots and frequency entries. Original catalog bytes and full generated outputs remain preserved in that attempt. Only the eighteen converter-produced new rows were applied; removing that additive block restores each original catalog byte for byte. `selective-addition.json` records the comparison. No unrelated drift correction or converter `--check` success is claimed. Actual key-set/placeholder guards passed; all four CSV files, including both Chinese variants, contain eighteen distinct new entries with exact values matching their converter-produced catalog rows.

## Qualification receipts

All paths below are relative to this worktree. Exact manifests retain all 2,651 source inputs and tool hashes. Every passing process had exit zero, natural descendant retirement, final active count zero, no memory/abnormal-exit flags and confirmed cleanup.

| Gate | Retained evidence | Receipt SHA-256 |
| --- | --- | --- |
| 132 behavior + 409 selected catalog assertions; four CSV checks | `target/bearing-controls-preparation/attempt-b53a69ed933e47138ac32a9c1d823139` | `93573F1362F9B8EBA543B02BA1D36F16B5DCD8906BCD4B6361958DAE3D402385` |
| Full UI TypeScript build + production Vite | `target/bearing-types-aligned-preparation/attempt-cac0a7fcc41e4aea93b7d4917a8d519f` | `31FC7381A4F2371CCFF9AF629E0BDCF84839FC6BE6867ADA7D341DD7EE8ADD23` |
| Private original-host/component browser, nine cases | `target/bearing-browser-preparation/attempt-554bcdbfeb634c8483371c5993ec22e2` | `3A189D16C5E739F9870ECCF7F9137E82A1D1F79DEDB4FFA1B30499594072622B` |

The first TypeScript attempt is preserved as failed: `target/bearing-types-preparation/attempt-9dd0b4df247c4b1cae5ca88ed3c8032b`, receipt `D26DAC3BF7A9D831C44325039E5764223FF854CCE2FD130D2D193950206E3C90`. Its only diagnostic was a test mock's widened receipt-status type. Adding the contextual `vi.fn<Bridge['command']>` annotation corrected the type without changing runtime or assertions; Vite was not reached in that attempt. Original test bytes and manifests remain in the failed evidence. The successful type/browser source manifests match exactly: `4A914721AE0CA7F602827724FBC4E5B623723121AC2D7C1117F15103C8FEB6F8`. The focused source predates only that type annotation.

Vite produced 41 source-matched files, manifest `D22C07DA7242BAD8E0E49BC1462F07C20CD7F1ED11F08C79C4A92E24455C0920`. The actual `xlog.html` entry is 943 bytes, SHA-256 `AB003ECA0C8C9F0A6EC7F8D993AFE1108CB1CAFCDCB76A72C5D908052ED49749`, independently compared with the file. Chunk-size warnings remain; no bundle-size or package claim is made.

The browser used the reviewed private headless runtime `45024029FC5C041FF2693E0174A2165559507E46825FBC25D573307D503F9BC3` with no global desktop input. Six owner layouts cover Classic/dark and Roster/light at laptop, floor and UHD/200% sizes; three native RotorStrip cases preserve original defaults. Exact Swift body fixtures were wrapped in synthetic snapshots, native read responses and intercepted receipts. Assertions cover explicit reads, exact portable/base calls and owner SP/LP angles, historical/approximate precision, typed unavailability, later noncausal observation, stale authority, Stop during a pending passive read, RX Stop and retained navigation layout. Peak bearing/receipt/approximation/capability-retirement states require the entire rotor group within the viewport and no horizontal document overflow before hit-test scrolling. External requests, owner-branch native fallback, page errors and console errors were zero. Browser and Job both exited normally; result SHA-256 is `81AE6BE85D92392D79F6E8252194FA9320EEAF5A5596EA18A0655F4D1AA75110`.

This qualifies the frontend boundary with synthetic transport and real original components. It does not qualify live rotor motion, a native packaged GUI, user station configuration, or a complete upstream test suite. Only this evidence document changed after the final passing source manifest.
