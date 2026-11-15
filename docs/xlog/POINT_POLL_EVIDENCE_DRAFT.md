# Point dispatch and later poll evidence — source draft

Base: `26d2d68414c2c56a3a038c2919484a1cefe3ab86`. Focused assertions,
private host browser checks and integrated TypeScript/Vite passed with the
preserved runner/typecheck failures described below.
It adds no Point button, bearing resolver, protocol endpoint or hardware action.

`useOwnerConnection` publishes `ControlPoll` only from an actual successful
`api.poll()` call. The envelope retains the request's original monotonic start,
the existing snapshot deadline, opaque connection token, UI session, owner and
workspace. Initial status and connect replies never establish this provenance.
Ordinary command admission and completion fence in-flight polls. Stop intent
also clears and fences this evidence immediately without waiting for dispatch;
polls started before or during Stop cannot restore it when Stop finishes.

The optional, currently unwired `observedPoint(command, capturedScope, requestId)`
uses the same dispatch admission as existing `command(): Promise<void>`. Its
caller must supply the scope captured with the proposed target: resolving the
scope from current mutable state could retarget an old callback after reconnect.
A missing/expired poll, mismatched scope/target, absent capability or ordinary
pending action is a local refusal and causes no Bridge call. Once a Bridge call
is attempted, a lost reply remains uncertain; it is never relabeled a refusal
and no retry is sent.

`pointEvidence.ts` stores the immutable request and historical settlement
separately from the existing pure rotor observer. Stop, source changes and target
replacement retire observation authority, but a correctly identified late receipt
can still settle the original request. The latest operation is exposed by the
hook; a superseded operation's caller retains its own returned result. This is
not a durable multi-operation journal or automatic receipt reconciliation.

The contained observation clock catches thrown reads and latches invalid or
backward values. A pre-admission fault locally refuses the attempt before the
Bridge call. A fault after submission retires observations only: an exact receipt
is stored before observation metadata is processed, cannot become uncertainty
because of that metadata, and does not prevent the independent Stop path.

Only a fresh owner poll that started after command completion, with a newer
revision/generated timestamp than the baseline, may supply later rotor state.
An exact azimuth match is labeled as a later snapshot match, never proof that the
command caused movement. Receipt status remains unchanged, including rejected
and uncertain statuses. Unknown azimuth/motion remain unknown; there is no
invented angular tolerance, field-sample timestamp or selection generation.
Consumers must inspect current scope/deadline at render time; a serialized copy
does not renew authority.

## Focused validation and remaining gates

`target/point-focused-preparation` pins the source tree, Node, PowerShell,
Vitest/Vite/TypeScript/esbuild, lockfile and reviewed D378 Job Object guard.
One worker, 2 GiB aggregate Job cap, Node 1024 MiB heap, 300 seconds, fresh
8 GiB physical/6 GiB commit admission and 2 GiB runtime floors are retained.
Root exit and all descendants must drain naturally within three seconds; forced
cleanup is reported separately and cannot turn a failed lifecycle into a pass.

The focused batch passed 64 cases: 31 existing pure observation tests,
nine existing connection tests, 13 historical-settlement cases and 11 new
actual-hook cases. It covers status/connect exclusion, actual request-start
provenance, delayed pre-command polls, Stop during Point, pre-Stop poll retirement,
source retirement with late receipts, exact caller scope after reconnect,
capability refusal, clock failures before/after dispatch with independent Stop,
and uncertain outcomes without retry. Both actual catalog guards passed all 407
assertions. The runner nevertheless exited 1 because its expected catalog count
was copied as 406 from the earlier rotor source. This base already adds the
`OwnerContactCardBody.tsx` MIGRATED entry at hardcoded-strings.test.ts:177.
The original runner receipt remains **failed**; no captured child exception text
exists. Independent read-only evaluation verifies every artifact predicate except
407-versus-406, all suite paths/no skips, unchanged source/tool pins and natural
root/descendant retirement. It does not relabel the aggregate runner green.

Evidence is retained in `target/point-focused-preparation`:

- `attempt-c030376f76ba420fb01dc693175b916d/receipt.json`:
  `B9F1B934F4072C507A927A09D1EE5816A83D51AE79847A953E13021F93D78F6C`.
- Focused JSON: `8DC930A70F82B291F780CF73A617DE0CE89231F3DB3A9C1EB9029A59C04D1393`.
- Catalog JSON: `BDC397FE5F56CB43484C5A9EBE3356CF87FDA58D8CDF1B33164228604FF49263`.
- Identical before/after source manifests:
  `D263F8E68D77E96225C0E710FDB3C7F7AAD1FF3400714C9B41F7AD6138C93BFD`.
- `qualification-5ab4f683deb948bf93f44c618c0335f2.json`:
  `27D2DD4DA718FFED52F420E2A7F5B493984BA74148F8611F5233A88DC0D4DD72`.

The actual original-host browser fixture also passed nine cases (six owner cases
at three sizes/two themes, three native positive controls) with this modified
hook. It checks global Stop through telemetry expiry, ordinary acknowledgement,
route replacement, reconnect, capability retirement and Spots navigation. There
is no Point UI in this slice, so this is not visual Point-operation acceptance.
The original floor screenshot was visually inspected: unknown azimuth and the
global Stop remain visible above the original Spots table.

Browser evidence is retained in
`target/point-browser-preparation/attempt-f3fd3e9f4446465d898b7234b6cc60b7`:

- Passed receipt: `8488A5A2A1138F984CD1276CC0E83D80C60710D0A7071DBFD298A2CC188C9856`.
- Nine-case result: `B2224EB8CF46CBE5B766C0B565B64787FDD05CFFED435C38A6540F230F819BD6`.
- The source manifests match the focused run's `D263F8E6…93BFD` exactly.
- Guard root PID 13596 and private browser PID 3452 exited normally, final active
  processes zero, cleanup confirmed, no limit/abnormal flags; peak Job memory
  1,464,180,736 bytes. No external requests, page/console errors or warnings.

Full `tsc -b` and Vite passed in
`target/point-types-aligned-preparation/attempt-c865ff7b08eb49eeb0bb6cece364a734`:

- Receipt: `85FC96C50585EED3FA195AAEF5EF71954E654E82A84FC7592FE0B78BCACE32EB`.
- Identical final source manifests:
  `5D24C6DCF5B1F0E5D0F323B4EAE94959AE7199C9963B140FBA38EDA1A005BB18`.
- The 41-asset production manifest:
  `A897EC99256C5DC9697BFB083A7F721720F86E06BE5F11B5F59D490B46BE3B9D`.
- PID 16744 exited zero naturally; final active processes zero, cleanup confirmed,
  no limit/abnormal flags, peak Job memory 2,638,450,688 bytes. The original
  4 GiB/600-second profile, Node 2560 MiB heap and 8/6 GiB admission remained.

The earlier full typecheck in `point-types-preparation/attempt-405134e7042c482098b32dde28e981ac`
failed with TS2345 forwarding a string to Bridge's UUID-shaped parameter; receipt
`C32C94A2202CDF1E2D53E5E45108418F38E779DDBD6BDD89E3408B020C1CB3A6`
is preserved. The sole correction types the internal parameter as
`Parameters<Bridge['command']>[1]`, without a cast or API/runtime validation change.
Pinned TypeScript 5.9.3 with actual UI compiler options emits identical 13,136-byte
JavaScript before/after (`840B79DE0D2E6647ED1D48CEBA67B4565A21529759A96E1EC5545B573F5110B4`).
The source copy and equivalence receipt
`50452F9585F445734A9BAF54058D60AA559474C8E99A90FA001822CF3377A282`
are in `target/point-request-type-equivalence`. This connects the earlier
64/407/browser-nine runtime evidence to the final type-aligned source; those
tests were not gratuitously repeated.

A proposed concurrent TypeScript runner also stopped **before child creation**
in `point-types-concurrent-preparation/attempt-af50ac4a15ca4ddc882f9137141a2b80`:
default JSON timestamp coercion made an unchanged launcher identity compare false.
Its failed receipt and original preparation are preserved. The separate parser
correction preparation never launched TypeScript. After the owner job retired,
the successful run used the original serial profile, not that concurrency helper.

All Bridge responses are synthetic; none prove live rotor or owner telemetry
behavior. Upstream full CI, native packaging and RF qualification remain separate.
No global desktop input, real owner controls or hardware were used.
