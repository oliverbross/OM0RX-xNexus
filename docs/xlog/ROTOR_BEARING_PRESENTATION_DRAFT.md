# Compact original rotor presentation — qualification record

Base `bbae97f624adb1cd7f9ed6af54338312b7130b0e`; presentation-only follow-up to the qualified functional bearing slice. Focused tests, full TypeScript/Vite and private browser checks passed. Parent visual review accepted floor dark exact-request inspection and UHD light approximation; no further UI changes were requested. This record is not a release or installation claim.

Only the owner branch of the original RotorStrip changes. Current azimuth, SP/LP, last-request target and later observed readings display one decimal. Retained facts, immutable requests, owner angles and command callbacks remain exact and unchanged. Rounding may display `360.0` for a value just below 360; no different angle is sent.

The compact Last Point button is keyboard focusable and uses the existing Radix Tooltip. Its translated inspection label and portaled content expose the complete request ID, exact rotor ID and unrounded target angle. Pending, uncertain and locally refused outcomes stay distinct. Later snapshot readings remain explicitly noncausal, never a target-reached confirmation. Stop remains separate and globally reachable. Native RotorStrip behavior is unchanged.

The eighteen existing owner labels remain; three statuses are shortened and two labels added in all nine catalogs and four CSV files. No whole converter regeneration or unrelated catalog drift is included.

## Frozen source

- Source inventory: `10AD6BB8330E935F75E2E893DBA770FD5E48B64DB23CCF2BD49D164A22703D57`.
- Focused and TypeScript before/after manifests: `05A7809059DA75CDD4FE5378CFAD812C34F38808BDF702AA0F13308920F7E68D`, identical in both attempts.
- RotorStrip: `9746FDA726B4B739CDB9EC619C79F994E083278EA3D4EE82DE03ECD1DB029A26`.
- Bearing presentation tests: `6C82250523036369D9DB6A4055548E8E7F954A3F48DADC69D57BF38F16640659`.

## Focused validation — passed

Evidence: `target/rotor-presentation-focused-preparation/attempt-4c3f3808c0324707aa1e05af9d3757cb`.

25/25 selected component/native/portal cases passed: eight bearing presentation, three external rotor, three native long-path, nine native satellite-dial, and two existing portal-zoom cases. All 409 selected catalog assertions passed (379 hardcoded-string and 30 placeholder). Four CSV locales each matched all twenty owner rows to their catalog values. No skipped or failed cases. This does not claim all UI/upstream tests or a complete converter check.

- Receipt: `78BA0CEFDAEF36AA8F511587D89EB8A1BFCC18BDCCF668187500F88F8C1CD70B`.
- Presentation report: `30EE8A6EECE9C7B34F9CBA53B66B09E2EF29D7BE228929F477D718D5D527B11A`.
- Catalog report: `3A2C49B624E68B938ABCC4E6B03D6FE55B4CF303E3E925C60A21029636220830`.
- CSV report: `E8CBDF69EE12B38A01321B90C421B85F86DFB30087A9E427F89DCDFC7AD934F6`.
- Guard: 2 GiB, 300 seconds, Node heap 1024 MiB, one worker, BelowNormal; 8/6 GiB admission and 2/2 GiB physical/commit floors, three-second natural drain. PID 3380 exited zero naturally, final active processes zero, cleanup confirmed, no memory/abnormal notifications, peak 865,566,720 bytes. Completed 2026-11-15 20:22:55 UTC.

## Full TypeScript and production frontend — passed

Evidence: `target/rotor-presentation-types-preparation/attempt-e1e72385388c42b08a9f18c5350531c1`.

Full existing UI `tsc -b` passed without diagnostics, followed by successful Vite production build. The existing chunk-size warning remains; it is not a failure. The manifest contains 41 generated assets. Independently checked `xlog.html` is present (943 bytes), and its actual hash equals the manifest entry.

- Receipt: `86BF7DA4B74CF204A54B19E8196314D2EB97CEC42EC07D5298769AF317EE827A`.
- Asset manifest: `B09728A35E92B6708851E815F1CC71682EF19620C179029E572EB198BA652222`.
- `xlog.html`: `68232432D32CA401566B0E81C9EE83570984A572FDFBBEB74044B6367902DF9C`.
- Vite log: `14D955903E83C23063B46604CD57374A0F58086F39FBA9B125EBE357B43CCC52`.
- Guard: 4 GiB, 600 seconds, Node heap 2560 MiB, BelowNormal; same admission/floors and natural-drain policy. PID 15948 exited zero naturally, final active processes zero, cleanup confirmed, no resource notifications, peak 2,641,207,296 bytes. Completed 2026-11-15 20:28:01 UTC.

## Private browser — passed

Evidence: `target/rotor-presentation-browser-preparation/attempt-5829c7db91074496be3e236b403068c0`.

Nine cases passed: laptop/floor/UHD200 with six owner Classic/dark and Roster/light layouts, plus three native positive controls. The actual original host/components/styles were exercised using synthetic transport and qualified Swift body fixtures; this is not a live owner or hardware test. All exact Point-wire assertions remain unchanged. Keyboard focus exposed the complete request UUID, rotor identity and unrounded angle, blur dismissed the tooltip and focus reached the separate Stop control. Peak strip geometry was checked before hit testing. All external requests, forbidden native calls, page errors, console errors and console warnings were zero.

- Receipt: `5DDE741CADB8AADE3DD9FBD9CE4A15DE5C1DBFBEC511D811A77F2EB1C439EB35`.
- Results: `47CC0C26C75BF05E889B588A23F40FA26AD8538F153B1A5C707333136243B23D`.
- Browser exit: `66EE723CEDA7BDC7472E9EC012FDD4BE0DE1C66FF5609C2DB07046760A56AF3B`.
- Same source before/after manifest `05A7809059DA75CDD4FE5378CFAD812C34F38808BDF702AA0F13308920F7E68D` as both earlier gates.
- Guard: 2 GiB, 600 seconds, Node heap 1024 MiB, BelowNormal; same admission/floors and three-second natural-drain policy. PID 18220 exited zero naturally; Edge PID 22916 closed ordinarily with exit zero. Final active processes zero, cleanup confirmed, no resource notifications, peak 1,517,625,344 bytes. Completed 2026-11-15 20:32:17 UTC.
- Representative screenshots in the attempt's `screenshots/`: `3-owner-dark-later-observation.png`, `3-owner-dark-exact-request-inspection.png`, `4-owner-light-exact-request-inspection.png`, `7-owner-light-approximation.png`. Parent independently reviewed floor dark exact-request inspection and UHD light approximation, accepting accessible full identity, compact readability and separate Stop.

All three qualification attempts passed first time without source/assertion changes between gates. Only this documentation changes after their source manifests were verified. The pending draft and all original receipts remain preserved.

No live native app, global desktop input, owner transport, hardware, physical rotation, package or install is part of these checks. The old archive worktree remains untouched; no archive implementation is included.
