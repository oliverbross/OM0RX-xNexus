# Combined original Nexus UI source qualification

The integration combines manual-create source `58504ce79ffa85456e1a4984051e94f218bcd801` with canonical rotor/operating source `7e0d77d3da03e2d5f60caaabb6f7af81e51f5fc8`. It preserves actual Nexus components, Classic/Roster layouts and owner-only services. This is frontend source qualification, not an installed/native or actual SQLite acceptance claim.

## Merge review

Five additive conflicts were resolved: four translation CSVs and the migrated-string coverage list. Both manual and rotor additions remain. OriginalHost auto-merged and was reviewed for both paths. Twenty nonshared manual runtime/test/style files remain byte-exact to the qualified manual commit; every nonshared M4 runtime/test change matches canonical main. Fifteen shared files contain the expected catalog/CSV/host union. All four CSVs retain the exact 13 manual and 20 rotor keys/values without whole-file regeneration.

## Combined gates

Evidence is retained under `target/` in the `original-manual-create` worktree. Each run checked all recorded source hashes before and after; no product source changed during validation. All children retired naturally with zero final active processes and no forced cleanup or resource/abnormal-exit notification.

| Gate | Actual result | Receipt SHA-256 |
| --- | --- | --- |
| `combined-manual-rotor-types-vite-14g` | Strict `tsc -b` passed; production Vite emitted 41 files to a unique target directory. 5,379 pins unchanged. | `3744E518C93D319991AEC201C39988910AFFD4390D81AB1D825FE5404A34755A` |
| `combined-manual-rotor-catalog` | 440/440: hardcoded strings 382, placeholders 30, invariants 8, i18n 20; no skips. 5,377 pins unchanged. | `F3850BCBF38CE9F76D88C416B7614BAA78DDB9079A51968411599646EF8F3F79` |
| `combined-manual-rotor-focused` | 55/55 across 11 nonempty manual, host, connection and rotor suites; no skips. 5,377 pins unchanged. | `06EDFC2359BF220B9213DCD346B10BE4427366EE66EDC97ECEE9CC1845013DAA` |
| `combined-manual-browser` | Three viewports, six Classic/Roster layouts passed using actual components and synthetic owner services. Root and private Edge exited 0. 5,381 pins unchanged. | `77AF569D440F118025D84734E668E9DBD64D360CC78BB7986FCA1B4FEB6CCDA9` |

The manual browser retained guidance-once, exact MHz/UTC/multiline projection, independent correction storage, immutable queued payload, freeze-storage failure/discard, uncertain outcome and same-ID recovery checks. It observed zero native calls and no external traffic, page or console errors. No OS desktop input or native app launch occurred. Floor draft and uncertain-state screenshots were visually reviewed; original form geometry and sticky actions remained usable.

The TS run used the reviewed 4 GiB/600-second/2560 MiB heap guard, with an additional immediate 14 GiB physical/14 GiB commit admission check. Catalog/focused runs used 2 GiB/300 seconds/1024 MiB heap and one worker; browser used 2 GiB/600 seconds/1024 MiB heap. Existing resource floors and timing/assertion budgets were unchanged.

The first browser outer invocation omitted its required `-Released` flag and stopped at line 3, before guard/child/browser creation. The sole released browser attempt passed. No assertion failure was retried in this integration.

## Reused evidence and remaining boundaries

The separate combined rotor nine-case browser proposal was deliberately not executed, prioritizing the requested original-interface test installer. Its nonshared M4 runtime remains exact to the previously qualified nine-case source at `7e0d77d3da03e2d5f60caaabb6f7af81e51f5fc8`; receipt `5DDE741CADB8AADE3DD9FBD9CE4A15DE5C1DBFBEC511D811A77F2EB1C439EB35` is retained in the rotor-bearing-presentation worktree. The combined 55 tests, full TypeScript/Vite and manual host browser qualify the new shared integration; they do not claim a new rotor nine-case run.

Prior manual qualification and preserved fixture failures remain documented in `ORIGINAL_MANUAL_CREATE_SOURCE.md`; this combined TypeScript/Vite pass covers the final guidance presentation change. No version was changed or installer created here. Candidate source/version/provenance, native packaging and installed acceptance remain separate gates; no full-CI, production write, physical control or RF acceptance is claimed.
