# Milestones: claurst

## v1.0 — Initial Bug-Fix + Bearer Auth

**Shipped:** 2026-05-09
**Phases:** 3 | **Plans:** 9 | **Timeline:** 2026-05-05 → 2026-05-09 (4 days)
**Files changed:** 24 | **Lines:** +2,133 / -476 | **Commits:** 122

### What Shipped

1. Fixed welcome screen silent exit (BUG-01) — Enter now advances to TUI session
2. Fixed collapsed thinking block content leak — animated dots instead of leaked reasoning text
3. Fixed first-run startup routing — no-credentials users see Welcome page, not ProviderSetup
4. Full ANTHROPIC_AUTH_TOKEN Bearer auth with conflict detection and correct `/status` display
5. TDD integration test suite: 6 serial bearer_auth tests + startup routing regression guard
6. Code review fixes: UTF-8 boundary panics, dead code, connection pool reuse, brand color rename

### Requirements Satisfied

- BUG-01: Welcome screen silent exit — ✅ satisfied

### Tech Debt

- `RenderContext::default()` in test paths always shows one dot (non-production path)
- `auth_status()` conflict check mirrors resolver rather than delegating (intentional minimal scope)

### Archive

- `.planning/milestones/v1.0-ROADMAP.md` — full phase details
- `.planning/milestones/v1.0-REQUIREMENTS.md` — requirements with outcomes
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md` — audit report (passed)
