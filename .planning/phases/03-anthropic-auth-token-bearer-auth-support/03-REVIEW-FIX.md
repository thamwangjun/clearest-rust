---
phase: 03-anthropic-auth-token-bearer-auth-support
fixed_at: 2026-05-08T00:00:00Z
review_path: .planning/phases/03-anthropic-auth-token-bearer-auth-support/03-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 03: Code Review Fix Report

**Fixed at:** 2026-05-08T00:00:00Z
**Source review:** .planning/phases/03-anthropic-auth-token-bearer-auth-support/03-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6
- Fixed: 6
- Skipped: 0

## Fixed Issues

### CR-01: Missing conflict guard — `use_bearer_pinned=true` + `Config.api_key`

**Files modified:** `crates/core/src/lib.rs`, `crates/core/tests/bearer_auth.rs`
**Commit:** 567a186
**Applied fix:** Added D-02 condition 3b between conditions 3 and 4 in `resolve_anthropic_auth_async`: `if use_bearer_pinned && top_level_api_key.is_some()` bails with an actionable error. Added corresponding test `pin_bearer_with_top_level_api_key_errors` to `bearer_auth.rs` that sets `cfg.api_key = Some("sk-top-level-key".into())` alongside `use_bearer_auth: Some(true)` and asserts the error message contains "use_bearer_auth".

### CR-02: Weak entropy in PKCE code verifier and OAuth state generators

**Files modified:** `crates/core/src/lib.rs`
**Commit:** 4209cd8
**Applied fix:** Replaced both `generate_code_verifier()` and `generate_state()` implementations: removed the two-UUID concatenation approach (which lost 12 bits to fixed version/variant markers) and replaced with `getrandom::getrandom(&mut bytes).expect("OS CSPRNG unavailable")` on a `[0u8; 32]` buffer, giving the full 256 bits of OS CSPRNG entropy. The `getrandom` crate was already a direct workspace dependency.

### WR-01: `StatusCommand::execute` silently masks auth conflict errors

**Files modified:** `crates/commands/src/lib.rs`
**Commit:** 6c2a3e9
**Applied fix:** Changed the `Err(_)` match arm in `StatusCommand::execute` at line 1266 to `Err(e) => format!("Auth error: {e}")` so conflict errors (e.g. "ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN are both set") surface in the `/status` output instead of being swallowed as "Not authenticated".

### WR-02: `auth_status` "not logged in" hint omits `ANTHROPIC_AUTH_TOKEN`

**Files modified:** `crates/cli/src/main.rs`
**Commit:** 2deded3
**Applied fix:** Updated the hint string at line 3443 from `"Run \`claude auth login\` or set ANTHROPIC_API_KEY."` to `"Run \`claude auth login\`, set ANTHROPIC_API_KEY, or set ANTHROPIC_AUTH_TOKEN."` so users with a bearer token but no API key receive actionable guidance.

### WR-03: `env_test_mutex` in `main.rs` unit tests uses a poisonable mutex

**Files modified:** `crates/cli/src/main.rs`
**Commit:** 562260a
**Applied fix:** Applied option A — replaced all four `.lock().unwrap()` call sites in the test module with `.lock().unwrap_or_else(|p| p.into_inner())`. If any test panics while holding the lock the mutex is poisoned; subsequent tests now recover the inner guard rather than propagating spurious panics.

### IN-01: Dead variable `bare_name` — computed but not used for dispatch

**Files modified:** `crates/cli/src/main.rs`
**Commit:** e6cc199
**Applied fix:** Renamed `bare_name` to `bare_name_for_error` and added a comment clarifying that dispatch uses the full prefixed name (`self.tool_def.name`) while the stripped name is intentionally reserved for the `Err` branch error message. Updated the `Err(e)` arm to use the new name.

---

_Fixed: 2026-05-08T00:00:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
