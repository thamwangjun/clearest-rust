# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

---

## Milestone: v1.0 — Initial Bug-Fix + Bearer Auth

**Shipped:** 2026-05-09
**Phases:** 3 | **Plans:** 9

### What Was Built

- Fixed welcome screen silent exit (BUG-01) — Enter now proceeds to TUI session
- Fixed collapsed thinking block content leak — animated dots instead of leaked reasoning text
- Fixed first-run startup routing — no-credentials users land on Welcome page
- Full ANTHROPIC_AUTH_TOKEN Bearer auth support with conflict detection and correct `/status` display
- 6 serial integration tests + startup routing regression guard

### What Worked

- TDD approach for bearer auth was clean: RED phase with 6 failing tests gave a clear target, GREEN phase was focused, compile gate caught wiring errors fast
- `serial_test` for async env-var tests eliminated flakiness risk before it could bite
- Audit-before-close (`/gsd-audit-milestone`) gave high confidence — no surprises at completion
- Code review after Phase 3 execution caught several real bugs (UTF-8 boundary panics, dead params, duplicate constants)
- Scope change in Phase 3 (removing `use_bearer_auth` rather than exposing it) was surfaced in UAT and approved cleanly — UAT as a gate works

### What Was Inefficient

- Phase 3 grew from 3 planned plans to 6 actual plans — the initial plan underestimated the call-site wiring, `auth_status()` fast-path, and D-04 scope change complexity
- Multiple UAT reruns for the bearer auth proxy issue that turned out to be an environment artifact (bearer auth billing header) — earlier environment isolation would have saved cycles
- Phase 2 animated dots spec was dropped mid-UAT (discovered to be wrong requirements) — the original requirements should have been validated earlier against actual behavior

### Patterns Established

- `serial_test` for any async integration tests that mutate env vars
- `config.env` injection at fast-path callers that bypass the main auth loop (mirroring the main-loop pattern)
- Startup routing via `show()` (not `show_provider_setup()`) as the correct welcome page entry point
- Code review pass after phase execution as a standard quality gate

### Key Lessons

1. Plan for 1.5–2× the plans when implementing new auth flows — the call-site wiring and fast-path callers always add plans beyond the core feature
2. UAT environment artifacts (proxy, billing headers) can look like feature bugs; have a quick curl-based isolation test ready before investigating the code
3. Requirements that depend on visual behavior (animated dots cadence) need to be validated against existing animation patterns before writing the plan

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Phases | Plans | Key Change |
|-----------|--------|-------|------------|
| v1.0 | 3 | 9 | First milestone — established TDD + audit + UAT gate pattern |

### Cumulative Quality

| Milestone | Tests added | Zero-dep additions |
|-----------|------------|-------------------|
| v1.0 | ~10 (regression + integration) | `serial_test` (dev-dep only) |

### Top Lessons (Verified Across Milestones)

1. Audit before close gives real confidence — don't skip it
2. UAT as a scope gate: catching the `use_bearer_auth` removal in UAT was cheaper than shipping the wrong API
