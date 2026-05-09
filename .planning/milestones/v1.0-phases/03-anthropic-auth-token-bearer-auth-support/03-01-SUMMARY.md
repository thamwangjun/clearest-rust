---
phase: 03-anthropic-auth-token-bearer-auth-support
plan: "01"
subsystem: testing
tags: [rust, tdd, integration-tests, serial_test, tokio, bearer-auth, anthropic]

# Dependency graph
requires: []
provides:
  - "6 failing integration tests covering all D-09 bearer auth cases (RED TDD phase)"
  - "serial_test = 3 dev-dependency for env-isolated async tests"
  - "crates/core/tests/bearer_auth.rs with #[serial] annotated test functions"
affects:
  - 03-02

# Tech tracking
tech-stack:
  added:
    - serial_test = "3" (async test serialisation, prevents env var race conditions)
  patterns:
    - "TDD RED gate: tests compiled against not-yet-existing API to prove failing contract"
    - "#[serial] on every env-mutating async test + reset_anthropic_env() at top and bottom"
    - "Helper fn anthropic_config_with() for building provider-specific Config"

key-files:
  created:
    - crates/core/tests/bearer_auth.rs
  modified:
    - crates/core/Cargo.toml

key-decisions:
  - "TDD RED state confirmed: compile errors on unwrap_err() (not on Option) and missing use_bearer_auth field — no false passes"
  - "Used #[serial] from serial_test crate (not tokio::test serial) for correct async serialisation"
  - "reset_anthropic_env() called both at top and bottom of each test to guard against panic mid-test leaving env dirty (threat T-03-T-01)"

patterns-established:
  - "bearer-auth RED pattern: write tests against target API shape before implementing; confirms RED via compile errors"
  - "env isolation pattern: remove_var both ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN at start + end"

requirements-completed:
  - D-07
  - D-08
  - D-09

# Metrics
duration: 10min
completed: 2026-05-07
---

# Phase 3 Plan 01: Bearer Auth Integration Tests (RED Phase) Summary

**6 async #[serial] integration tests establishing D-09 bearer auth contracts using serial_test = 3, all failing at compile time (RED gate confirmed via unwrap_err/use_bearer_auth type errors)**

## Performance

- **Duration:** ~10 min
- **Started:** 2026-05-07T07:33:00Z
- **Completed:** 2026-05-07T07:41:54Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments

- Added `serial_test = "3"` to `crates/core/Cargo.toml` [dev-dependencies] (D-08)
- Created `crates/core/tests/bearer_auth.rs` with 6 async `#[serial]` test functions (D-07)
- Covered all 5 D-09 cases: happy path bearer, both-env conflict, use_bearer_auth+env conflict, use_bearer_auth+settings conflict, config.env injection
- Added regression guard for existing ANTHROPIC_API_KEY → x-api-key path
- RED gate confirmed: 8 compile errors due to `unwrap_err()` not on `Option<T>` and `use_bearer_auth` field not yet on `ProviderConfig`

## Task Commits

Each task was committed atomically:

1. **Task 1: Add serial_test dev-dependency and write RED test file** - `fd751cf` (test)

**Plan metadata:** pending (docs commit below)

## Files Created/Modified

- `crates/core/tests/bearer_auth.rs` - 6 async integration tests for D-09 bearer auth cases, all `#[serial]`, 155 lines
- `crates/core/Cargo.toml` - Added `serial_test = "3"` to [dev-dependencies]

## Decisions Made

- Used `serial_test = "3"` (not `tokio::test(flavor = "serial")`) because it works across all async runtimes and matches existing codebase testing patterns
- reset_anthropic_env() called at top AND bottom (not just top) to handle panics mid-test leaving env dirty (D-08, threat T-03-T-01)
- Tests written against the target API shape (Result-returning resolver + use_bearer_auth field) to create the failing contract upfront

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. The RED gate was achieved exactly as expected: compile errors on `unwrap_err()` (not a method on `Option<T>`) and `use_bearer_auth` (field does not exist on `ProviderConfig` yet).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 03-02 (GREEN phase) can now implement the upgraded `resolve_anthropic_auth_async` returning `anyhow::Result<Option<(String, bool)>>` and add `use_bearer_auth: Option<bool>` to `ProviderConfig`
- All 6 test contracts are locked in; any implementation that satisfies them is correct
- serial_test dependency is in place so no Cargo.toml changes needed in 03-02

## Self-Check: PASSED

- crates/core/tests/bearer_auth.rs: FOUND
- crates/core/Cargo.toml: FOUND (serial_test = "3" present)
- Commit fd751cf: FOUND
- RED state: CONFIRMED (8 compile errors, cargo check exits non-zero)

---
*Phase: 03-anthropic-auth-token-bearer-auth-support*
*Completed: 2026-05-07*
