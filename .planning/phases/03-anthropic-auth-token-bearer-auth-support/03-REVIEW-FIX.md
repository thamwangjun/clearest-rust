---
phase: 03-anthropic-auth-token-bearer-auth-support
fixed_at: 2026-05-08T00:00:00Z
review_path: .planning/phases/03-anthropic-auth-token-bearer-auth-support/03-REVIEW.md
fix_scope: all
findings_in_scope: 7
fixed: 7
skipped: 0
iteration: 1
status: all_fixed
---

# Phase 03: Code Review Fix Report

**Fixed at:** 2026-05-08
**Source review:** .planning/phases/03-anthropic-auth-token-bearer-auth-support/03-REVIEW.md
**Iteration:** 1

## Summary

All 7 findings (2 critical, 4 warnings, 1 info) were fixed. Each fix was verified to compile cleanly via `cargo check --workspace` and committed atomically. The fixes close a silent auth conflict gap, propagate actionable error messages at three call sites, redact bearer tokens from debug output, and add two missing test cases.

- Findings in scope: 7
- Fixed: 7
- Skipped: 0

## Fixed Issues

### CR-01: Config.api_key + ANTHROPIC_AUTH_TOKEN env conflict goes undetected

**Files modified:** `crates/core/src/lib.rs`
**Commit:** a4a5e24
**Applied fix:** Added `top_level_api_key` capture alongside `env_api_key`/`provider_api_key` at the top of `resolve_anthropic_auth_async()`, then added a fourth guard (D-02 condition 4) that bails with an actionable error message when both `Config.api_key` and `ANTHROPIC_AUTH_TOKEN` are simultaneously set. Previously this case silently fell through to x-api-key mode.

### CR-02: Conflict errors silently swallowed in commands/lib.rs

**Files modified:** `crates/commands/src/lib.rs`
**Commit:** 196c92f
**Applied fix:** Changed `provider_for_config()` signature from `async fn ... -> Option<...>` to `async fn ... -> anyhow::Result<Option<...>>` and replaced `.await.ok().flatten()` with `.await?`. Wrapped the existing `find_map` return in `Ok(...)`. Updated both call sites (~line 2502 code review, ~line 4307 auto-naming) to match on `Ok(Some(...))`, `Ok(None)`, and `Err(e)` — surfacing auth errors with actionable messages. Fixed `/status` call site (~line 1963) with a `match` that pushes `ERROR: {e}` into the status lines instead of swallowing the error.

### WR-01: Silent error swallow in refresh_provider_runtime_state

**Files modified:** `crates/cli/src/main.rs`
**Commit:** 1fb74b2
**Applied fix:** Replaced `.await.ok().flatten().unwrap_or((String::new(), false))` with `.await.context("Failed to resolve auth credentials during /refresh")?.unwrap_or((String::new(), false))`. Auth conflict errors during `/refresh` now propagate up and fail the operation with an actionable message.

### WR-02: BridgeConfig derives Debug exposing session_token in logs

**Files modified:** `crates/bridge/src/lib.rs`
**Commit:** 02c975c
**Applied fix:** Removed `Debug` from the `#[derive(...)]` on `BridgeConfig` and added a manual `impl std::fmt::Debug for BridgeConfig` that lists all fields but shows `session_token` as `Some("<redacted>")` or `None` (using `.as_ref().map(|_| "<redacted>")`), preventing bearer/OAuth tokens from appearing in debug logs.

### WR-03: Missing test for bearer-pinned config with no token available

**Files modified:** `crates/core/tests/bearer_auth.rs`
**Commit:** d1610bf
**Applied fix:** Added `pin_bearer_with_no_token_returns_none` test that calls `reset_anthropic_env()`, constructs a config with `use_bearer_auth: Some(true)`, and asserts `resolve_anthropic_auth_async().await.unwrap()` returns `None` (no panic, no error) when neither `ANTHROPIC_AUTH_TOKEN` nor OAuth tokens are present.

### WR-04: Missing test for injection guard preserving pre-existing env value

**Files modified:** `crates/core/tests/bearer_auth.rs`
**Commit:** 5598b2d
**Applied fix:** Added `config_env_injection_does_not_overwrite_existing_env` test that pre-sets `ANTHROPIC_AUTH_TOKEN` to `"btr-from-real-env"`, runs the injection loop simulation with `"btr-from-settings"`, and asserts the resolver returns the pre-existing real-env value. Validates the `is_err()` guard in the injection loop correctly defers to environment values over settings values.

### IN-01: serial_test pinned to major version without minor constraint

**Files modified:** `crates/core/Cargo.toml`
**Commit:** be66999
**Applied fix:** Changed `serial_test = "3"` to `serial_test = "3.1"` to lock in a known-good minor version for async + tokio compatibility, preventing unexpected breakage from future 3.x releases.

## Skipped Issues

None

---

_Fixed: 2026-05-08_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
