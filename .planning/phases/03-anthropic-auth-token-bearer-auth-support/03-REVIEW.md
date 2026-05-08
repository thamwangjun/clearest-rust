---
phase: 03-anthropic-auth-token-bearer-auth-support
reviewed: 2026-05-08T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - crates/cli/src/main.rs
  - crates/commands/src/lib.rs
  - crates/core/Cargo.toml
  - crates/core/src/lib.rs
  - crates/core/tests/bearer_auth.rs
findings:
  critical: 2
  warning: 3
  info: 1
  total: 6
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-05-08T00:00:00Z
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

This phase adds `ANTHROPIC_AUTH_TOKEN` bearer auth support: a new resolver (`resolve_anthropic_auth_async`), conflict detection for mutually exclusive auth modes, a `use_bearer_auth` pin in `ProviderConfig`, bearer-aware display in `claude auth status` and `/status`, and a `detect_api_key_env_source` helper. The core logic is sound for the primary happy paths, but two correctness gaps remain: one missing conflict-detection case that lets a contradictory configuration silently succeed, and weak entropy in the PKCE/state generators. Three additional warnings cover swallowed errors, an incomplete user-facing hint, and a fragile test isolation pattern.

---

## Critical Issues

### CR-01: Missing conflict guard — `use_bearer_auth=true` + `Config.api_key` (top-level)

**File:** `crates/core/src/lib.rs:1308-1331`

**Issue:** The resolver guards three of four conflict combinations involving `use_bearer_pinned`:
- condition 2: `use_bearer_pinned && env_api_key` — error
- condition 3: `use_bearer_pinned && provider_api_key` — error
- condition 4: `top_level_api_key && env_auth_token` — error

But the combination `use_bearer_pinned=true && top_level_api_key.is_some()` has **no guard**. When a user has `use_bearer_auth=true` in `provider_configs.anthropic` alongside a top-level `Config.api_key` (e.g. set via `--api-key` CLI flag or directly in settings), the resolver silently falls through to Priority 2 (`resolve_anthropic_api_key`), which picks up the top-level key and returns `(key, false)` — the opposite of what `use_bearer_pinned` requested. The user's intent is violated without any warning.

**Fix:**
```rust
// Add after condition 3 (line 1323), before condition 4:
// D-02 condition 3b: bearer pin + top-level Config.api_key
if use_bearer_pinned && top_level_api_key.is_some() {
    anyhow::bail!(
        "provider_configs.anthropic.use_bearer_auth=true conflicts with \
         Config.api_key (x-api-key mode). \
         Remove api_key or set use_bearer_auth=false."
    );
}
```

Also add a corresponding test in `crates/core/tests/bearer_auth.rs` mirroring `pin_bearer_with_settings_api_key_errors` but setting `cfg.api_key = Some(...)` instead of `provider.api_key`.

---

### CR-02: Weak entropy in PKCE code verifier and OAuth state generators

**File:** `crates/core/src/lib.rs:3559-3587`

**Issue:** Both `generate_code_verifier()` and `generate_state()` build their random bytes by concatenating two `uuid::Uuid::new_v4()` values. UUID v4 has 6 fixed bits (variant + version markers) out of 128, so the effective entropy is 122 + 122 = 244 bits, not 256 bits as the "32-byte random" comment states. More importantly, using `Uuid::new_v4()` as the entropy source obscures the security dependency: if `getrandom` feature flags are misconfigured (e.g. on non-standard targets), the UUID crate may silently fall back to a weak or panicking RNG, whereas calling `getrandom::getrandom` directly surfaces the failure immediately.

The `getrandom` crate is already a direct workspace dependency (`crates/core/Cargo.toml:99`). The PKCE state parameter in particular must satisfy RFC 6749 §10.12 ("unguessable value"), so the entropy source should be explicit and auditable.

**Fix:**
```rust
pub fn generate_code_verifier() -> String {
    use base64::Engine;
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("OS CSPRNG unavailable");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub fn generate_state() -> String {
    use base64::Engine;
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("OS CSPRNG unavailable");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}
```

---

## Warnings

### WR-01: `StatusCommand::execute` silently masks auth conflict errors

**File:** `crates/commands/src/lib.rs:1266`

**Issue:** The `Err(_)` arm returns the string `"Not authenticated"` without surfacing the error message. If `resolve_anthropic_auth_async` returns an `Err` (e.g. the D-02 conflict "ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN are both set"), the user running `/status` sees `Auth: Not authenticated` instead of the actual conflict explanation. A misconfigured user appears unauthenticated rather than getting the actionable error message.

**Fix:**
```rust
let auth_status = match ctx.config.resolve_anthropic_auth_async().await {
    Ok(Some((_, true))) => "Authenticated (Bearer token)".to_string(),
    Ok(Some((_, false))) => "Authenticated (API key)".to_string(),
    Ok(None) => "Not authenticated".to_string(),
    Err(e) => format!("Auth error: {e}"),
};
```

---

### WR-02: `auth_status` "not logged in" hint omits `ANTHROPIC_AUTH_TOKEN`

**File:** `crates/cli/src/main.rs:3442-3443`

**Issue:** When the user is not logged in and the active provider is Anthropic, the hint reads:

```
Run `claude auth login` or set ANTHROPIC_API_KEY.
```

`ANTHROPIC_AUTH_TOKEN` is a valid credential path introduced in this phase, but it is not mentioned. A user who has a bearer token available but no API key will not find the correct guidance from this message.

**Fix:**
```rust
"Run `claude auth login`, set ANTHROPIC_API_KEY, or set ANTHROPIC_AUTH_TOKEN.".to_string()
```

---

### WR-03: `env_test_mutex` in `main.rs` unit tests uses a poisonable mutex

**File:** `crates/cli/src/main.rs:3580-3582`

**Issue:** The test helper `env_test_mutex()` uses `std::sync::Mutex` and all callers call `.lock().unwrap()`. If any test panics while holding the lock, the mutex becomes poisoned and all subsequent tests calling `.lock().unwrap()` will also panic — producing spurious failures that look like real test failures. The integration tests in `crates/core/tests/bearer_auth.rs` use `#[serial]` from `serial_test` for test ordering, which does not have this poisoning hazard.

**Fix (option A — tolerate poisoning):**
```rust
let _guard = env_test_mutex().lock().unwrap_or_else(|p| p.into_inner());
```

**Fix (option B — align with bearer_auth.rs pattern):**
Replace the hand-rolled mutex guard with `#[serial]` from `serial_test` (add `serial_test` to `[dev-dependencies]` in the `cli` crate's `Cargo.toml` and annotate each env-sensitive test with `#[serial]`).

---

## Info

### IN-01: Dead variable `bare_name` — computed but not used for dispatch

**File:** `crates/cli/src/main.rs:86-91`

**Issue:** `bare_name` strips the server-name prefix from `self.tool_def.name`, but `manager.call_tool(...)` on line 95 is called with `&self.tool_def.name` (the full prefixed name). The stripping has no effect on dispatch. `bare_name` is only used in the `Err` branch for the error message, so either the dispatch call should use `bare_name`, or the variable should be renamed `bare_name_for_error` with a comment to clarify intent.

**Fix:**
```rust
// Rename to document intent:
let bare_name_for_error = self
    .tool_def
    .name
    .strip_prefix(&prefix)
    .unwrap_or(&self.tool_def.name);

// …
Err(e) => ToolResult::error(format!("MCP tool '{}' failed: {}", bare_name_for_error, e)),
```

---

_Reviewed: 2026-05-08T00:00:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
