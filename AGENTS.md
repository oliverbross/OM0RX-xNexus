# OM0RX-xNexus fork workflow

This is OM0RX's independent derivative of kd9taw/Nexus: upstream Nexus with its
own identity (name, bundle identifier, data folders, keychain service, version
suffix `-xlog.N`), the CS/DS columns in the Call Roster and Band Activity, and
the native-Windows build tooling under `scripts/xnexus/`. It runs as full Nexus
beside OM0RX-xLog (see docs/xlog/README.md); the earlier xLog companion mode was
removed in 1.25.0-xlog.17.

Never push to upstream. Use project author OM0RX
<oliverbross@users.noreply.github.com>, not the upstream maintainer's identity.
Upstream-specific publishing/identity instructions below apply to the upstream
project, not to impersonating its maintainer here. Use absolute git -C paths,
explicit staging and one logical change per commit. Pushes need the owner's
approval for each push; no force push and no upstream publication.
The fork uses .xnexus-hooks/pre-push for its target/source gates; preserve the
upstream .githooks source unchanged for future merges. Keep imported deployment
workflows disabled (guarded to kd9taw/Nexus). Tests proportional to fork changes
are required; report unrun upstream gates explicitly.

---

# Nexus agent entry point

Read [CLAUDE.md](CLAUDE.md) before making changes. It is the shared contributor
and AI-agent guide for this repository, including Codex. Follow its working
protocol, approval gates, transmit-safety invariants, and verification requirements.

Read [ARCHITECTURE.md](ARCHITECTURE.md) to locate the relevant subsystem, then
read the module's own contract and the existing implementation before editing.
Use [.github/workflows/ci.yml](.github/workflows/ci.yml) for the current verification
commands and feature combinations.

For UI and layout work, also read
[the UI layout skill](.claude/skills/ui-layout/SKILL.md). For release documentation,
read [the release documentation skill](.claude/skills/release-docs/SKILL.md).

Keep the operating rules in their existing source. Do not duplicate them here or
hand-edit the generated rules inside `CLAUDE.md`. This file provides instruction
discovery; it does not change the project's rules or authorize any action.
