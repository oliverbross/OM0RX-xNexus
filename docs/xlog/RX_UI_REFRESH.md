# Decoded-fact refresh feedback

A temporary local RX-status failure used to turn every accepted classification
lease into a permanent zero-deadline tombstone. Returning the same still-valid
fact could not restore its display. A temporary owner-snapshot freshness gap
also poisoned facts even though neither their owner nor their route changed.

The original receiver now hides facts during unavailable observations while
preserving their original deadlines internally. A successful read may display
them again only before that same deadline. Capability withdrawal, explicit RX
start/stop and connection replacement still retire them. No stale fact is
renewed and no transmission permission is derived from these display leases.

Six focused receiver lifecycle cases passed, including a short read failure,
original-deadline expiry and a freshness gap. Existing tests for pending Stop,
connection replacement and foreign receiver results remain in this suite.
Evidence: .build/three-bug-fix-check in the xLog workspace. The first edit's
caught variable-replacement error is preserved; the corrected focused run
passed with natural process retirement and zero remaining processes.
