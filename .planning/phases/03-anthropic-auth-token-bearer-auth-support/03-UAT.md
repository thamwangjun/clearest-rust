---
status: diagnosed
phase: 03-anthropic-auth-token-bearer-auth-support
source: 03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md
started: 2026-05-08T00:00:00Z
updated: 2026-05-08T09:30:00Z
retest: true
retest_reason: Gap closure (03-04) applied — re-verifying 5 previously failing tests
---

## Current Test

[testing complete]

## Tests

### 1. Cold Start Smoke Test
expected: Kill any running claurst process. Clear any stale lock files. Run `cargo build --workspace` from the repo root — build exits 0. Then run `claurst` (or `cargo run -p claurst`) — binary starts without panics, prints normal startup output or awaits input.
result: pass

### 2. Bearer Token via ANTHROPIC_AUTH_TOKEN Env Var
expected: Unset ANTHROPIC_API_KEY. Set ANTHROPIC_AUTH_TOKEN=<any_token_value>. Run claurst auth status. Expected: prints "Logged in." with "API key: ANTHROPIC_AUTH_TOKEN" in the details. Then start the REPL (same env) and run /status — Auth line should show "Authenticated (Bearer token)".
result: pass

### 3. Conflict Detection — Both Env Vars Set
expected: Set both ANTHROPIC_API_KEY=somekey and ANTHROPIC_AUTH_TOKEN=sometoken. Run claurst. It should exit with an explicit error message about conflicting credentials (e.g., mentioning both ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN cannot both be set). No crash, no silent fallback — just a clear error.
result: pass

### 4. Bearer Token via settings.json config.env
expected: Unset ANTHROPIC_AUTH_TOKEN from process env. Add `"config": {"env": {"ANTHROPIC_AUTH_TOKEN": "<token>"}}` to `~/.claurst/settings.json`. Run claurst auth status. Expected: prints "Logged in." with "API key: ANTHROPIC_AUTH_TOKEN" (token sourced from settings, not env).
result: issue
reported: "Not logged in for Anthropic. Run `claude auth login`, set ANTHROPIC_API_KEY, or set ANTHROPIC_AUTH_TOKEN. — config.env injection not active for auth status subcommand path"
severity: major

### 5. use_bearer_auth Pin in settings.json
expected: Add `"use_bearer_auth": true` to the Anthropic provider config in settings.json. Set ANTHROPIC_AUTH_TOKEN but NOT ANTHROPIC_API_KEY. Run claurst auth status. Expected: "Logged in." with source "ANTHROPIC_AUTH_TOKEN". Then run /status in REPL — Auth: Authenticated (Bearer token).
result: pass

### 6. Existing API Key Regression
expected: Unset ANTHROPIC_AUTH_TOKEN entirely. Set only ANTHROPIC_API_KEY=<your_key>. Run claurst normally. It works exactly as before — no 503 errors, correct x-api-key header sent. Bearer auth changes should be fully transparent when only ANTHROPIC_API_KEY is present.
result: pass

## Summary (Round 1 — pre 03-05/03-06 fixes)

total: 6
passed: 5
issues: 1
pending: 0
skipped: 0
blocked: 0

---

## Round 2 — Post 03-06 Fix Verification (2026-05-09)

Tests run against rebuilt binary after commit 3a9f8d0.

### Scenario 1 — ANTHROPIC_AUTH_TOKEN via env → Logged in
result: PASS

### Scenario 2 — ANTHROPIC_API_KEY + ANTHROPIC_AUTH_TOKEN → conflict error
result: FAIL (pre-fix) → FIXED
fix: Added explicit conflict check in auth_status() before detect_api_key_env_source() call; exits with error naming both vars when both are set.
commit: 3a9f8d0

### Scenario 3 — use_bearer_auth in settings.json → parse error
result: FAIL (pre-fix) → FIXED
fix: Removed use_bearer_auth field from ProviderConfig; added #[serde(deny_unknown_fields)] so settings.json with use_bearer_auth causes hard parse error.
commit: 3a9f8d0

### Scenario 4 — ANTHROPIC_API_KEY alone → Logged in
result: PASS

## Summary (Round 2)

total: 4
passed: 4
issues: 0
pending: 0

## Gaps

- truth: "ANTHROPIC_AUTH_TOKEN set in settings.json config.env should result in 'Logged in.' from claurst auth status"
  status: failed
  reason: "User reported: Not logged in for Anthropic — config.env injection not reached before auth status check"
  severity: major
  test: 4
  root_cause: "auth status dispatches via a fast-path at main.rs:354-355 (raw arg check) that returns before the config.env injection loop at lines 517-523 ever runs. auth_status() loads Settings but contains no injection logic — so ANTHROPIC_AUTH_TOKEN from config.env is never written to the process env before detect_api_key_env_source() reads it."
  artifacts:
    - path: "crates/cli/src/main.rs"
      issue: "Fast-path at lines 354-355 dispatches handle_auth_command() before config.env injection at lines 517-523 runs"
    - path: "crates/cli/src/main.rs"
      issue: "auth_status() at line 3316 loads Settings but never calls the injection loop"
  missing:
    - "Move config.env injection into auth_status() (or a shared helper) so it runs before detect_api_key_env_source() regardless of entry point"
  debug_session: ""
