# Phase 03: ANTHROPIC_AUTH_TOKEN Bearer Auth Support - Context

**Gathered:** 2026-05-06
**Status:** Ready for planning
**Source:** UAT session + live debugging

<domain>
## Phase Boundary

This phase makes `ANTHROPIC_AUTH_TOKEN` a first-class credential source throughout the Claurst auth stack. When `ANTHROPIC_AUTH_TOKEN` is set (via env var or `settings.json config.env`), Claurst must use `Authorization: Bearer <token>` instead of `x-api-key: <token>` on all outbound API requests.

Additionally: the `config.env` map in `settings.json` must be injected into the process environment at startup (currently stored but never applied), and a user-visible `use_bearer_auth` flag must be exposed in the `provider_configs.anthropic` JSON block so users can force bearer auth without relying on env var naming conventions.

Out of scope: OAuth flow changes, any provider other than Anthropic, UI/TUI changes.

</domain>

<decisions>
## Implementation Decisions

### D-01: ANTHROPIC_AUTH_TOKEN as Bearer-mode credential
`ANTHROPIC_AUTH_TOKEN` env var is a Claurst-specific alias that maps to Bearer auth.
When present and non-empty, `resolve_anthropic_auth_async()` returns `(token, use_bearer_auth=true)`.
It is checked AFTER `ANTHROPIC_API_KEY` (x-api-key mode) so ANTHROPIC_API_KEY still wins.

### D-02: config.env injection at startup
The `config.env` HashMap in `~/.claurst/settings.json` must be injected into `std::env` at startup in `crates/cli/src/main.rs`, before any auth resolution calls.
Semantics: process env wins — only `set_var` when `std::env::var(key).is_err()`.
This makes `ANTHROPIC_AUTH_TOKEN` and `ANTHROPIC_BASE_URL` in `settings.json` config.env behave identically to real env vars.

### D-03: provider_configs.anthropic.use_bearer_auth JSON flag
Add `use_bearer_auth: Option<bool>` to the Anthropic provider config struct in `crates/core/src/lib.rs`.
When set to `true` in `~/.claurst/settings.json`, forces Bearer auth mode regardless of which credential env var was set.
Priority: `use_bearer_auth: true` in config > ANTHROPIC_AUTH_TOKEN env var > ANTHROPIC_API_KEY (x-api-key).

### D-04: All request paths must respect use_bearer_auth
Both `AnthropicClient::send_with_retry` (messages endpoint) and `AnthropicClient::fetch_available_models` already branch on `self.config.use_bearer_auth` — no changes needed there.
The fix is upstream: ensure `use_bearer_auth=true` is passed into `ClientConfig` when ANTHROPIC_AUTH_TOKEN or config flag is active.

### D-05: Regression test coverage
Add a unit test in `crates/core/src/lib.rs` (or `crates/core/tests/`) that:
- Sets `ANTHROPIC_AUTH_TOKEN` in env, clears `ANTHROPIC_API_KEY`
- Calls `resolve_anthropic_auth_async()` 
- Asserts `use_bearer_auth == true` and token value matches

### Claude's Discretion
- Exact location of config.env injection (before vs after CLI arg parsing) — keep before auth resolution
- Whether to add `ANTHROPIC_AUTH_TOKEN` to the import_config migration allowlist
- How to document the new setting in any help text or provider setup UI

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Auth resolution
- `crates/core/src/lib.rs` — `resolve_anthropic_auth_async()` (~line 1275), `resolve_anthropic_api_key()` (~line 1240), `api_key_env_vars_for_provider()` (~line 625)
- `crates/cli/src/main.rs` — startup auth resolution (~line 556–595), config.env injection (added in earlier session, ~line 516–525), `resolve_bridge_config()` (~line 315)

### API request layer
- `crates/api/src/lib.rs` — `AnthropicClient::send_with_retry()` (~line 720–760), `fetch_available_models()` (~line 700–720) — both already branch on `use_bearer_auth`
- `crates/api/src/client.rs` — `ClientConfig` struct with `use_bearer_auth: bool` field

### Provider config
- `crates/core/src/lib.rs` — `ProviderConfig` struct (~line 857), `provider_configs` field in `Config`

### Settings file
- `~/.claurst/settings.json` — live config; `config.env` block, `config.provider_configs` block

</canonical_refs>

<specifics>
## Specific Ideas

### Current state (from live debugging, 2026-05-06)
- `ANTHROPIC_AUTH_TOKEN` env var is referenced only in `import_config.rs` (migration note) — never read as auth credential
- `resolve_anthropic_api_key()` checks only `ANTHROPIC_API_KEY` — no bearer path
- `config.env` is merged in settings layer but never `set_var`'d into the process
- `use_bearer_auth` flag exists in `ClientConfig` and both request methods but is only ever set to `true` via OAuth flow or the `minimax` provider hardcode

### Partial fix already applied (DO NOT DUPLICATE)
Two changes were made in the UAT session but may not have been committed:
1. `crates/core/src/lib.rs`: ANTHROPIC_AUTH_TOKEN check added in `resolve_anthropic_auth_async` — verify this commit exists before re-applying
2. `crates/cli/src/main.rs`: config.env injection loop added before `--dump-system-prompt` fast path — verify this commit exists before re-applying

Check: `git log --oneline | grep -i "auth_token\|bearer\|env.*inject"` to confirm what's already committed.

### Proxy server requirement
User's proxy (`http://epsilon.net.tham.one:53080`) requires `Authorization: Bearer <token>` header — rejects `x-api-key`. The token is `btr-...` format (stored in `~/.claurst/settings.json` config.env.ANTHROPIC_AUTH_TOKEN).

</specifics>

<deferred>
## Deferred Ideas

- Support `ANTHROPIC_AUTH_TOKEN` in named-command and ACP paths (crates/commands/src/lib.rs ~line 162, 1967) — these currently construct their own `ClientConfig`; bearer support can be added in a follow-up
- Expose bearer mode toggle in the onboarding provider setup UI
- Add `ANTHROPIC_AUTH_TOKEN` to `api_key_env_vars_for_provider("anthropic")` return slice (would auto-populate more paths but changes semantics — requires audit)

</deferred>

---

*Phase: 03-anthropic-auth-token-bearer-auth-support*
*Context gathered: 2026-05-06 from UAT debugging session*
