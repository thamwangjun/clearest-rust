---
phase: 03-anthropic-auth-token-bearer-auth-support
plan: 06
subsystem: auth
tags: [rust, auth, bearer-token, conflict-detection, serde, settings]

# Dependency graph
requires:
  - phase: 03-05
    provides: config.env injection in auth_status()
provides:
  - ProviderConfig without use_bearer_auth, with deny_unknown_fields
  - Simplified resolver: 3 conflict checks, 2 credential priorities
  - Conflict detection in auth_status() for both-env-vars-set scenario
  - 6 passing bearer_auth integration tests (4 obsolete removed, 1 new added)
affects: [UAT-scenario-2, UAT-scenario-3]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "serde deny_unknown_fields on config structs to fail loudly on unknown settings keys"
    - "Mirror resolver conflict checks in CLI display code (auth_status) to surface errors at the earliest point"

key-files:
  created: []
  modified:
    - crates/core/src/lib.rs
    - crates/core/tests/bearer_auth.rs
    - crates/cli/src/main.rs

key-decisions:
  - "Removed use_bearer_auth from ProviderConfig entirely — env var names are sufficient to determine auth mode unambiguously"
  - "Added #[serde(deny_unknown_fields)] to ProviderConfig so stale/unknown settings fields fail loudly instead of being silently ignored"
  - "Added conflict check directly in auth_status() rather than refactoring to call resolve_anthropic_auth_async, to keep the fix minimal and avoid restructuring auth_status return type"
  - "D-02 condition 5 added: provider api_key in settings + ANTHROPIC_AUTH_TOKEN env → hard error"

patterns-established:
  - "Pattern: CLI display commands that inspect auth state must mirror the resolver's conflict checks, not just detect presence of credentials"

requirements-completed: [D-01, D-02, D-03, D-04, D-05, D-06, D-07, D-08, D-09]

# Metrics
duration: 20min
completed: 2026-05-09
---

# Phase 03 Plan 06: Remove use_bearer_auth, Simplify Resolver, Fix UAT Scenarios 2 and 3

**Removed `use_bearer_auth` from ProviderConfig with `#[serde(deny_unknown_fields)]`; simplified resolver to 3 conflict checks and 2 credential priorities; added conflict detection in `auth_status()` to fix both UAT failures**

## Performance

- **Duration:** ~20 min
- **Completed:** 2026-05-09
- **Tasks:** 1 of 2 complete (Task 2 is a human-verify checkpoint; verification results provided by user)
- **Files modified:** 3

## Accomplishments

- `use_bearer_auth` field removed from `ProviderConfig`; `#[serde(deny_unknown_fields)]` added so settings.json with `use_bearer_auth` causes a hard serde parse error (fixes UAT Scenario 3)
- Resolver `resolve_anthropic_auth_async()` simplified: removed `use_bearer_pinned` binding and D-02 conditions 2, 3, 3b; added D-02 condition 5 (provider api_key + ANTHROPIC_AUTH_TOKEN → conflict error); Priority 1 now routes `ANTHROPIC_AUTH_TOKEN` directly
- `auth_status()` in main.rs gets early conflict check: when both `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` are set for Anthropic, prints error to stderr and exits 1 (fixes UAT Scenario 2)
- 4 obsolete tests using `use_bearer_auth: Some(true)` removed from `bearer_auth.rs`
- New test `provider_api_key_with_auth_token_errors` added for D-02 condition 5
- All 6 bearer_auth integration tests pass; `cargo check --workspace` exits 0

## Task Commits

1. **Fix: remove use_bearer_auth, add conflict detection** — `3a9f8d0` (fix)

## Files Created/Modified

- `crates/core/src/lib.rs` — `ProviderConfig`: removed `use_bearer_auth` field and its serde attrs, added `#[serde(deny_unknown_fields)]`. `resolve_anthropic_auth_async()`: removed `use_bearer_pinned` and conditions 2/3/3b, added condition 5, simplified Priority 1.
- `crates/core/tests/bearer_auth.rs` — Deleted 4 obsolete tests; added `provider_api_key_with_auth_token_errors`.
- `crates/cli/src/main.rs` — Added early conflict detection in `auth_status()` before `detect_api_key_env_source()`.

## Decisions Made

- Removed `use_bearer_auth` from `ProviderConfig`: the field added complexity without user value; env var names (`ANTHROPIC_API_KEY` vs `ANTHROPIC_AUTH_TOKEN`) are sufficient to unambiguously determine auth mode.
- Added `#[serde(deny_unknown_fields)]` on `ProviderConfig` to make stale or unknown settings fields fail loudly — aligns with the principle that misconfigured settings files should error, not silently fall back.
- Kept the conflict check in `auth_status()` as an independent check (not by calling the resolver) to keep the fix minimal. The resolver remains the authoritative enforcement point; `auth_status()` mirrors the most critical check (both env vars set) for early user-facing error reporting.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] Conflict check in auth_status() for both-env-vars-set**
- **Found during:** Task 1 — UAT Scenario 2 failure diagnosis
- **Issue:** `auth_status()` calls `detect_api_key_env_source()` which silently picks `ANTHROPIC_API_KEY` when both env vars are set. The resolver's conflict check (D-02 condition 1) is never invoked from the auth status display path.
- **Fix:** Added inline conflict check in `auth_status()` before `detect_api_key_env_source()` — mirrors condition 1 from the resolver.
- **Files modified:** `crates/cli/src/main.rs`
- **Commit:** `3a9f8d0`

## Verification

```
cargo test -p claurst-core --test bearer_auth -- --test-threads=1
# 6 passed

cargo check --workspace
# exits 0

grep use_bearer_auth crates/core/src/lib.rs
# (empty)

grep deny_unknown_fields crates/core/src/lib.rs
# crates/core/src/lib.rs:856:    #[serde(deny_unknown_fields)]
```

## UAT Results (Round 2)

| Scenario | Expected | Result |
|----------|----------|--------|
| 1 — ANTHROPIC_AUTH_TOKEN via env | Logged in. | PASS |
| 2 — Both env vars set | Explicit conflict error | PASS (after fix) |
| 3 — use_bearer_auth in settings.json | Hard serde parse error | PASS (after fix) |
| 4 — ANTHROPIC_API_KEY alone | Logged in. | PASS |

## Self-Check: PASSED

- [x] `crates/core/src/lib.rs` modified (deny_unknown_fields present, use_bearer_auth absent)
- [x] `crates/core/tests/bearer_auth.rs` modified (6 tests, provider_api_key_with_auth_token_errors present)
- [x] `crates/cli/src/main.rs` modified (conflict check in auth_status)
- [x] Commit `3a9f8d0` exists

---
*Phase: 03-anthropic-auth-token-bearer-auth-support*
*Completed: 2026-05-09*
