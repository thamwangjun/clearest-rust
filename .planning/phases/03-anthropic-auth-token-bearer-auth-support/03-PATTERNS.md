# Phase 3: ANTHROPIC_AUTH_TOKEN Bearer Auth Support - Pattern Map

**Mapped:** 2026-05-06
**Files analyzed:** 4 new/modified files
**Analogs found:** 4 / 4

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/core/src/lib.rs` (ProviderConfig struct + resolve_anthropic_auth_async) | model + service | request-response | Self (existing resolver at lines 1275–1340 + struct at lines 854–885) | exact — modify in place |
| `crates/cli/src/main.rs` (auth call site update) | controller | request-response | Self (existing call site at lines 564–586) | exact — modify in place |
| `crates/core/tests/bearer_auth.rs` | test | request-response | `crates/core/tests/parity_smoke.rs` | role-match |
| `crates/core/Cargo.toml` (add serial_test dev-dep) | config | n/a | Self (existing [dev-dependencies] at line 105–107) | exact — append |

## Pattern Assignments

---

### `crates/core/src/lib.rs` — ProviderConfig struct extension (lines 854–885)

**Analog:** Same file, same struct — extend in place.

**Existing struct pattern** (lines 854–885):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// API key (overrides environment variable)
    pub api_key: Option<String>,
    /// Override the default base URL for this provider
    pub api_base: Option<String>,
    /// Whether this provider is enabled (default: true)
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Model ID whitelist (empty = allow all)
    #[serde(default)]
    pub models_whitelist: Vec<String>,
    /// Model ID blacklist
    #[serde(default)]
    pub models_blacklist: Vec<String>,
    /// Provider-specific options (passed through to provider implementation)
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            api_base: None,
            enabled: true,
            models_whitelist: Vec::new(),
            models_blacklist: Vec::new(),
            options: HashMap::new(),
        }
    }
}
```

**New field pattern to add** — copy `#[serde(default)]` style from `models_whitelist`; use `skip_serializing_if` for Option field (same convention as `api_key` which is `Option<String>`):
```rust
// Add after `options` field in struct body:
/// When Some(true), force Bearer auth (Authorization: Bearer) instead of x-api-key.
/// Mutually exclusive with api_key in this config and ANTHROPIC_API_KEY env var.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub use_bearer_auth: Option<bool>,

// Add to Default impl:
use_bearer_auth: None,
```

---

### `crates/core/src/lib.rs` — resolve_anthropic_auth_async (lines 1267–1340)

**Analog:** Same file — rewrite `resolve_anthropic_auth_async` and update `resolve_auth_async` wrapper.

**Existing resolver signature + structure** (lines 1267–1284):
```rust
pub async fn resolve_auth_async(&self) -> Option<(String, bool)> {
    if self.selected_provider_id() != "anthropic" {
        return self.resolve_api_key().map(|key| (key, false));
    }
    self.resolve_anthropic_auth_async().await
}

pub async fn resolve_anthropic_auth_async(&self) -> Option<(String, bool)> {
    if let Some(key) = self.resolve_anthropic_api_key() {
        return Some((key, false));
    }
    if let Ok(token) = std::env::var("ANTHROPIC_AUTH_TOKEN") {
        if !token.is_empty() {
            return Some((token, true));
        }
    }
    let tokens = crate::oauth::OAuthTokens::load().await?;
    // ... OAuth refresh block ...
    if let Some(cred) = tokens.effective_credential() {
        Some((cred.to_string(), tokens.uses_bearer_auth()))
    } else {
        None
    }
}
```

**Existing empty-string filter pattern** (lines 1244–1254) — copy this exactly for new conflict-check variables:
```rust
.filter(|key| !key.is_empty())
// e.g.:
let env_api_key = std::env::var("ANTHROPIC_API_KEY")
    .ok()
    .filter(|v| !v.is_empty());
```

**Error propagation pattern** — existing usage of `anyhow::bail!` in `main.rs` (line 570):
```rust
anyhow::bail!(
    "No API key found. Options:\n\
     - Set ANTHROPIC_API_KEY for Anthropic\n\
     ..."
);
```
New conflict errors must use the same `anyhow::bail!` macro (already a workspace dep in `crates/core/Cargo.toml` line 78).

**New resolver signature target** (from RESEARCH.md Pattern 1):
```rust
pub async fn resolve_anthropic_auth_async(
    &self,
) -> anyhow::Result<Option<(String, bool)>> {
    let provider_cfg = self.provider_configs.get("anthropic");
    let use_bearer_pinned = provider_cfg
        .and_then(|p| p.use_bearer_auth)
        .unwrap_or(false);

    let env_api_key = std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|v| !v.is_empty());
    let env_auth_token = std::env::var("ANTHROPIC_AUTH_TOKEN")
        .ok()
        .filter(|v| !v.is_empty());
    let provider_api_key = provider_cfg
        .and_then(|p| p.api_key.as_deref())
        .filter(|s| !s.is_empty());

    // D-02 condition 1
    if env_api_key.is_some() && env_auth_token.is_some() {
        anyhow::bail!(
            "ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN are both set; \
             these are mutually exclusive (x-api-key vs Bearer auth). \
             Unset one to continue."
        );
    }
    // D-02 condition 2
    if use_bearer_pinned && env_api_key.is_some() {
        anyhow::bail!(
            "provider_configs.anthropic.use_bearer_auth=true conflicts with \
             ANTHROPIC_API_KEY (x-api-key). Unset ANTHROPIC_API_KEY or set \
             use_bearer_auth=false."
        );
    }
    // D-02 condition 3
    if use_bearer_pinned && provider_api_key.is_some() {
        anyhow::bail!(
            "provider_configs.anthropic.use_bearer_auth=true conflicts with \
             provider_configs.anthropic.api_key. Remove the api_key or set \
             use_bearer_auth=false."
        );
    }

    // Priority 1: explicit bearer pin -> ANTHROPIC_AUTH_TOKEN env only
    if use_bearer_pinned {
        return Ok(env_auth_token.map(|t| (t, true)));
        // returns Ok(None) if token not set — main.rs "no key" branch handles it
    }
    // Priority 2: x-api-key path (settings.json -> top-level api_key -> env)
    if let Some(key) = self.resolve_anthropic_api_key() {
        return Ok(Some((key, false)));
    }
    // Priority 3: bare ANTHROPIC_AUTH_TOKEN env (no pin, no api key configured)
    if let Some(t) = env_auth_token {
        return Ok(Some((t, true)));
    }
    // Priority 4: OAuth tokens (unchanged)
    let tokens = match crate::oauth::OAuthTokens::load().await {
        Some(t) => t,
        None => return Ok(None),
    };
    // ... existing OAuth refresh block verbatim, final return wrapped in Ok(...) ...
}
```

`resolve_auth_async` wrapper must also change to propagate the `Result`:
```rust
pub async fn resolve_auth_async(&self) -> anyhow::Result<Option<(String, bool)>> {
    if self.selected_provider_id() != "anthropic" {
        return Ok(self.resolve_api_key().map(|key| (key, false)));
    }
    self.resolve_anthropic_auth_async().await
}
```

---

### `crates/cli/src/main.rs` — auth call site + config.env injection (lines 515–586)

**Analog:** Same file — two distinct hunks.

**config.env injection hunk** (lines 515–522) — already correct in working tree, keep verbatim:
```rust
// Inject config.env into the process environment so settings.json env vars
// (e.g. ANTHROPIC_AUTH_TOKEN, ANTHROPIC_BASE_URL) are visible to all resolvers.
// Real process env vars always win — only set if not already present.
for (key, value) in &config.env {
    if std::env::var(key).is_err() {
        std::env::set_var(key, value);
    }
}
```

**Auth call site — current pattern** (lines 564–586):
```rust
let active_provider = config.selected_provider_id();
let (api_key, use_bearer_auth) = if active_provider == "anthropic" {
    match config.resolve_anthropic_auth_async().await {
        Some(auth) => auth,
        None => {
            if is_headless {
                anyhow::bail!(
                    "No API key found. Options:\n\
                     - Set ANTHROPIC_API_KEY for Anthropic\n\
                     ..."
                );
            } else {
                (String::new(), false)
            }
        }
    }
} else {
    (String::new(), false)
};
```

**Auth call site — new pattern** (from RESEARCH.md Pattern 2 — add `?` to propagate conflict errors):
```rust
let active_provider = config.selected_provider_id();
let (api_key, use_bearer_auth) = if active_provider == "anthropic" {
    match config.resolve_anthropic_auth_async().await? {  // <-- ? added here
        Some(auth) => auth,
        None => {
            if is_headless {
                anyhow::bail!(
                    "No API key found. Options:\n\
                     - Set ANTHROPIC_API_KEY for Anthropic\n\
                     ..."
                );
            } else {
                (String::new(), false)
            }
        }
    }
} else {
    (String::new(), false)
};
```
The `?` propagates `anyhow::Error` upward to the `tokio::main` handler, which already returns `anyhow::Result<()>` — no other call-site changes needed.

---

### `crates/core/tests/bearer_auth.rs` — new integration test file

**Analog:** `crates/core/tests/parity_smoke.rs` (role-match)

**Existing integration test file conventions** (parity_smoke.rs lines 1–13):
```rust
//! T5-1 parity smoke tests.
//! Verifies that core data structures are usable as the TS CLI would use them.

use claurst_core::{
    session_storage::{TranscriptEntry, transcript_dir},
    // ...
};
use tempfile::TempDir;

#[test]
fn session_dir_encoding() {
    // ...
}
```

Key conventions extracted:
- Module-level doc comment with `//!` at top
- Imports via `claurst_core::` public paths (verify exact path for `Config` and `ProviderConfig` — see Assumption A1 note below)
- `#[test]` attribute for sync tests; `#[tokio::test]` for async (consistent with project's `tokio::test` usage)
- `snake_case` descriptive test function names
- `unwrap()` / `unwrap_err()` acceptable in test assertions
- No `use super::*` (integration tests use the public crate API, not module internals)

**New file full pattern** (from RESEARCH.md Code Examples):
```rust
//! Phase 3: ANTHROPIC_AUTH_TOKEN bearer auth resolution + conflict detection.

use claurst_core::{Config, ProviderConfig};  // confirmed: both re-exported at crate root via lib.rs:74
use serial_test::serial;
use std::collections::HashMap;

fn reset_anthropic_env() {
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("ANTHROPIC_AUTH_TOKEN");
}

fn anthropic_config_with(provider: ProviderConfig) -> Config {
    let mut cfg = Config::default();
    cfg.provider = Some("anthropic".into());
    cfg.provider_configs.insert("anthropic".into(), provider);
    cfg
}

#[tokio::test]
#[serial]
async fn auth_token_env_resolves_to_bearer() { ... }

#[tokio::test]
#[serial]
async fn both_env_vars_set_errors() { ... }

#[tokio::test]
#[serial]
async fn pin_bearer_with_env_api_key_errors() { ... }

#[tokio::test]
#[serial]
async fn pin_bearer_with_settings_api_key_errors() { ... }

#[tokio::test]
#[serial]
async fn config_env_injection_resolves_bearer() { ... }

// Regression: ANTHROPIC_API_KEY alone still works (Pitfall 2 guard)
#[tokio::test]
#[serial]
async fn api_key_only_resolves_to_x_api_key() { ... }
```

**Assumption A1 — RESOLVED:** Import path confirmed as `claurst_core::{Config, ProviderConfig}` (crate-root re-export via `lib.rs:74`). The `use` statement above is correct as written.

---

### `crates/core/Cargo.toml` — dev-dependencies section (lines 105–107)

**Analog:** Same file — append to `[dev-dependencies]`.

**Existing dev-dependencies** (lines 105–107):
```toml
[dev-dependencies]
tempfile = { workspace = true }
```

**New entry to append:**
```toml
serial_test = "3"
```

Result:
```toml
[dev-dependencies]
tempfile = { workspace = true }
serial_test = "3"
```

---

## Shared Patterns

### anyhow Error Propagation
**Source:** `crates/cli/src/main.rs` (lines 570–578, existing `anyhow::bail!` usage)
**Apply to:** `resolve_anthropic_auth_async` in `crates/core/src/lib.rs` (conflict bail! calls), call site in `crates/cli/src/main.rs` (add `?`)
```rust
anyhow::bail!("... descriptive error message naming both conflicting fields ...");
```

### Empty-String Filter
**Source:** `crates/core/src/lib.rs` (lines 1244, 1248, 1253, 1280–1283)
**Apply to:** All new env var reads in `resolve_anthropic_auth_async`
```rust
std::env::var("ANTHROPIC_API_KEY")
    .ok()
    .filter(|v| !v.is_empty())
```
Treat empty strings as "not set" for both conflict detection and resolution.

### Serial Test Env Isolation
**Source:** RESEARCH.md (new pattern — no existing analog in repo)
**Apply to:** All test functions in `crates/core/tests/bearer_auth.rs`
```rust
#[tokio::test]
#[serial]
async fn test_name() {
    // Reset BOTH vars at top, even if only one is under test
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("ANTHROPIC_AUTH_TOKEN");
    // ... test body ...
    // Reset again at bottom as belt-and-braces (survives mid-test panics poorly;
    // #[serial] is the real guard, but cleanup helps readability)
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("ANTHROPIC_AUTH_TOKEN");
}
```

### serde Optional Field
**Source:** `crates/core/src/lib.rs` (existing `ProviderConfig` fields lines 861–871)
**Apply to:** New `use_bearer_auth: Option<bool>` field in `ProviderConfig`
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub use_bearer_auth: Option<bool>,
```
`#[serde(default)]` ensures deserialization of old settings.json (without the field) yields `None`. `skip_serializing_if = "Option::is_none"` keeps the settings.json clean when the field is unset.

---

## No Analog Found

All files have close analogs in the codebase. No entries for this section.

---

## Metadata

**Analog search scope:** `crates/core/src/lib.rs`, `crates/cli/src/main.rs`, `crates/core/tests/`, `crates/core/Cargo.toml`
**Files scanned:** 4 source files + 2 integration test files
**Pattern extraction date:** 2026-05-06
