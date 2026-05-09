---
status: complete
phase: 03-anthropic-auth-token-bearer-auth-support
source: 03-01-SUMMARY.md, 03-02-SUMMARY.md, 03-03-SUMMARY.md, 03-04-SUMMARY.md, 03-05-SUMMARY.md, 03-06-SUMMARY.md
started: 2026-05-09T00:00:00Z
updated: 2026-05-09T12:00:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Cold Start Smoke Test
expected: Kill any running claurst process. Run `cargo build --workspace` from the repo root — exits 0. Then run the binary (`./target/debug/claurst --help` or `cargo run -p claurst -- --help`) — starts without panics or errors.
result: pass

### 2. ANTHROPIC_AUTH_TOKEN via env → auth status
expected: Unset ANTHROPIC_API_KEY. Set ANTHROPIC_AUTH_TOKEN=anyvalue. Run `claurst auth status`. Prints "Logged in." with "API key: ANTHROPIC_AUTH_TOKEN" in the details. No error, no crash.
result: pass

### 3. ANTHROPIC_AUTH_TOKEN via env → REPL /status
expected: With ANTHROPIC_AUTH_TOKEN set (and ANTHROPIC_API_KEY unset), start the REPL (`claurst`). Run `/status`. The Auth line shows "Authenticated (Bearer token)".
result: pass

### 4. Conflict detection — both env vars set
expected: Set both ANTHROPIC_API_KEY=somekey and ANTHROPIC_AUTH_TOKEN=sometoken. Run `claurst auth status`. Binary exits with an explicit error message naming both ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN as conflicting. No silent fallback, no crash without message.
result: pass

### 5. config.env injection — ANTHROPIC_AUTH_TOKEN in settings.json
expected: Unset ANTHROPIC_AUTH_TOKEN from shell. Add `"config": {"env": {"ANTHROPIC_AUTH_TOKEN": "<token>"}}` to `~/.claurst/settings.json`. Run `claurst auth status`. Prints "Logged in." with "API key: ANTHROPIC_AUTH_TOKEN" (token sourced from settings file, not shell env).
result: pass

### 6. deny_unknown_fields — use_bearer_auth in settings.json
expected: Add `"use_bearer_auth": true` to the Anthropic provider block in `~/.claurst/settings.json`. Run `claurst` or `claurst auth status`. Binary exits with a hard serde parse error about an unknown field — it does NOT silently ignore the unknown key.
result: pass

### 7. Existing API key regression
expected: Unset ANTHROPIC_AUTH_TOKEN entirely. Set only ANTHROPIC_API_KEY=<your_key>. Run `claurst auth status`. Shows "Logged in." as before. No 503 errors, no behaviour change from before phase 03.
result: pass

## Summary

total: 7
passed: 7
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
