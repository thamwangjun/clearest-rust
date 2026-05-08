---
status: complete
phase: 03-anthropic-auth-token-bearer-auth-support
source: 03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md
started: 2026-05-08T00:00:00Z
updated: 2026-05-08T00:00:00Z
---

## Current Test

<!-- OVERWRITE each test - shows where we are -->

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

- truth: "With ANTHROPIC_AUTH_TOKEN set alongside ANTHROPIC_API_KEY, claurst should show a conflict error or correctly reflect bearer auth state"
  status: failed
  reason: "User reported: /status shows Auth: Not authenticated when both vars are set with a valid API key; conflict detection not surfacing a clear error"
  severity: major
  test: 2
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "ANTHROPIC_AUTH_TOKEN set in settings.json config.env should inject into process env before auth resolver runs, resulting in bearer auth being used"
  status: failed
  reason: "User reported: /status shows Auth: Not authenticated even with ANTHROPIC_AUTH_TOKEN set in settings.json config.env block"
  severity: major
  test: 4
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "use_bearer_auth: true with ANTHROPIC_AUTH_TOKEN set should force bearer mode — /status should reflect auth is configured"
  status: failed
  reason: "User reported: /status shows Auth: Not authenticated even with use_bearer_auth: true and ANTHROPIC_AUTH_TOKEN set"
  severity: major
  test: 5
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Existing ANTHROPIC_API_KEY path must work unchanged — x-api-key header sent correctly"
  status: failed
  reason: "User reported: 503 from proxy meaning wrong auth header sent even for API key path — phase broke existing header-setting logic"
  severity: blocker
  test: 6
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""

- truth: "Setting both ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN should produce an explicit conflict error — no silent fallback"
  status: failed
  reason: "User reported: ANTHROPIC_API_KEY=xyz ANTHROPIC_AUTH_TOKEN=sometoken ./target/debug/claurst — no errors at all, conflict detection not firing"
  severity: blocker
  test: 3
  root_cause: ""
  artifacts: []
  missing: []
  debug_session: ""
