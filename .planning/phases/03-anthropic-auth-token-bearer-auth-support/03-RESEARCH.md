# Phase 03: ANTHROPIC_AUTH_TOKEN Bearer Auth Support - Research

**Researched:** 2026-05-06
**Domain:** Rust auth-resolution layer in `claurst-core` + `claurst-cli`; integration tests with serialised env-var mutation
**Confidence:** HIGH

## Summary

This phase upgrades `resolve_anthropic_auth_async()` from "first non-empty wins" to a strict, conflict-aware resolver that distinguishes `ANTHROPIC_API_KEY` (x-api-key mode) from `ANTHROPIC_AUTH_TOKEN` (bearer mode) and errors when both credential paths are simultaneously active. A new `use_bearer_auth: Option<bool>` user-facing flag is added to `ProviderConfig` so users can pin bearer mode without relying on env-var naming. The `config.env` injection loop already in the working tree is folded into the same plan so settings.json env vars become visible to the resolver before it runs.

The downstream API layer (`crates/api/src/lib.rs`, `crates/api/src/client.rs`) **already** supports `use_bearer_auth: bool` end-to-end — both `send_with_retry()` and `fetch_available_models()` branch on the flag and emit `Authorization: Bearer ...` vs `x-api-key: ...`. No changes are required there. All work is concentrated in `crates/core/src/lib.rs`, `crates/cli/src/main.rs`, and a new integration test file.

**Primary recommendation:** Change `resolve_anthropic_auth_async()`'s return type to `anyhow::Result<Option<(String, bool)>>`, perform conflict detection on entry, then evaluate credential sources in priority order. Keep the existing working-tree `config.env` injection in `main.rs` unchanged. Add `serial_test = "3"` as a dev-dependency on `claurst-core` and write the regression suite in `crates/core/tests/bearer_auth.rs` with `#[serial]` on every env-mutating test.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Credential Exclusivity (the core new constraint)
- **D-01:** `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` are **mutually exclusive**. The auth resolver must error — not silently pick one — when both are active.
- **D-02:** A conflict exists under **any** of these conditions:
  1. Both `ANTHROPIC_API_KEY` env var and `ANTHROPIC_AUTH_TOKEN` env var are non-empty at the same time
  2. `use_bearer_auth: true` is set in `provider_configs.anthropic` AND `ANTHROPIC_API_KEY` env var is non-empty
  3. `use_bearer_auth: true` is set in `provider_configs.anthropic` AND `api_key` is set in `provider_configs.anthropic` (settings.json)
- **D-03:** Conflict detection lives inside `resolve_anthropic_auth_async()`. Change its return type from `Option<(String, bool)>` to `Result<Option<(String, bool)>, <error type>>` (use `anyhow::Error` or a dedicated `AuthConflictError`). The caller in `main.rs` already propagates errors via `?` — updating the call site is minimal.

#### ProviderConfig Extension
- **D-04:** Add `use_bearer_auth: Option<bool>` to the `ProviderConfig` struct in `crates/core/src/lib.rs`. Default is `None` (no opinion). When `Some(true)`, the resolver treats this as if `ANTHROPIC_AUTH_TOKEN` is the intended credential path and applies conflict checks accordingly.

#### config.env Injection
- **D-05:** The `config.env` injection loop (`for (key, value) in &config.env { if std::env::var(key).is_err() { std::env::set_var(key, value); } }`) is **already in the working tree** in `crates/cli/src/main.rs`. Fold it into the main plan — do not commit it separately.

#### Uncommitted Changes Handling
- **D-06:** Two working-tree diffs exist but are uncommitted: (1) `ANTHROPIC_AUTH_TOKEN` env check in `resolve_anthropic_auth_async`, (2) `config.env` injection in `main.rs`. **Fold both into the main plan.** The plan will rewrite/extend the resolver anyway (conflict detection changes the signature); treat the existing diffs as a starting delta that the plan overwrites cleanly.

#### Regression Tests
- **D-07:** Tests live in a **new file** `crates/core/tests/bearer_auth.rs` (separate integration test file, not inline in lib.rs).
- **D-08:** Use `serial_test` as a dev-dependency; annotate all env-mutating tests with `#[serial]`. Reset relevant env vars at the top of each test (clear both `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` before setting the test-specific state).
- **D-09:** Test coverage must include:
  1. **Happy path — ANTHROPIC_AUTH_TOKEN env:** Set `ANTHROPIC_AUTH_TOKEN`, clear `ANTHROPIC_API_KEY` → `resolve_anthropic_auth_async()` returns `Ok(Some((token, true)))`.
  2. **Conflict 1:** Both env vars set → returns `Err(...)` with a message mentioning the conflict.
  3. **Conflict 2:** `use_bearer_auth: true` in config + `ANTHROPIC_API_KEY` env set → `Err(...)`.
  4. **Conflict 3:** `use_bearer_auth: true` in config + `api_key` in provider config → `Err(...)`.
  5. **config.env injection:** Config with `env: { "ANTHROPIC_AUTH_TOKEN": "btr-..." }`, no process env vars set → after injection, `ANTHROPIC_AUTH_TOKEN` is visible and auth resolves to bearer mode.

### Claude's Discretion
- Exact error type (anyhow::Error with context vs. a dedicated AuthConflictError enum variant)
- Error message wording — should clearly name both conflicting fields
- Whether to add `ANTHROPIC_AUTH_TOKEN` to the `import_config` migration allowlist

### Deferred Ideas (OUT OF SCOPE)
- Support `ANTHROPIC_AUTH_TOKEN` in named-command and ACP paths (`crates/commands/src/lib.rs` ~line 162, 1967) — these construct their own `ClientConfig`; bearer support can be added in a follow-up phase
- Expose bearer mode toggle in the onboarding provider setup UI
- Add `ANTHROPIC_AUTH_TOKEN` to `api_key_env_vars_for_provider("anthropic")` return slice — changes semantics; requires an audit of all callers
</user_constraints>

<phase_requirements>
## Phase Requirements

No requirement IDs are tracked in `.planning/REQUIREMENTS.md` for this phase (REQUIREMENTS.md only carries `BUG-01` for Phase 1; Phase 3 is owner-scoped tooling work). The CONTEXT.md decisions D-01 through D-09 act as the requirement set for this phase. The plan must address all nine.

| ID (synthetic) | Description | Research Support |
|----|-------------|------------------|
| D-01..D-03 | Mutual exclusivity + Result return | "Architecture Patterns / Pattern 1: Conflict-First Resolver" |
| D-04 | ProviderConfig.use_bearer_auth | "Code Examples / ProviderConfig change" |
| D-05/D-06 | config.env injection + working-tree fold-in | "Working Tree Delta" in canonical refs (CONTEXT) — kept verbatim |
| D-07/D-08 | New integration test file with serial_test | "Standard Stack" + "Code Examples / Integration test boilerplate" |
| D-09 | Five test cases | "Validation Architecture / Phase Requirements -> Test Map" |
</phase_requirements>

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Credential discovery + conflict detection | `claurst-core` (config / resolver) | — | Pure logic over `Config` + env vars; no I/O beyond `std::env::var`. Belongs with the rest of `Config::resolve_*` in `crates/core/src/lib.rs`. |
| Process env injection from `config.env` | `claurst-cli` (`main.rs` startup) | — | Side-effecting and process-global; correctly placed in the binary entry point before the resolver runs. Library code must not mutate process env. |
| Header dispatch (Bearer vs x-api-key) | `claurst-api` (`AnthropicClient`) | — | Already implemented and verified — out of scope for changes. |
| User-facing settings schema (`use_bearer_auth`) | `claurst-core` (`ProviderConfig`) | — | Same struct already holds `api_key`, `api_base`, `enabled`; adding the flag here keeps serde derives intact. |
| Regression tests | `claurst-core` integration tests (`tests/bearer_auth.rs`) | — | Tests must construct a real `Config`, mutate process env, and call `resolve_anthropic_auth_async().await`. Integration test file isolates them from inline `mod tests` in `lib.rs`, which is the convention per `.planning/codebase/TESTING.md`. |

## Standard Stack

### Core (already in tree, no version bump needed)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `anyhow` | workspace `1` | Error type for `resolve_anthropic_auth_async` Result | All other startup errors in `main.rs` use `anyhow::Result` + `?`; fits existing propagation. [VERIFIED: workspace `Cargo.toml`] |
| `tokio` | workspace `1.44` | `#[tokio::test]` for the async resolver tests | Already used for the only async resolver in scope. [VERIFIED: workspace `Cargo.toml`] |
| `serde` | workspace `1` | Adding `Option<bool>` field to a `Serialize/Deserialize` struct | `ProviderConfig` already derives both. [VERIFIED: `crates/core/src/lib.rs:854`] |

### Supporting (new dev-dependency)
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serial_test` | `3.2` | Serialise env-var-mutating tests so they don't race | Mandated by D-08. Latest stable is 3.2.0; MSRV 1.68.2 — compatible with workspace's Rust 1.95. [VERIFIED: crates.io / docs.rs serial_test 3.2.0] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `serial_test` | `sealed_test` | `sealed_test` forks a subprocess per test for stronger isolation; heavier, slower, and not necessary because we only need serialisation, not full process isolation. CONTEXT.md D-08 already locks `serial_test`. |
| `anyhow::Error` for conflicts | Dedicated `AuthConflictError` enum (thiserror) | A dedicated enum is more typed but adds a public surface area. Recommendation: use `anyhow::Error` with `.context("ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN are both set ...")` — matches all other `resolve_*` paths. (D-08 marks this as Claude's discretion.) |

**Installation:**

In `crates/core/Cargo.toml`, under `[dev-dependencies]`:

```toml
[dev-dependencies]
tempfile = { workspace = true }
serial_test = "3"
```

(Optionally promote to `[workspace.dependencies]` if other crates may need it later — Phase 3 only needs it in `claurst-core`.)

**Version verification:** `serial_test` 3.2.0 confirmed as latest on crates.io and docs.rs (no newer 4.x line). [VERIFIED: docs.rs/crate/serial_test/latest, crates.io/crates/serial_test]

## Architecture Patterns

### System Architecture Diagram

```
                   ┌────────────────────────────────────────┐
                   │        process startup (main.rs)       │
                   └────────────────────────────────────────┘
                                       │
                  1. load Config (settings.json + CLI args)
                                       │
                                       ▼
                   ┌────────────────────────────────────────┐
                   │  config.env injection loop (line ~518) │  <-- D-05 (already in tree)
                   │  std::env::set_var(k,v) if not set     │
                   └────────────────────────────────────────┘
                                       │
                                       ▼
              if active_provider == "anthropic"
                                       │
                                       ▼
                   ┌────────────────────────────────────────┐
                   │  Config::resolve_anthropic_auth_async()│  <-- D-01..D-04 changes
                   │                                        │
                   │   step 1: detect conflicts (D-02)      │
                   │     ├─ both env vars non-empty? -> Err │
                   │     ├─ use_bearer_auth=true            │
                   │     │  + ANTHROPIC_API_KEY set? -> Err │
                   │     └─ use_bearer_auth=true            │
                   │        + provider api_key set? -> Err  │
                   │                                        │
                   │   step 2: resolve in priority order    │
                   │     ├─ if use_bearer_auth=Some(true)   │
                   │     │   -> read ANTHROPIC_AUTH_TOKEN   │
                   │     ├─ resolve_anthropic_api_key()     │
                   │     │   -> (key, false)                │
                   │     ├─ ANTHROPIC_AUTH_TOKEN env        │
                   │     │   -> (token, true)               │
                   │     └─ OAuthTokens::load (existing)    │
                   │                                        │
                   │   returns: Result<Option<(String,bool)>>│
                   └────────────────────────────────────────┘
                                       │
                                       ▼
                   ┌────────────────────────────────────────┐
                   │  ClientConfig { api_key, use_bearer_auth }
                   │  -> AnthropicClient::new                │  (already supports both)
                   └────────────────────────────────────────┘
                                       │
                                       ▼
                   ┌────────────────────────────────────────┐
                   │ AnthropicClient::send_with_retry        │
                   │  if use_bearer_auth:                    │
                   │    Authorization: Bearer <key>          │
                   │  else:                                  │
                   │    x-api-key: <key>                     │
                   └────────────────────────────────────────┘
```

### Component Responsibilities

| File | Responsibility | Phase 3 changes |
|------|----------------|-----------------|
| `crates/core/src/lib.rs` (~line 855) | `ProviderConfig` struct definition | Add `use_bearer_auth: Option<bool>` field with `#[serde(default, skip_serializing_if = "Option::is_none")]` |
| `crates/core/src/lib.rs` (~line 1275) | `resolve_anthropic_auth_async()` | Change signature to `anyhow::Result<Option<(String, bool)>>`; add conflict checks; honour `use_bearer_auth: Some(true)` from provider config |
| `crates/core/src/lib.rs` (~line 1267) | `resolve_auth_async()` (wrapper) | Update return type to match the inner resolver |
| `crates/cli/src/main.rs` (~line 515) | `config.env` injection loop | Already in working tree (D-05) — keep as-is |
| `crates/cli/src/main.rs` (~line 564–586) | Auth resolution call site | Update `match` arm to handle the new `Result`; map `Err` to a startup error (use `?` plus a contextual message) |
| `crates/core/tests/bearer_auth.rs` | New integration test file | All five D-09 test cases |
| `crates/core/Cargo.toml` | Crate manifest | Add `serial_test = "3"` to `[dev-dependencies]` |

### Pattern 1: Conflict-First Resolver
**What:** Run all conflict detection before any priority resolution.
**When to use:** Whenever a function chooses between mutually exclusive sources of the same data.
**Example:**
```rust
// crates/core/src/lib.rs (sketch)
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

    // Priority order (after conflicts cleared):
    // 1) explicit pinned bearer mode -> read ANTHROPIC_AUTH_TOKEN
    if use_bearer_pinned {
        if let Some(t) = env_auth_token {
            return Ok(Some((t, true)));
        }
        // pinned but no token: fall through to OAuth/none
    }
    // 2) existing api-key resolution path (settings.json -> env -> Config.api_key)
    if let Some(key) = self.resolve_anthropic_api_key() {
        return Ok(Some((key, false)));
    }
    // 3) bare ANTHROPIC_AUTH_TOKEN env (no pin)
    if let Some(t) = env_auth_token {
        return Ok(Some((t, true)));
    }
    // 4) OAuth tokens (unchanged from current implementation)
    // ... existing OAuth refresh block, returning Ok(Some((..., true))) or Ok(None)
}
```

### Pattern 2: Caller error propagation
**What:** Convert `Option<(String,bool)>` -> `Result<Option<(String,bool)>>` with minimal call-site churn.
**Where:** `crates/cli/src/main.rs:566`. Current code is `match config.resolve_anthropic_auth_async().await { Some(auth) => ..., None => ... }`. New code:
```rust
let (api_key, use_bearer_auth) = if active_provider == "anthropic" {
    match config.resolve_anthropic_auth_async().await? {
        Some(auth) => auth,
        None => { /* existing 'no key found' branch unchanged */ }
    }
} else {
    (String::new(), false)
};
```
The `?` propagates the conflict error up to `tokio::main`'s `anyhow::Result<()>` and prints it as a clean startup error.

### Anti-Patterns to Avoid
- **Adding `ANTHROPIC_AUTH_TOKEN` to `api_key_env_vars_for_provider("anthropic")`** — explicitly deferred (CONTEXT.md). That function feeds `resolve_anthropic_api_key()` which assumes x-api-key semantics; mixing in a bearer token there would silently regress to the wrong header. Leave the slice as `&["ANTHROPIC_API_KEY"]`.
- **Calling the resolver before `config.env` injection** — order matters. The injection loop (line ~518) must run before line ~566. Already correct; do not refactor.
- **Mutating env vars from library code** (`crates/core`) — keep all `std::env::set_var` calls in `crates/cli/src/main.rs`. Library tests are the exception (they have to set env to exercise the resolver).
- **Asserting on exact error message text** in tests — assert on substring (e.g., `err.to_string().contains("ANTHROPIC_API_KEY")`); message wording is Claude's discretion and may evolve.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Serialising env-mutating tests | A custom `Mutex<()>` static or hand-rolled lock module | `serial_test` `#[serial]` attribute | Battle-tested, supports test groups (`#[serial(env)]`), survives panics, integrates with `cargo test` parallel runner. Hand-rolled mutex is easy to forget on new tests. |
| Error type for the resolver | A new `AuthConflictError` enum unless really needed | `anyhow::Error` with `.context()` or `bail!` | Matches every other `main.rs` startup path; downstream callers only need a printable error. |
| Reading `ANTHROPIC_AUTH_TOKEN` priority logic | Re-implementing inside callers | Centralise in `resolve_anthropic_auth_async()` only | All other call sites (named_commands, ACP) are deferred — keep the single resolver as the choke point. |

**Key insight:** The Bearer/x-api-key dispatch is already implemented in `crates/api/src/lib.rs:700–703` and `:753–756`. Phase 3 is purely about **upgrading the credential resolver and config schema**, not touching the wire protocol.

## Runtime State Inventory

This phase **adds a new field** and **upgrades a function signature**; it is not a rename or migration. Inventory still useful:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | `~/.claurst/settings.json` carries `provider_configs.anthropic` and `config.env`. The new `use_bearer_auth: Option<bool>` field is additive; existing settings.json files remain valid because `Option<bool>` defaults to `None` via `#[serde(default)]`. | None — backward compatible by construction. Verify by writing a roundtrip test that loads a settings.json without the field and confirms `use_bearer_auth == None`. |
| Live service config | n/a — local CLI, no remote config | None |
| OS-registered state | None — Phase 3 changes nothing about how the binary is registered or launched | None |
| Secrets / env vars | `ANTHROPIC_API_KEY` (existing semantics unchanged); `ANTHROPIC_AUTH_TOKEN` (newly first-class). Process env vars are read at startup; user must `unset` one if both are present. The resolver now actively errors on conflict instead of preferring one. | Document the new exclusivity in the error message itself (no separate doc artefact required by CONTEXT.md). |
| Build artifacts / installed packages | None — pure source change, `cargo build` regenerates everything from this commit forward | None |

## Common Pitfalls

### Pitfall 1: Test pollution from process-global env vars
**What goes wrong:** One test sets `ANTHROPIC_AUTH_TOKEN`, another concurrent test reads it, sees a value it didn't set, and asserts unexpectedly.
**Why it happens:** `cargo test` runs tests in parallel by default. `std::env` mutation is process-global.
**How to avoid:** Annotate every env-mutating test with `#[serial]` (D-08). Reset both `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` at the **top** of each test using `std::env::remove_var`, even when only one is being exercised — leftover state from a prior test could otherwise trigger a conflict path.
**Warning signs:** Flaky tests; tests that pass alone but fail in `cargo test`; conflict errors in tests that don't intentionally set both vars.

### Pitfall 2: Order-of-fallback regressions
**What goes wrong:** After adding the conflict check, the existing precedence (Config.api_key > provider_configs.anthropic.api_key > env > OAuth) silently changes for users who set neither env var.
**Why it happens:** Easy to refactor the priority ladder while moving conditions around.
**How to avoid:** The plan must include a regression test asserting that with **only** `ANTHROPIC_API_KEY` set (no `use_bearer_auth`, no auth token), the resolver still returns `Ok(Some((key, false)))` exactly as before. Treat this as a pre-existing behaviour smoke test — not in D-09's five but worth adding.
**Warning signs:** OAuth tests start firing in unrelated environments; users report "my key stopped working".

### Pitfall 3: Forgetting `use_bearer_auth=Some(true)` with no token
**What goes wrong:** User pins bearer mode but forgets to set `ANTHROPIC_AUTH_TOKEN`. Resolver falls through to `resolve_anthropic_api_key()`, returns an x-api-key, and bearer pinning is silently ignored — the user gets x-api-key auth despite pinning.
**Why it happens:** The fallback logic is shared between modes.
**How to avoid:** When `use_bearer_pinned == true` and no auth token is found, **do not fall through to `resolve_anthropic_api_key()`** — either return `Ok(None)` (let `main.rs` show "no key found") or return an error like "use_bearer_auth=true but ANTHROPIC_AUTH_TOKEN is not set". Recommendation: return `Ok(None)`; the existing `None` branch in `main.rs` already produces a helpful message.
**Warning signs:** Wrong header on the wire when the user thought they pinned bearer.

### Pitfall 4: Edition 2021 vs 2024 `unsafe { set_var }`
**What goes wrong:** A future edition bump to 2024 will break tests that call `std::env::set_var` without an `unsafe` block.
**Why it happens:** Rust 2024 makes `std::env::set_var` and `remove_var` `unsafe fn`. [CITED: doc.rust-lang.org/edition-guide/rust-2024/newly-unsafe-functions.html]
**How to avoid:** Workspace is currently on edition 2021 ([VERIFIED: `Cargo.toml:20`]) — no action required for Phase 3. When the workspace migrates to edition 2024, `cargo fix --edition` will auto-wrap the calls.
**Warning signs:** N/A this phase.

### Pitfall 5: Empty-string env vars treated as "set"
**What goes wrong:** `std::env::var("ANTHROPIC_API_KEY") == Ok("")` is technically a "set" var; naive `is_ok()` checks would trigger a false conflict error.
**Why it happens:** Shells like `export FOO=` set the var to empty.
**How to avoid:** The existing resolver already uses `.filter(|v| !v.is_empty())` (lines 1235, 1253, 1283) — copy that pattern in the new conflict checks. Treat empty strings as "not set" for both detection and resolution.
**Warning signs:** Phantom conflict errors in CI environments that pre-set blank credential vars.

## Code Examples

### ProviderConfig change (verified location)
```rust
// crates/core/src/lib.rs ~line 854 — current
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub models_whitelist: Vec<String>,
    #[serde(default)]
    pub models_blacklist: Vec<String>,
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
    // NEW (Phase 3):
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_bearer_auth: Option<bool>,
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
            use_bearer_auth: None, // NEW
        }
    }
}
```
[VERIFIED: read of `crates/core/src/lib.rs:854–885`]

### Integration test boilerplate
```rust
// crates/core/tests/bearer_auth.rs (NEW FILE)
//! Phase 3: ANTHROPIC_AUTH_TOKEN bearer auth resolution + conflict detection.

use claurst_core::config::{Config, ProviderConfig};
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

// D-09 case 1 — happy path bearer
#[tokio::test]
#[serial]
async fn auth_token_env_resolves_to_bearer() {
    reset_anthropic_env();
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", "btr-test-1");

    let cfg = Config::default(); // provider not set -> defaults still resolve anthropic
    let res = cfg.resolve_anthropic_auth_async().await.unwrap();

    assert_eq!(res, Some(("btr-test-1".to_string(), true)));
    reset_anthropic_env();
}

// D-09 case 2 — env conflict
#[tokio::test]
#[serial]
async fn both_env_vars_set_errors() {
    reset_anthropic_env();
    std::env::set_var("ANTHROPIC_API_KEY", "sk-...");
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", "btr-...");

    let cfg = Config::default();
    let err = cfg.resolve_anthropic_auth_async().await.unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("ANTHROPIC_API_KEY") && msg.contains("ANTHROPIC_AUTH_TOKEN"),
        "expected error to name both vars, got: {msg}");

    reset_anthropic_env();
}

// D-09 case 3 — pin + env api key
#[tokio::test]
#[serial]
async fn pin_bearer_with_env_api_key_errors() {
    reset_anthropic_env();
    std::env::set_var("ANTHROPIC_API_KEY", "sk-...");

    let cfg = anthropic_config_with(ProviderConfig {
        use_bearer_auth: Some(true),
        ..ProviderConfig::default()
    });
    let err = cfg.resolve_anthropic_auth_async().await.unwrap_err();
    assert!(err.to_string().contains("use_bearer_auth"));

    reset_anthropic_env();
}

// D-09 case 4 — pin + provider api_key in settings
#[tokio::test]
#[serial]
async fn pin_bearer_with_settings_api_key_errors() {
    reset_anthropic_env();

    let cfg = anthropic_config_with(ProviderConfig {
        api_key: Some("sk-from-settings".into()),
        use_bearer_auth: Some(true),
        ..ProviderConfig::default()
    });
    let err = cfg.resolve_anthropic_auth_async().await.unwrap_err();
    assert!(err.to_string().contains("use_bearer_auth"));

    reset_anthropic_env();
}

// D-09 case 5 — config.env injection feeds the resolver
#[tokio::test]
#[serial]
async fn config_env_injection_resolves_bearer() {
    reset_anthropic_env();

    // Simulate the main.rs injection loop directly to keep the test in claurst-core.
    let mut env: HashMap<String, String> = HashMap::new();
    env.insert("ANTHROPIC_AUTH_TOKEN".into(), "btr-from-settings".into());
    for (k, v) in &env {
        if std::env::var(k).is_err() {
            std::env::set_var(k, v);
        }
    }

    let cfg = Config::default();
    let res = cfg.resolve_anthropic_auth_async().await.unwrap();
    assert_eq!(res, Some(("btr-from-settings".to_string(), true)));

    reset_anthropic_env();
}
```
Notes:
- `reset_anthropic_env()` is called at the top **and bottom** of each test as belt-and-braces against panics in `assert!` partway through.
- The case-5 test deliberately reproduces the main.rs injection loop in-test rather than calling into `claurst-cli` (which depends on `claurst-core`, not the other way around). This keeps the integration test in the right crate.
- Test names use the existing `snake_case` descriptive convention. [CITED: `.planning/codebase/TESTING.md` — naming]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `std::env::set_var` is safe | `unsafe fn` in Rust 2024 edition | Rust 2024 (edition guide) | Workspace is edition 2021; no immediate change. [CITED: doc.rust-lang.org/edition-guide/rust-2024/newly-unsafe-functions.html] |
| Hand-rolled `Mutex` for serial tests | `serial_test` crate `#[serial]` attribute | crate stabilised pre-3.0, current 3.2.0 | Standard pattern; CONTEXT.md mandates it. |
| `Option<T>` resolver returning silent priority | `Result<Option<T>>` resolver with explicit conflicts | This phase | Removes silent precedence ambiguity between `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN`. |

**Deprecated/outdated:**
- The previous "pick first non-empty" priority assumption that lived implicitly in `resolve_anthropic_auth_async` (current line 1276–1284) is being explicitly retired by this phase per CONTEXT.md.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The `claurst_core::config` module is the public path for `Config` and `ProviderConfig` (used in test boilerplate as `claurst_core::config::Config`). The exact public re-export path may be `claurst_core::Config` directly or via a `config` module. | Code Examples / Integration test boilerplate | Test imports won't compile until corrected. Planner should grep `pub use` in `crates/core/src/lib.rs` to confirm and adjust the test imports accordingly. [ASSUMED] |
| A2 | `Config::default()` produces a config where `selected_provider_id() == "anthropic"`. The current resolver dispatches to `resolve_anthropic_auth_async()` only when the active provider is anthropic. | Code Examples (case 1 and 5 use `Config::default()`) | Tests may need to explicitly set `cfg.provider = Some("anthropic".into())` to be defensive — adopt this in case 1 and 5 even if the assumption holds, to make tests robust to future default changes. [ASSUMED] |

**If this table is short:** Most claims are verified against the actual files. The two `[ASSUMED]` items are mechanical and easy to confirm in the planner's first reading pass.

## Open Questions (RESOLVED)

1. **Should `Config::default()` set `provider` to `Some("anthropic")` for tests, or rely on `selected_provider_id()` defaulting?**
   - What we know: The wrapper `resolve_auth_async` checks `selected_provider_id() != "anthropic"` and returns early; the test directly calls `resolve_anthropic_auth_async()` which does not perform that check. Calling the inner resolver directly avoids the question.
   - What's unclear: Whether tests should exercise the wrapper (`resolve_auth_async`) or the inner function. Tests above use the inner function, which matches D-09's wording ("`resolve_anthropic_auth_async()` returns ...").
   - RESOLVED: Keep tests targeted at `resolve_anthropic_auth_async()` per D-09; this avoids the provider-id branching question entirely.

2. **Should `Config` add a top-level convenience accessor for `use_bearer_auth` (analogous to `resolve_anthropic_api_base()`)?**
   - What we know: `resolve_anthropic_api_base()` exists at ~line 1343. There is room for `resolve_anthropic_use_bearer_auth() -> Option<bool>` symmetry.
   - What's unclear: Not requested by CONTEXT.md.
   - RESOLVED: Defer. Field access via `provider_configs.get("anthropic").and_then(|p| p.use_bearer_auth)` is fine for the resolver and the only reader.

3. **Should the user-facing error message include a suggested fix line?**
   - What we know: The current `bail!` examples include unset/remove guidance.
   - What's unclear: Tone consistency with other startup errors in `main.rs`.
   - RESOLVED: Imitate the existing `No API key found ...` block in `main.rs:570–578` style — short instructions list. Plan can iterate on wording in code review.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | Build + test | ✓ | 1.95.0 | — |
| `rustc` | Compile | ✓ | 1.95.0 | — |
| `serial_test` crate | New dev-dep | ✓ (via crates.io) | 3.2.0 | — |
| Network access for crates.io | First-time `serial_test` fetch | ✓ (assumed; standard dev environment) | — | If offline: vendor via `cargo vendor` (deferred — not Phase 3 work) |
| Anthropic API endpoint | Runtime auth verification (smoke test, manual) | not exercised by Phase 3 tests | — | Tests are pure-resolver; no live HTTP |

**Missing dependencies with no fallback:** None.
**Missing dependencies with fallback:** None.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | `cargo test` (built-in harness) + `tokio::test` for async + `serial_test::serial` for env-var serialisation |
| Config file | None — Cargo.toml + integration test directory layout |
| Quick run command | `cargo test -p claurst-core --test bearer_auth` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| D-09.1 | `ANTHROPIC_AUTH_TOKEN` env -> bearer auth `Ok(Some((token,true)))` | integration | `cargo test -p claurst-core --test bearer_auth auth_token_env_resolves_to_bearer` | ❌ Wave 0 |
| D-09.2 | Both env vars set -> `Err` mentioning conflict | integration | `cargo test -p claurst-core --test bearer_auth both_env_vars_set_errors` | ❌ Wave 0 |
| D-09.3 | `use_bearer_auth=true` + `ANTHROPIC_API_KEY` env -> `Err` | integration | `cargo test -p claurst-core --test bearer_auth pin_bearer_with_env_api_key_errors` | ❌ Wave 0 |
| D-09.4 | `use_bearer_auth=true` + provider `api_key` -> `Err` | integration | `cargo test -p claurst-core --test bearer_auth pin_bearer_with_settings_api_key_errors` | ❌ Wave 0 |
| D-09.5 | `config.env` injection makes token visible -> bearer auth resolves | integration | `cargo test -p claurst-core --test bearer_auth config_env_injection_resolves_bearer` | ❌ Wave 0 |
| (regression) | `ANTHROPIC_API_KEY` alone still resolves to `(key, false)` | integration | `cargo test -p claurst-core --test bearer_auth api_key_only_resolves_to_x_api_key` (recommended add) | ❌ Wave 0 |
| (compile gate) | `resolve_anthropic_auth_async` signature change compiles in `main.rs` | implicit | `cargo check --workspace` | ✅ existing |

### Sampling Rate
- **Per task commit:** `cargo test -p claurst-core --test bearer_auth`
- **Per wave merge:** `cargo test -p claurst-core` (full core crate)
- **Phase gate:** `cargo test --workspace` green before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] Add `serial_test = "3"` to `crates/core/Cargo.toml` `[dev-dependencies]`
- [ ] Create `crates/core/tests/bearer_auth.rs` with the five D-09 tests + the recommended regression test
- [ ] Add `use_bearer_auth: Option<bool>` to `ProviderConfig` and its `Default` impl (compile-time prerequisite for tests using the field)

## Project Constraints (from CLAUDE.md)

No `./CLAUDE.md` file exists in the repo root. [VERIFIED: `ls /Users/thamw/development/local/clearest-rust/CLAUDE.md` -> not found.] No project-skills directories exist either (`.claude/skills/`, `.agents/skills/` both absent). Constraints come from `.planning/codebase/`:

- **Rust-only** (`STATE.md`): No new language runtimes. ✓ — Phase 3 is pure Rust.
- **No breaking schema changes without migration** (`STATE.md`): Adding `use_bearer_auth: Option<bool>` with `#[serde(default)]` is non-breaking; existing settings.json files deserialize unchanged. ✓
- **Workspace resolver v2** (`STATE.md`): Already in `Cargo.toml`. No change needed. ✓
- **Test conventions** (`.planning/codebase/TESTING.md`): Integration tests in `crates/<name>/tests/<file>.rs`; `snake_case` descriptive names; `use super::*` not applicable for integration tests; `unwrap()` and `expect()` acceptable in tests. ✓ matched in test boilerplate above.

## Sources

### Primary (HIGH confidence)
- `/Users/thamw/development/local/clearest-rust/crates/core/src/lib.rs:625–699, 854–886, 1240–1320` — verified `ProviderConfig`, `api_key_env_vars_for_provider`, `resolve_anthropic_auth_async`
- `/Users/thamw/development/local/clearest-rust/crates/cli/src/main.rs:505–598` — verified `config.env` injection loop and auth resolution call site
- `/Users/thamw/development/local/clearest-rust/crates/api/src/lib.rs:425–760` (greps confirm) — `use_bearer_auth` already wired through `send_with_retry` and `fetch_available_models`
- `/Users/thamw/development/local/clearest-rust/Cargo.toml` — workspace dependencies and edition 2021
- `/Users/thamw/development/local/clearest-rust/crates/core/Cargo.toml` — current dev-dependencies (only `tempfile`)
- `/Users/thamw/development/local/clearest-rust/.planning/codebase/TESTING.md` — testing conventions (no third-party runner; tokio::test for async; integration tests in `tests/`)
- `/Users/thamw/development/local/clearest-rust/.planning/phases/03-anthropic-auth-token-bearer-auth-support/03-CONTEXT.md` — locked decisions D-01 through D-09
- `/Users/thamw/development/local/clearest-rust/crates/core/src/import_config.rs:371–384, 743–754` — confirmed `ANTHROPIC_AUTH_TOKEN` already present in import allowlist + skip-reason map

### Secondary (MEDIUM confidence)
- [serial_test on crates.io (3.2.0)](https://crates.io/crates/serial_test) — version & MSRV
- [serial_test on docs.rs](https://docs.rs/serial_test/latest/serial_test/) — `#[serial]` attribute behaviour, file_serial variant
- [Rust 2024 edition guide — newly unsafe functions](https://doc.rust-lang.org/edition-guide/rust-2024/newly-unsafe-functions.html) — `set_var`/`remove_var` becoming `unsafe fn`

### Tertiary (LOW confidence)
- None.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — workspace deps inspected directly; `serial_test` version cross-checked against crates.io and docs.rs
- Architecture: HIGH — every line/symbol cited was read from source
- Pitfalls: HIGH for items 1–3 (locked by CONTEXT.md & code reading); MEDIUM for items 4–5 (citation + idiom check)
- Test boilerplate: MEDIUM — public path for `Config` (assumption A1) and default provider id (assumption A2) flagged

**Research date:** 2026-05-06
**Valid until:** 2026-06-05 (30 days — stack is stable, no fast-moving deps)
