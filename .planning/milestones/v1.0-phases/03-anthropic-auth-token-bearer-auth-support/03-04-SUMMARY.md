---
phase: 03-anthropic-auth-token-bearer-auth-support
plan: "04"
subsystem: auth
tags: [rust, bearer-auth, anthropic, status-command, env-vars]

# Dependency graph
requires:
  - phase: 03-01
    provides: resolve_anthropic_auth_async in claurst_core::config::Config
  - phase: 03-02
    provides: Bearer token env var conflict detection (D-02 conditions)
  - phase: 03-03
    provides: Test infrastructure and validation patterns for bearer auth
provides:
  - "/status slash command correctly labels bearer token as 'Authenticated (Bearer token)'"
  - "auth_status() CLI function detects ANTHROPIC_AUTH_TOKEN as a valid logged-in credential"
  - "detect_api_key_env_source() helper centralises Anthropic env credential detection with bearer fallback"
affects: [phase-04-if-any, uat-verification]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Use resolve_anthropic_auth_async for status display — replaces ad-hoc resolve_api_key() + OAuthTokens::load() blocks"
    - "Extract env var detection into testable helper to enable unit tests of private async functions"
    - "Serialise env-var-sensitive tests with OnceLock<Mutex<()>> to prevent parallel test contamination"

key-files:
  created: []
  modified:
    - crates/commands/src/lib.rs
    - crates/cli/src/main.rs

key-decisions:
  - "Replace OAuthTokens::load() + resolve_api_key() in StatusCommand::execute with resolve_anthropic_auth_async — the resolver already handles all four credential paths (bearer pin, api_key, ANTHROPIC_AUTH_TOKEN env, OAuth)"
  - "Extract env var detection into detect_api_key_env_source() helper in main.rs to keep auth_status() concise and enable unit testing"
  - "OAuth tokens now map to 'Authenticated (Bearer token)' or 'Authenticated (API key)' labels in /status — subscription_type label removed (intentional; plan specifies this mapping)"

patterns-established:
  - "Pattern: resolve_anthropic_auth_async is the single source of truth for Anthropic auth status display"
  - "Pattern: env-var-sensitive tests must use a Mutex guard to serialise execution"

requirements-completed: [BEARER-STATUS-DISPLAY]

# Metrics
duration: 5min
completed: "2026-05-08"
---

# Phase 03 Plan 04: Bearer Auth Status Display Summary

**StatusCommand::execute and auth_status() now correctly show 'Authenticated (Bearer token)' when ANTHROPIC_AUTH_TOKEN is set, closing the UAT gap where authenticated bearer-token users saw 'Not authenticated'**

## Performance

- **Duration:** 5 min
- **Started:** 2026-05-08T09:04:06Z
- **Completed:** 2026-05-08T09:09:00Z
- **Tasks:** 3 of 3 (Task 3 human-verify checkpoint — APPROVED by user)
- **Files modified:** 2

## Accomplishments

- Replaced the ad-hoc `OAuthTokens::load() + resolve_api_key()` auth block in `StatusCommand::execute` with a single call to `ctx.config.resolve_anthropic_auth_async()`, which maps all four credential paths to correct labels
- Added `detect_api_key_env_source()` helper in `crates/cli/src/main.rs` that includes a bearer-token fallback for the Anthropic provider, wired into `auth_status()` to replace the inline detection loop
- Added 7 unit tests (3 for StatusCommand, 4 for detect_api_key_env_source) with environment-serialisation mutex to prevent parallel test contamination

## Task Commits

Each task was committed atomically:

1. **Task 1: RED — failing StatusCommand auth label tests** - `e8e0741` (test)
2. **Task 1: GREEN — replace auth detection in StatusCommand::execute** - `30b0d55` (feat)
3. **Task 2: feat — add ANTHROPIC_AUTH_TOKEN detection in auth_status()** - `c02c0e3` (feat)

_Note: Task 2 combined RED (tests added) and GREEN (implementation) in one commit because the helper function and its tests were developed together. Tests confirm pre-fix behaviour via test name comments._

## Files Created/Modified

- `crates/commands/src/lib.rs` — StatusCommand::execute auth block replaced; 3 auth-label unit tests added
- `crates/cli/src/main.rs` — detect_api_key_env_source() helper added; auth_status() wired to use it; 4 unit tests added

## Decisions Made

- Used `resolve_anthropic_auth_async` as the single resolver in `/status` — it already handles OAuth at Priority 4, so the explicit `OAuthTokens::load()` check is no longer needed and was removed
- Extracted env var detection into a named helper (`detect_api_key_env_source`) rather than using an inline shadow binding (as the plan suggested) — this makes the logic unit-testable without running the full async `auth_status()` function
- OAuth tokens now display as "Authenticated (Bearer token)" or "Authenticated (API key)" in `/status` rather than the subscription_type — this is the correct mapping as specified in the plan

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Extracted env detection into testable helper instead of inline shadow binding**
- **Found during:** Task 2
- **Issue:** The plan's prescribed inline shadow binding (`let env_api_key_source = if env_api_key_source.is_none() && ...`) inside `auth_status()` would be untestable since `auth_status()` is a private async function with no test module
- **Fix:** Extracted the detection logic into `detect_api_key_env_source(active_provider: &str) -> Option<String>` helper and added `#[cfg(test)] mod tests` to `main.rs`. The `auth_status()` call site is now a single line: `let env_api_key_source = detect_api_key_env_source(active_provider);`
- **Files modified:** crates/cli/src/main.rs
- **Verification:** 4 unit tests pass; cargo check -p claurst exits 0
- **Committed in:** c02c0e3

---

**Total deviations:** 1 auto-fixed (Rule 2 — testability improvement)
**Impact on plan:** No scope change. Implementation is strictly better than the plan's prescribed approach — same behaviour, testable design. Token value is never printed in either path (threat model T-03-04-01 and T-03-04-02 satisfied).

## Issues Encountered

- Parallel test execution caused env-var contamination in the three StatusCommand tests (ANTHROPIC_AUTH_TOKEN leaking between tests). Resolved by adding a `OnceLock<Mutex<()>>` guard and using `--test-threads=1` for those tests.

## Threat Surface Scan

No new threat surface introduced. Both display paths use the env var name as the source label — the token value is never rendered. Satisfies T-03-04-01 and T-03-04-02.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

All four UAT scenarios verified and approved by user:

1. `ANTHROPIC_AUTH_TOKEN=x ./target/debug/claurst auth status` → "Logged in." / "API key: ANTHROPIC_AUTH_TOKEN" — PASSED
2. REPL `/status` with `ANTHROPIC_AUTH_TOKEN=x` → `Auth: Authenticated (Bearer token)` — PASSED
3. `ANTHROPIC_API_KEY=sk-ant-test ./target/debug/claurst auth status` → unchanged behaviour — PASSED
4. No env vars → "Not logged in for Anthropic." — PASSED

Bearer auth status display is complete. Phase 03 is fully done.

---
*Phase: 03-anthropic-auth-token-bearer-auth-support*
*Completed: 2026-05-08*
