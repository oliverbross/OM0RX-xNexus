# 1.24.1-xlog.15 CS/DS columns candidate

Pre-version integrated source: `453e97df4f9b49ed23810b0cec7b3d9a6ddd3f98`
(fast-forward of `claude/xlog-cs-ds-display` onto `cc585febf`).
Upstream provenance remains `c88740a94172b3e78ae587d62bd0a54394d1e85f`.
This commit changes only the five version surfaces and adds this record. The
package step binds the resulting exact clean commit; no self-commit hash is guessed.

Change since `.14`: companion decodes show xLog's call status (CS) and DXCC
status (DS) as two dedicated columns in xLog's own codes and colours, replacing
the single need badge inside the message text. UI-only: no FT transmit, owner
protocol, PTT or receive-path change. Focused UI tests (OwnerNeedBadge,
OperateDecodes xLog tint, xlogStatus) passed with a clean TypeScript check on the
feature branch.

The `claude/code-review-fixes` branch (FT operation retirement and keychain/mode
fixes) is deliberately not included; it touches FT TX coordination and awaits
separate sign-off.

Paired owner: xLog `1.6.62` (`4ce654a1`) or later. The CS/DS values stay Unknown
whenever the owner reports needs evidence as not ready; that is an owner-side
condition, not a display fault.

At this checkpoint `.15` has not been built or installed.
