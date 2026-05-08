---
status: diagnosed
phase: 03-anthropic-auth-token-bearer-auth-support
source: 03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md
started: 2026-05-08T00:00:00Z
updated: 2026-05-08T01:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Cold Start Smoke Test
expected: Kill any running claurst process. Clear any stale lock files. Run `cargo build --workspace` from the repo root — build exits 0. Then run `claurst` (or `cargo run -p claurst`) — binary starts without panics, prints normal startup output or awaits input.
result: pass

### 2. Bearer Token via ANTHROPIC_AUTH_TOKEN Env Var
expected: Unset ANTHROPIC_API_KEY. Set ANTHROPIC_AUTH_TOKEN=<any_token_value>. Run claurst. It connects to Anthropic using bearer auth — either succeeds (if token is valid) or returns an auth rejection error from Anthropic (not a local panic or "no credentials" error). If you have a proxy running, the outbound request shows `Authorization: Bearer <token>` (not `x-api-key`).
result: issue
reported: "With ANTHROPIC_AUTH_TOKEN set + ANTHROPIC_API_KEY=invalid: looks good. With ANTHROPIC_AUTH_TOKEN set + ANTHROPIC_API_KEY=<valid-key>: /status shows Auth: Not authenticated instead of a conflict error or authenticated state."
severity: major

### 3. Conflict Detection — Both Env Vars Set
expected: Set both ANTHROPIC_API_KEY=somekey and ANTHROPIC_AUTH_TOKEN=sometoken. Run claurst. It should exit with an explicit error message about conflicting credentials (e.g., mentioning both ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN cannot both be set). No crash, no silent fallback — just a clear error.
result: issue
reported: "ANTHROPIC_API_KEY=xyz ANTHROPIC_AUTH_TOKEN=sometoken ./target/debug/claurst — no errors at all, conflict detection not firing."
severity: blocker

### 4. Bearer Token via settings.json config.env
expected: Unset ANTHROPIC_AUTH_TOKEN from process env. Add `"config": {"env": {"ANTHROPIC_AUTH_TOKEN": "<token>"}}` to `~/.claurst/settings.json` (or the active settings file). Run claurst. It reads the token from settings.json and uses bearer auth — same behavior as Test 2 (succeeds or returns an Anthropic auth rejection, not a "no credentials" error).
result: issue
reported: "/status shows Auth: Not authenticated even with ANTHROPIC_AUTH_TOKEN set in settings.json config.env block."
severity: major

### 5. use_bearer_auth Pin in settings.json
expected: Add `"use_bearer_auth": true` to the Anthropic provider config in settings.json. Set ANTHROPIC_AUTH_TOKEN but NOT ANTHROPIC_API_KEY. Run claurst. It uses bearer mode — does NOT fall back to x-api-key. If you also try with no ANTHROPIC_AUTH_TOKEN set (while use_bearer_auth=true), claurst should return Ok(None)/no-credentials rather than silently falling through to look for an api key.
result: issue
reported: "/status shows Auth: Not authenticated even with use_bearer_auth: true and ANTHROPIC_AUTH_TOKEN set."
severity: major

### 6. Existing API Key Regression
expected: Unset ANTHROPIC_AUTH_TOKEN entirely. Set only ANTHROPIC_API_KEY=<your_key>. Run claurst normally. It works exactly as before this phase — no regressions, connects via x-api-key header. The bearer auth changes should be fully transparent when only ANTHROPIC_API_KEY is present.
result: issue
reported: "ERROR API request failed error=API error 503: Service temporarily unavailable — proxy returns 503 which means incorrect request format (wrong auth header). ANTHROPIC_AUTH_TOKEN should be sent as Authorization: Bearer, ANTHROPIC_API_KEY should be sent as x-api-key."
severity: blocker

## Summary

total: 6
passed: 1
issues: 5
pending: 0
skipped: 0
blocked: 0

## Gaps

- truth: "/status should reflect auth is configured when ANTHROPIC_AUTH_TOKEN is set (via env, config.env, or use_bearer_auth pin)"
  status: failed
  reason: "User reported: /status shows Auth: Not authenticated when ANTHROPIC_AUTH_TOKEN configured via env var, settings.json config.env, or use_bearer_auth: true — covers tests 2, 4, 5"
  severity: major
  test: 2
  root_cause: "/status handler (crates/commands/src/lib.rs:1268) calls sync resolve_api_key() which only checks ANTHROPIC_API_KEY via api_key_env_vars_for_provider() returning [\"ANTHROPIC_API_KEY\"]; ANTHROPIC_AUTH_TOKEN is never consulted in this path. resolve_anthropic_auth_async() is never called by /status."
  artifacts:
    - path: "crates/commands/src/lib.rs"
      issue: "StatusCommand::execute uses resolve_api_key() (~line 1268) instead of resolve_anthropic_auth_async()"
    - path: "crates/core/src/lib.rs"
      issue: "api_key_env_vars_for_provider(\"anthropic\") at line 627 returns only [\"ANTHROPIC_API_KEY\"] — ANTHROPIC_AUTH_TOKEN excluded from all sync auth paths"
    - path: "crates/cli/src/main.rs"
      issue: "auth_status() at ~line 3328 has the same gap — iterates api_key_env_vars_for_provider which excludes ANTHROPIC_AUTH_TOKEN"
  missing:
    - "Replace resolve_api_key() in /status handler with resolve_anthropic_auth_async().await"
    - "Display 'Authenticated (Bearer token)' when result is Some((_, true)), 'Authenticated (API key)' when Some((_, false))"
    - "Fix auth_status() in main.rs similarly"
  debug_session: ""

- truth: "ANTHROPIC_AUTH_TOKEN set in settings.json config.env should inject into process env before auth resolver runs, resulting in bearer auth being used"
  status: failed
  reason: "Same root cause as test 2 — /status never calls resolve_anthropic_auth_async() so injected token is invisible to it. Config.env injection itself works correctly."
  severity: major
  test: 4
  root_cause: "Same as test 2 — /status uses sync resolve_api_key(). Config.env injection (main.rs:518-522) is correct; ANTHROPIC_AUTH_TOKEN IS injected before resolver call. The display gap is in /status, not in injection."
  artifacts:
    - path: "crates/commands/src/lib.rs"
      issue: "/status auth display path doesn't call resolve_anthropic_auth_async()"
  missing:
    - "Same fix as test 2 gap"
  debug_session: ""

- truth: "use_bearer_auth: true with ANTHROPIC_AUTH_TOKEN set should force bearer mode — /status should reflect auth is configured"
  status: failed
  reason: "Same root cause as tests 2 and 4 — /status never calls resolve_anthropic_auth_async()"
  severity: major
  test: 5
  root_cause: "Same as test 2 — /status uses sync resolve_api_key(). The use_bearer_auth field IS read correctly by resolve_anthropic_auth_async(); the bug is purely in /status display."
  artifacts:
    - path: "crates/commands/src/lib.rs"
      issue: "/status auth display path doesn't call resolve_anthropic_auth_async()"
  missing:
    - "Same fix as test 2 gap"
  debug_session: ""

- truth: "Existing ANTHROPIC_API_KEY path must work unchanged — x-api-key header sent correctly"
  status: failed
  reason: "User reported: 503 from proxy meaning wrong auth header sent even for API key path — phase broke existing header-setting logic"
  severity: blocker
  test: 6
  root_cause: "Phase 03's config.env injection loop (main.rs:518-522) now injects ANTHROPIC_BASE_URL from settings.json into process env. User has ANTHROPIC_BASE_URL pointing to a bearer-only proxy in their settings.json config.env block. This proxy rejects x-api-key requests with 503. The header-setting logic in crates/api/src/lib.rs:753-757 is correct and unchanged. The regression is that the injection loop activates the proxy URL for all auth modes including API key."
  artifacts:
    - path: "crates/cli/src/main.rs"
      issue: "config.env injection loop (lines 518-522) propagates ANTHROPIC_BASE_URL from settings.json into process env, routing API key requests to bearer-only proxy"
    - path: "crates/core/src/lib.rs"
      issue: "resolve_provider_api_base() reads injected ANTHROPIC_BASE_URL from process env, changing endpoint"
  missing:
    - "Clarify whether ANTHROPIC_BASE_URL in config.env should be injected for all auth modes"
    - "Either exclude ANTHROPIC_BASE_URL from config.env injection, or document that it affects all auth modes and advise user to remove it from settings when testing API key path"
  debug_session: ""

- truth: "Setting both ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN should produce an explicit conflict error — no silent fallback"
  status: failed
  reason: "User reported: ANTHROPIC_API_KEY=xyz ANTHROPIC_AUTH_TOKEN=sometoken ./target/debug/claurst — no errors at all, conflict detection not firing"
  severity: blocker
  test: 3
  root_cause: "Debug agent confirmed conflict detection IS working in the current binary (tested against current build, exits with conflict error). UAT was likely conducted against a stale binary predating commit a6896a8 (May 7 15:53) which added the ? propagation at main.rs:566. Not a real bug in current code — needs re-verification."
  artifacts:
    - path: "crates/core/src/lib.rs"
      issue: "None — conflict detection at lines 1301-1307 is correct"
    - path: "crates/cli/src/main.rs"
      issue: "None — ? propagation at line 566 is correct"
  missing:
    - "Re-run test 3 with current binary (cargo build first) to confirm conflict detection fires"
  debug_session: ""
