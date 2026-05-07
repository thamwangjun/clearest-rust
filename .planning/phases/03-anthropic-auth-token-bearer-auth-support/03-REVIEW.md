---
phase: 03-anthropic-auth-token-bearer-auth-support
reviewed: 2026-05-07T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - crates/core/tests/bearer_auth.rs
  - crates/core/Cargo.toml
  - crates/core/src/lib.rs
  - crates/cli/src/main.rs
  - crates/commands/src/lib.rs
findings:
  critical: 2
  warning: 4
  info: 1
  total: 7
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-05-07
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

Phase 3 adds `ANTHROPIC_AUTH_TOKEN` bearer auth resolution to `resolve_anthropic_auth_async()`, including conflict detection, priority ordering, and main.rs wiring. The core resolver logic (D-01..D-09) is structurally correct for the cases it tests, but has two significant correctness gaps: a silent conflict case where `Config.api_key` (top-level field set by CLI `--api-key` flag) coexists with `ANTHROPIC_AUTH_TOKEN` env — the resolver silently prefers the API key without surfacing an error or warning — and two call sites in `commands/lib.rs` and `main.rs` that swallow conflict errors silently via `.ok()`, presenting the user with a generic provider-init failure instead of the actionable conflict message the resolver produces.

---

## Critical Issues

### CR-01: `Config.api_key` + `ANTHROPIC_AUTH_TOKEN` env conflict goes undetected

**File:** `crates/core/src/lib.rs:1281-1329`

**Issue:** The three conflict-detection guards at lines 1297–1320 only check `env_api_key` (read from the `ANTHROPIC_API_KEY` env var) and `provider_api_key` (read from `provider_configs.anthropic.api_key`). Neither guard checks `Config.api_key` — the top-level field that is populated when the user passes `--api-key` on the CLI or when `settings.json` has a root-level `api_key`.

When `Config.api_key` is set alongside `ANTHROPIC_AUTH_TOKEN` in the environment, none of the three guards fire. Execution falls through to Priority 2 (`resolve_anthropic_api_key()`), which returns `Config.api_key` and silently returns `(key, false)`, completely ignoring the auth-token. The user gets x-api-key mode when they configured bearer auth, with no error or warning.

`resolve_anthropic_api_key()` at line 1246 checks `self.api_key` first, before `provider_configs.api_key` and the env var — so any non-empty `Config.api_key` shadows `ANTHROPIC_AUTH_TOKEN` without detection.

**Fix:** Add a fourth guard before the Priority 2 branch. The top-level API key should be captured early in the function, mirroring how `env_api_key` and `provider_api_key` are captured:

```rust
// At the top of resolve_anthropic_auth_async, alongside env_api_key / provider_api_key:
let top_level_api_key = self.api_key
    .as_deref()
    .filter(|s| !s.is_empty());

// New guard (D-02 condition 4): top-level api_key + ANTHROPIC_AUTH_TOKEN env
if top_level_api_key.is_some() && env_auth_token.is_some() {
    anyhow::bail!(
        "Config api_key and ANTHROPIC_AUTH_TOKEN are both set; \
         these are mutually exclusive (x-api-key vs Bearer auth). \
         Unset one to continue."
    );
}
```

---

### CR-02: Conflict errors silently swallowed in `commands/lib.rs` — actionable error lost

**File:** `crates/commands/src/lib.rs:153`, `crates/commands/src/lib.rs:1963`

**Issue:** Both call sites use `.await.ok().flatten()` on `resolve_anthropic_auth_async()`, which converts an `Err` (the conflict/misconfiguration message) into `None`. The actual diagnostic — e.g., `"ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN are both set"` — is discarded.

At line 153, `provider_for_config` then returns `None`, and callers (lines 2502, 4307) surface a generic `"Cannot initialise provider client for code review."`. The user has no way to know the root cause is an auth conflict.

At line 1963 (`/status` command), the conflict is silently converted to empty credentials `("", false)`, which then reports a misleading "provider not healthy" or "no auth" status rather than identifying the real problem.

**Fix:** Propagate the error instead of swallowing it.

For `provider_for_config` (line 152), change its signature to return `Result<Option<...>>` or `anyhow::Result<...>`:

```rust
async fn provider_for_config(
    config: &Config,
) -> anyhow::Result<Option<std::sync::Arc<dyn claurst_api::LlmProvider>>> {
    let anthropic_auth = config.resolve_anthropic_auth_async().await?;
    // ... rest unchanged
}
```

For the `/status` command at line 1963, surface the error to the status output:

```rust
let anthropic_auth = match ctx.config.resolve_anthropic_auth_async().await {
    Ok(auth) => auth.unwrap_or((String::new(), false)),
    Err(e) => {
        lines.push(format!("  ERROR: {e}"));
        (String::new(), false)
    }
};
```

---

## Warnings

### WR-01: Silent error swallow in `refresh_provider_runtime_state` loses conflict signal

**File:** `crates/cli/src/main.rs:942-947`

**Issue:** Inside `refresh_provider_runtime_state`, `resolve_anthropic_auth_async().await.ok().flatten().unwrap_or(...)` silently discards a conflict error. This function already propagates other errors via `?` and returns `anyhow::Result`. The inconsistency means an auth conflict during a `/refresh` operation will silently produce an empty API key, leaving the refreshed client in an unauthenticated state with no indication why.

**Fix:**

```rust
let (api_key, use_bearer_auth) = config
    .resolve_anthropic_auth_async()
    .await
    .context("Failed to resolve auth credentials during /refresh")?
    .unwrap_or((String::new(), false));
```

---

### WR-02: `resolve_bridge_config` stores bearer OAuth token as bridge session token without documented intent

**File:** `crates/cli/src/main.rs:331-332`

**Issue:** When `use_bearer_auth=true`, `auth_credential` is the OAuth access token (a long-lived bearer credential). The function stores it verbatim as `bridge_config.session_token`. `BridgeConfig` derives `Debug` (see `crates/bridge/src/lib.rs:155`), so any code path that debug-prints a `BridgeConfig` (e.g., `{:?}` formatting in tracing/log macros) will expose the raw bearer token in logs.

The `info!` log at line 608 does not log the full struct, so this is not currently a direct leak, but the `Debug` derivation on a type containing `session_token: Option<String>` is an ongoing risk.

**Fix:** Implement a manual `Debug` for `BridgeConfig` in `crates/bridge/src/lib.rs` that redacts `session_token`:

```rust
impl std::fmt::Debug for BridgeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BridgeConfig")
            .field("enabled", &self.enabled)
            .field("server_url", &self.server_url)
            .field("device_id", &self.device_id)
            .field("session_token", &self.session_token.as_ref().map(|_| "<redacted>"))
            // ... other non-sensitive fields
            .finish()
    }
}
```

---

### WR-03: Missing test: `use_bearer_auth=true` pinned with no token available returns `None` silently

**File:** `crates/core/tests/bearer_auth.rs`

**Issue:** The priority-1 path at `lib.rs:1323-1324` is `return Ok(env_auth_token.map(|t| (t, true)))`. When `use_bearer_auth=true` is pinned but `ANTHROPIC_AUTH_TOKEN` is not set and no OAuth tokens exist, this returns `Ok(None)`. There is no test verifying this behavior. The absence of a credential in bearer-pinned mode would result in a silent empty-credential state in `main.rs` (line 580 produces `(String::new(), false)` for the interactive case), which is confusing because the user explicitly requested bearer mode.

**Fix:** Add a test:

```rust
#[tokio::test]
#[serial]
async fn pin_bearer_with_no_token_returns_none() {
    reset_anthropic_env();
    // No ANTHROPIC_AUTH_TOKEN set, no OAuth tokens on disk
    let cfg = anthropic_config_with(ProviderConfig {
        use_bearer_auth: Some(true),
        ..ProviderConfig::default()
    });
    let res = cfg.resolve_anthropic_auth_async().await.unwrap();
    assert_eq!(res, None);
    reset_anthropic_env();
}
```

Additionally, consider whether `Ok(None)` is the right return for this case or whether it should be `Err(...)` with an explicit message since the user has positively opted in to bearer mode.

---

### WR-04: `config_env_injection_resolves_bearer` test replicates injection logic inline — not testing actual main.rs path

**File:** `crates/core/tests/bearer_auth.rs:118-137`

**Issue:** The test comment states it "reproduces the loop inline" because `crates/cli` depends on `crates/core`, not the reverse. The inline reproduction copies the `if std::env::var(k).is_err()` guard correctly. However, the test does not verify the behavior when `ANTHROPIC_AUTH_TOKEN` is already set in the real environment before the injection (i.e., that the injection guard correctly defers to the pre-existing value). This is the edge case the guard is designed for, but it goes untested.

More importantly, if `main.rs`'s injection loop is ever modified (e.g., the `is_err()` guard removed), the inline test will not catch it because it does not call through `main.rs`.

**Fix:** Add a complementary test asserting that a pre-existing env value is not overwritten by the injection:

```rust
#[tokio::test]
#[serial]
async fn config_env_injection_does_not_overwrite_existing_env() {
    reset_anthropic_env();
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", "btr-from-real-env");

    // Simulate injection with a different value from settings
    let mut env = HashMap::new();
    env.insert("ANTHROPIC_AUTH_TOKEN".into(), "btr-from-settings".into());
    for (k, v) in &env {
        if std::env::var(k).is_err() {
            std::env::set_var(k, v);
        }
    }

    let cfg = Config::default();
    let res = cfg.resolve_anthropic_auth_async().await.unwrap();
    // Real env wins; settings value must not overwrite
    assert_eq!(res, Some(("btr-from-real-env".to_string(), true)));

    reset_anthropic_env();
}
```

---

## Info

### IN-01: `serial_test` pinned to major version `"3"` without a minor constraint

**File:** `crates/core/Cargo.toml:107`

**Issue:** The dependency `serial_test = "3"` uses a bare major version, which will automatically pull in any `3.x.y` release, including breaking patch-level changes in the serialization behavior or async runtime integration. For a test dependency that directly affects test isolation correctness (env var serialization), minor version pinning is advisable.

**Fix:**

```toml
serial_test = "3.1"
```

Use the most recently tested minor version to lock in known-good async + tokio compatibility.

---

_Reviewed: 2026-05-07_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
