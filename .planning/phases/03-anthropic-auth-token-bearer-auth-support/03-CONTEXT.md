# Phase 03: ANTHROPIC_AUTH_TOKEN Bearer Auth Support - Context

**Gathered:** 2026-05-06
**Status:** Ready for planning

<domain>
## Phase Boundary

This phase makes `ANTHROPIC_AUTH_TOKEN` a first-class credential source throughout the Claurst auth stack. When `ANTHROPIC_AUTH_TOKEN` is set (via env var or `settings.json config.env`), Claurst must use `Authorization: Bearer <token>` instead of `x-api-key: <token>` on all outbound API requests.

Additionally: the `config.env` map in `settings.json` must be injected into the process environment at startup (currently in working tree but uncommitted), and a user-visible `use_bearer_auth: Option<bool>` flag must be exposed in `provider_configs.anthropic` in settings.json so users can force bearer auth without relying on env var naming conventions.

**Critical:** `ANTHROPIC_API_KEY` (x-api-key mode) and `ANTHROPIC_AUTH_TOKEN` (bearer mode) are mutually exclusive. The resolver must detect and error on conflicts.

Out of scope: OAuth flow changes, any provider other than Anthropic, UI/TUI changes, ACP/commands paths.

</domain>

<decisions>
## Implementation Decisions

### Credential Exclusivity (the core new constraint)
- **D-01:** `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN` are **mutually exclusive**. The auth resolver must error — not silently pick one — when both are active.
- **D-02:** A conflict exists under **any** of these conditions:
  1. Both `ANTHROPIC_API_KEY` env var and `ANTHROPIC_AUTH_TOKEN` env var are non-empty at the same time
  2. `use_bearer_auth: true` is set in `provider_configs.anthropic` AND `ANTHROPIC_API_KEY` env var is non-empty
  3. `use_bearer_auth: true` is set in `provider_configs.anthropic` AND `api_key` is set in `provider_configs.anthropic` (settings.json)
- **D-03:** Conflict detection lives inside `resolve_anthropic_auth_async()`. Change its return type from `Option<(String, bool)>` to `Result<Option<(String, bool)>, <error type>>` (use `anyhow::Error` or a dedicated `AuthConflictError`). The caller in `main.rs` already propagates errors via `?` — updating the call site is minimal.

### ProviderConfig Extension
- **D-04:** Add `use_bearer_auth: Option<bool>` to the `ProviderConfig` struct in `crates/core/src/lib.rs`. Default is `None` (no opinion). When `Some(true)`, the resolver treats this as if `ANTHROPIC_AUTH_TOKEN` is the intended credential path and applies conflict checks accordingly.

### config.env Injection
- **D-05:** The `config.env` injection loop (`for (key, value) in &config.env { if std::env::var(key).is_err() { std::env::set_var(key, value); } }`) is **already in the working tree** in `crates/cli/src/main.rs`. Fold it into the main plan — do not commit it separately.

### Uncommitted Changes Handling
- **D-06:** Two working-tree diffs exist but are uncommitted: (1) `ANTHROPIC_AUTH_TOKEN` env check in `resolve_anthropic_auth_async`, (2) `config.env` injection in `main.rs`. **Fold both into the main plan.** The plan will rewrite/extend the resolver anyway (conflict detection changes the signature); treat the existing diffs as a starting delta that the plan overwrites cleanly.

### Regression Tests
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Auth resolution
- `crates/core/src/lib.rs` — `resolve_anthropic_auth_async()` (~line 1275), `ProviderConfig` struct (~line 855), `resolve_anthropic_api_key()` (~line 1240)
- `crates/cli/src/main.rs` — config.env injection (~line 515–522), auth resolution and ClientConfig construction (~line 564–593)

### API request layer (already supports bearer — no changes needed here)
- `crates/api/src/lib.rs` — `AnthropicClient::send_with_retry()` and `fetch_available_models()` — both already branch on `use_bearer_auth`
- `crates/api/src/client.rs` — `ClientConfig` struct with `use_bearer_auth: bool` field

### Settings schema
- `~/.claurst/settings.json` — live config; `config.env` block, `config.provider_configs.anthropic` block

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `resolve_anthropic_api_key()`: Already checks `provider_configs.anthropic.api_key` (settings.json) + `ANTHROPIC_API_KEY` env var — the conflict check in D-02 condition 3 reads from the same source
- `ClientConfig.use_bearer_auth: bool`: Already wired into both request methods — no changes to the API crate needed

### Established Patterns
- Return `Result` with `anyhow::Error` for fallible async operations in `main.rs` — the new `Result` return type from the resolver is consistent with how all other startup errors propagate
- `#[serial]` tests for env var mutation: not yet used in this repo — `serial_test` is a new dev-dependency

### Integration Points
- `resolve_anthropic_auth_async()` → `main.rs` line ~566: call site needs to handle `Err` variant (forward via `?` or match for a user-friendly startup error message)
- `config.env` injection must happen **before** any auth resolution call so `ANTHROPIC_AUTH_TOKEN` in settings.json is visible to the resolver — current placement at ~line 515 is correct

### Working Tree Delta (do NOT re-apply blindly)
- `crates/core/src/lib.rs`: +6 lines adding `ANTHROPIC_AUTH_TOKEN` env check (no conflict logic yet — this is superseded by D-01/D-02/D-03)
- `crates/cli/src/main.rs`: +9 lines adding `config.env` injection loop (this is correct as-is, keep it)

</code_context>

<specifics>
## Specific Ideas

### Proxy server context
User's proxy (`http://epsilon.net.tham.one:53080`) requires `Authorization: Bearer <token>` and rejects `x-api-key`. The token is `btr-...` format stored in `~/.claurst/settings.json` under `config.env.ANTHROPIC_AUTH_TOKEN`.

### Mutual exclusivity is the new constraint vs. prior context
The prior CONTEXT.md treated priority as "ANTHROPIC_API_KEY wins, then fall through to ANTHROPIC_AUTH_TOKEN". The new requirement is stricter: **don't silently pick one — error when both are set**. The conflict check must fire before any priority logic.

</specifics>

<deferred>
## Deferred Ideas

- Support `ANTHROPIC_AUTH_TOKEN` in named-command and ACP paths (`crates/commands/src/lib.rs` ~line 162, 1967) — these construct their own `ClientConfig`; bearer support can be added in a follow-up phase
- Expose bearer mode toggle in the onboarding provider setup UI
- Add `ANTHROPIC_AUTH_TOKEN` to `api_key_env_vars_for_provider("anthropic")` return slice — changes semantics; requires an audit of all callers

</deferred>

---

*Phase: 03-anthropic-auth-token-bearer-auth-support*
*Context gathered: 2026-05-06*
