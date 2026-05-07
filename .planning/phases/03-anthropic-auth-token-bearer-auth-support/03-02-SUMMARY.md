---
phase: 03-anthropic-auth-token-bearer-auth-support
plan: "02"
subsystem: auth
tags: [rust, tdd, bearer-auth, anyhow, conflict-detection, resolver, anthropic]

# Dependency graph
requires:
  - phase: 03-01
    provides: "6 failing integration tests covering all D-09 bearer auth cases (RED TDD phase)"
provides:
  - "ProviderConfig.use_bearer_auth: Option<bool> field with serde default + skip_serializing_if"
  - "resolve_anthropic_auth_async returning anyhow::Result<Option<(String, bool)>> with D-02 conflict detection"
  - "resolve_auth_async wrapper upgraded to anyhow::Result<Option<(String, bool)>>"
  - "All 6 bearer_auth.rs integration tests pass GREEN"
affects:
  - 03-03

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Conflict-first resolver: detect mutually-exclusive credential sources before any priority resolution"
    - "Bearer pin priority: use_bearer_auth=true returns Ok(None) rather than falling through to api-key path"
    - "Option-returning async to Result-returning async: OAuthTokens::load().await? replaced with explicit match"

key-files:
  created: []
  modified:
    - crates/core/src/lib.rs

key-decisions:
  - "Conflict detection fires before any priority resolution (D-02) — prevents silent credential escalation attacks"
  - "Bearer pin path returns Ok(None) when ANTHROPIC_AUTH_TOKEN not set (does NOT fall through to api_key) — guards Pitfall 3 from RESEARCH.md"
  - "OAuthTokens::load() ? operator removal: old code used ? on Option<T> (only valid in Option context); replaced with explicit match to satisfy Result return type"

patterns-established:
  - "conflict-first resolver: check all mutually exclusive pairs upfront with anyhow::bail! before entering priority chain"
  - "bearer pin isolation: when use_bearer_auth=true, resolver returns immediately (Ok(None) if token absent) to prevent unintended api-key fallback"

requirements-completed:
  - D-01
  - D-02
  - D-03
  - D-04
  - D-06

# Metrics
duration: 3min
completed: 2026-05-07
---

# Phase 3 Plan 02: GREEN Phase — Resolver Upgrade Summary

**Upgraded ProviderConfig with use_bearer_auth field and rewrote both resolver functions to return anyhow::Result with D-02 conflict-first detection, turning all 6 RED bearer_auth tests GREEN**

## Performance

- **Duration:** ~3 min
- **Started:** 2026-05-07T07:44:40Z
- **Completed:** 2026-05-07T07:47:39Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- Added `pub use_bearer_auth: Option<bool>` to `ProviderConfig` with `#[serde(default, skip_serializing_if = "Option::is_none")]` (D-04)
- Rewrote `resolve_anthropic_auth_async` to return `anyhow::Result<Option<(String, bool)>>` with 3 D-02 conflict conditions using `anyhow::bail!`
- Upgraded `resolve_auth_async` wrapper to propagate `anyhow::Result` (non-anthropic branch now returns `Ok(...)`)
- All 6 bearer_auth.rs integration tests pass GREEN (6 passed, 0 failed)
- `cargo check -p claurst-core` exits 0

## Task Commits

Each task was committed atomically:

1. **Task 1: Add use_bearer_auth field to ProviderConfig (D-04)** - `4c65600` (feat)
2. **Task 2: Rewrite resolver to return Result with conflict detection (D-01/D-02/D-03)** - `dd32292` (feat)

**Plan metadata:** pending (docs commit below)

## Files Created/Modified

- `crates/core/src/lib.rs` — Added `use_bearer_auth: Option<bool>` to ProviderConfig struct + Default impl; rewrote both resolver functions with Result return type and D-02 conflict detection

## Decisions Made

- `use_bearer_auth=true` with no ANTHROPIC_AUTH_TOKEN returns `Ok(None)` (Pitfall 3 guard) — does NOT fall through to api_key path; this is intentional isolation of bearer vs x-api-key modes
- Three D-02 conflict conditions ordered: (1) both env vars set, (2) bearer pin + env api key, (3) bearer pin + settings api_key — evaluated upfront before any credential priority logic
- The old `let tokens = crate::oauth::OAuthTokens::load().await?` used `?` on `Option<T>` which only works in Option-returning functions; replaced with explicit `match ... None => return Ok(None)` to correctly propagate through Result-returning function

## Deviations from Plan

None — plan executed exactly as written. Both functions were replaced with the exact implementation specified in the plan's `<action>` block.

## Issues Encountered

- Initial `cargo test` attempt was run from the wrong working directory (`/Users/thamw/development/local/clearest-rust/`) instead of the worktree root. This caused cargo to use the main repo's unmodified `crates/core/src/lib.rs`, producing the RED compile errors. Re-running from the worktree directory immediately passed all 6 tests.

## Threat Surface Scan

All security-relevant changes are covered by the plan's threat model (T-03-02-01 through T-03-02-04):
- Conflict detection (T-03-02-01): implemented via D-02 three-condition check
- Empty-string bypass guard (T-03-02-04): all env var reads use `.filter(|v| !v.is_empty())`
- No new threat surfaces introduced beyond those documented in the plan.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Plan 03-03 can now update `crates/cli/src/main.rs` call sites that use `resolve_auth_async()` and `resolve_anthropic_auth_async()` — these currently return `Result` but main.rs still awaits `Option`
- `cargo check -p claurst` fails with 5 errors at the call sites in `claurst-commands` (expected; deferred to Plan 03)
- `cargo check -p claurst-core` exits 0 — core library is complete and correct

## Self-Check

- crates/core/src/lib.rs: FOUND — `use_bearer_auth` field present, both resolver signatures updated
- Commit 4c65600: confirmed (feat(03-02): add use_bearer_auth field)
- Commit dd32292: confirmed (feat(03-02): rewrite resolver to return Result)
- 6 tests GREEN: confirmed (`test result: ok. 6 passed; 0 failed`)
- `cargo check -p claurst-core`: exits 0

## Self-Check: PASSED

---
*Phase: 03-anthropic-auth-token-bearer-auth-support*
*Completed: 2026-05-07*
