---
phase: 03-anthropic-auth-token-bearer-auth-support
plan: "03"
subsystem: auth
tags: [rust, bearer-auth, anyhow, result-propagation, cli-wiring, anthropic]

# Dependency graph
requires:
  - phase: 03-02
    provides: "resolve_anthropic_auth_async returning anyhow::Result<Option<(String, bool)>> with D-02 conflict detection"
provides:
  - "main.rs primary auth call site updated: resolve_anthropic_auth_async().await? (D-03)"
  - "config.env injection loop in main.rs before auth call site (D-05)"
  - "All secondary call sites in main.rs and crates/commands updated to .ok().flatten()"
  - "cargo build --workspace exits 0 (D-06)"
  - "All 6 bearer_auth integration tests pass GREEN"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Result-to-Option flattening: .ok().flatten().unwrap_or(default) for non-critical call sites that must not propagate errors"
    - "config.env injection-before-resolution ordering: process env injection precedes any auth resolver call"

key-files:
  created: []
  modified:
    - crates/cli/src/main.rs
    - crates/commands/src/lib.rs

key-decisions:
  - "Secondary call sites in commands/lib.rs use .ok().flatten() (not ?) because those code paths (provider discovery, health check) must not terminate the process on auth conflicts — they fall back silently to empty credentials"
  - "config.env injection loop placed between api_base setter and --dump-system-prompt fast path, ensuring settings.json ANTHROPIC_AUTH_TOKEN is visible before resolve_anthropic_auth_async runs"
  - "Pre-existing codex_adapter float precision test failure is out-of-scope (unrelated to bearer auth changes)"

patterns-established:
  - "call-site result propagation: primary run() path uses ? to propagate auth errors; secondary/utility paths use .ok().flatten() to degrade gracefully"

requirements-completed:
  - D-05
  - D-06

# Metrics
duration: 8min
completed: 2026-05-07
---

# Phase 3 Plan 03: main.rs Wiring (config.env injection + Result call site) Summary

**Wired main.rs to consume Result return from resolve_anthropic_auth_async via ?, added config.env process-env injection loop (D-05), and fixed three secondary call sites in commands/lib.rs to compile with the new Result type**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-05-07T07:50:00Z
- **Completed:** 2026-05-07T07:58:00Z
- **Tasks:** 1
- **Files modified:** 2

## Accomplishments

- Added config.env injection loop to `crates/cli/src/main.rs` at correct position (after api_base setter, before auth resolver call) — D-05 satisfied
- Added `?` to `resolve_anthropic_auth_async().await` in main.rs primary auth call site — D-03 satisfied
- Fixed secondary main.rs call site (~line 942) using `.ok().flatten()` pattern
- Fixed `crates/commands/src/lib.rs` `provider_for_config` call site using `.ok().flatten()` — Rule 1 auto-fix
- Fixed `crates/commands/src/lib.rs` health-check info call site using `.ok().flatten().unwrap_or()` — Rule 1 auto-fix
- `cargo build --workspace` exits 0 — D-06 satisfied
- All 6 bearer_auth integration tests pass GREEN

## Task Commits

Each task was committed atomically:

1. **Task 1: Update main.rs call site and verify config.env injection (D-03, D-05, D-06)** - `a6896a8` (feat)

**Plan metadata:** pending (docs commit below)

## Files Created/Modified

- `crates/cli/src/main.rs` — Added config.env injection loop (lines 515-522); added `?` to primary auth call site (line 566); fixed secondary call site (~line 942) with `.ok().flatten()`
- `crates/commands/src/lib.rs` — Fixed `provider_for_config` and health-check info call sites to use `.ok().flatten()` for Result-returning resolver

## Decisions Made

- Secondary call sites in `crates/commands/src/lib.rs` do NOT use `?` — these are provider discovery and UI health-check paths that should degrade gracefully (return empty credentials) rather than terminate the process on an auth conflict. This matches the expected behavior of those call paths.
- config.env injection loop placed before the `--dump-system-prompt` fast path, so even early-exit paths see injected env vars.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed secondary auth call sites in crates/commands/src/lib.rs**
- **Found during:** Task 1 (compile gate step)
- **Issue:** `cargo build --workspace` failed with 5 errors in `claurst-commands`: two call sites to `resolve_anthropic_auth_async` still used the old `Option`-based patterns (`.map(|(credential, _)|...)`, `.is_some_and(...)`, `.unwrap_or((String, bool))`) which are incompatible with the new `Result<Option<...>>` return type
- **Fix:** Changed both call sites to use `.ok().flatten()` before chaining the Option-based methods. This converts `Result<Option<T>>` to `Option<T>` — appropriate for paths that should degrade gracefully rather than propagate auth errors
- **Files modified:** `crates/commands/src/lib.rs` (lines 153, 1963)
- **Verification:** `cargo build --workspace` exits 0 after fix
- **Committed in:** `a6896a8` (Task 1 commit)

**2. [Rule 1 - Bug] Fixed secondary auth call site in crates/cli/src/main.rs (~line 942)**
- **Found during:** Task 1 (compile gate — second build attempt)
- **Issue:** A second call site in `main.rs` (OAuth token refresh path) used `resolve_anthropic_auth_async().await.unwrap_or((String::new(), false))` treating `Result` as an `Option`
- **Fix:** Added `.ok().flatten()` before `.unwrap_or(...)` to correctly handle `Result<Option<...>>`
- **Files modified:** `crates/cli/src/main.rs` (line ~942)
- **Verification:** `cargo build --workspace` exits 0 after fix
- **Committed in:** `a6896a8` (Task 1 commit)

---

**Total deviations:** 2 auto-fixed (both Rule 1 - compile-blocking type mismatches)
**Impact on plan:** Both fixes required for compilation. Secondary call sites intentionally use `.ok().flatten()` (not `?`) to preserve graceful degradation in non-critical paths.

## Issues Encountered

- Plan 03-02 SUMMARY noted that `cargo check -p claurst` would fail with "5 errors at the call sites in claurst-commands (expected; deferred to Plan 03)". The actual errors were in both `claurst-commands` and a second `claurst` (cli) call site — 7 errors total across 3 call sites, not 5. All resolved by this plan.
- Pre-existing test failure: `codex_adapter::tests::test_anthropic_to_openai_request_basic` fails due to floating-point precision (`0.699999988079071` != `0.7`) — unrelated to bearer auth changes, out of scope.

## Threat Surface Scan

All security-relevant changes are covered by the plan's threat model (T-03-03-01 through T-03-03-03):
- config.env injection uses `if std::env::var(key).is_err()` — real process env vars always win (T-03-03-01 mitigated)
- Error messages from `?` propagation show only var names/key paths, never credential values (T-03-03-02 accepted)
- No new threat surfaces introduced.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- Phase 3 is complete: ANTHROPIC_AUTH_TOKEN is a first-class credential source, conflicts are detected and error explicitly, `use_bearer_auth` is user-configurable in settings.json
- Manual smoke test: set `ANTHROPIC_AUTH_TOKEN` in `~/.claurst/settings.json` `config.env`, run `claurst`, observe `Authorization: Bearer` header in proxy logs (see VALIDATION.md)

## Self-Check: PASSED

- `crates/cli/src/main.rs` FOUND — config.env injection loop present (line 518), `resolve_anthropic_auth_async().await?` present (line 566)
- `crates/commands/src/lib.rs` FOUND — `.ok().flatten()` at both call sites
- Commit `a6896a8`: confirmed
- `cargo build --workspace`: exits 0
- bearer_auth tests: 6 passed, 0 failed

---
*Phase: 03-anthropic-auth-token-bearer-auth-support*
*Completed: 2026-05-07*
