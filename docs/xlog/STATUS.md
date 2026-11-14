# Execution ledger — 14 November 2026

- M0 complete: source downloaded with full upstream history; separate private GitHub repo
  created; fetch-only upstream configured; source pins and management plan recorded.
- M1a: read-only native client implemented; 20 synthetic tests, Clippy and formatting passed. See VERIFICATION.md.
- M1b: proposed to “xlog work”; current startup/reconnection fix retains priority.
  The owner confirmed host-owned station/rotor/cluster snapshots and logbook chase/history projections are the right adaptation surfaces. No supported bridge endpoint exists in the inspected xLog version.
- M2–M8: pending; no connected Nexus UI, hardware command, QSO writeback, decoder
  audio sharing, installer or on-air qualification is claimed.

The copied upstream application still has upstream identity/services. Do not run
or package it as xNexus until M2/M7 isolation gates are complete. The reference
probe only talks to an explicitly provided loopback bridge and cannot open hardware.

Nine GitHub issues and milestones are created; see BACKLOG.md. The user approved
temporarily disabling GitHub Actions on 14 November 2026, and the setting has
been applied. Imported release/deployment workflows remain disabled pending review.
The source bootstrap and tested read-only client are published on origin/main.
Bootstrap commit: 416616a39292cbf79d0cc9061512a5e6ffd2851c.
Portable push-hook permission correction: 1061735b2.
GitHub API verification confirmed the private repository, exact remote commit,
plan/client contents and Actions disabled. No installer was produced.
