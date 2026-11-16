# Decoded station classifications — 16 November 2026

The original host checked `needs.query`, but the actual xLog producer and SDK
advertise `logbook.classify`. A read-only check of the running owner showed that
capability with ready classification metadata. The host now uses the real
capability for both RX admission and visible needs evidence.

The original decode feed, Classic station cards and Roster retain the owner's
call and DXCC classifications separately: New Call, New Band, New Mode,
New Band+Mode, Worked, Confirmed and DXCC/ATNO facts. Missing or expired facts
remain Unknown. Known call history survives an unresolved entity classification.
Worked-before is mapped from a known call classification; current-band B4 is
mapped only where the owner status establishes it. Native B4 badges are not
used to invent an exact-mode explanation. The tooltip names the owner's DIGI
classification family rather than claiming FT8 and FT4 are distinct awards modes.

Validation:

- Failing-first: the capability mismatch and seven dropped-fact cases reproduced.
- Fifteen focused component/host/mapper cases passed.
- Strict app and node TypeScript passed with no diagnostics; the native Job
  retired naturally with zero remaining processes and no resource flags.
- Actual original components passed three private headless browser viewports,
  including the 175% floor: new-call/ATNO, new-band, worked and original evidence
  expiry. Results SHA-256:
  `8EF66B2D2DD86D8248FC93478A10C640F3CD21C73E0426DDFDEF17FE8B773E2F`.
  Source pins remained unchanged. No desktop input, production control or log
  mutation was used.

The first browser attempt is preserved: its fixture published a fact during
RX-start intent, correctly tombstoning that evidence. The corrected fixture
publishes after start settles, matching the real worker lifecycle; no product
TTL or invalidation rule was changed to pass. The unrelated upstream CI matrix
and native installation are not claimed by these focused source/UI gates.
TX implementation and its installed acceptance remain separate ongoing work.
