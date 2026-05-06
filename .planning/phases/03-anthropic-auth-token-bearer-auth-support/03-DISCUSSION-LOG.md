# Phase 03: Discussion Log

**Phase:** 03-anthropic-auth-token-bearer-auth-support
**Date:** 2026-05-06
**Source:** UAT session — Phase 02 UAT blocked by auth failure

## Summary

Phase 03 was triggered by a UAT blocker: the user's proxy server (`http://epsilon.net.tham.one:53080`) requires `Authorization: Bearer` headers but Claurst always sends `x-api-key`. The user has `ANTHROPIC_AUTH_TOKEN` set in `~/.claurst/settings.json` config.env but Claurst never reads it as a credential.

## Key Findings from Debugging

### Finding 1: config.env is dead config
`config.env` in `settings.json` is deserialized and merged in the settings layer but never applied to `std::env`. Any env var set there (including `ANTHROPIC_AUTH_TOKEN`, `ANTHROPIC_BASE_URL`) has no effect at runtime.

**Fix:** Inject `config.env` into `std::env` at startup in `main.rs`, before auth resolution. Process env wins.

### Finding 2: ANTHROPIC_AUTH_TOKEN not recognized
`resolve_anthropic_api_key()` checks `ANTHROPIC_API_KEY` only. `resolve_anthropic_auth_async()` falls through to OAuth token loading if no API key is found — it never checks `ANTHROPIC_AUTH_TOKEN`.

`ANTHROPIC_AUTH_TOKEN` appears only in `import_config.rs` as a migration comment.

**Fix:** After `ANTHROPIC_API_KEY` check fails in `resolve_anthropic_auth_async`, check `ANTHROPIC_AUTH_TOKEN` and return `(token, use_bearer_auth=true)`.

### Finding 3: Bearer auth infrastructure already exists
`ClientConfig.use_bearer_auth` exists. Both `send_with_retry` and `fetch_available_models` already branch on it. The fix is purely upstream — get `use_bearer_auth=true` into `ClientConfig` when the token comes from `ANTHROPIC_AUTH_TOKEN`.

### Finding 4: Partial fix applied but may not be committed
During the UAT session, both fixes (config.env injection, ANTHROPIC_AUTH_TOKEN check) were coded directly into the source. Their commit status is uncertain — the planner must verify before re-applying.

### Finding 5: named-command paths have their own ClientConfig construction
`crates/commands/src/lib.rs` lines ~162 and ~1967 construct `ClientConfig` independently. They may not inherit `use_bearer_auth` from the main startup path. Deferred to a follow-up phase to keep this one focused.

## User Requirements (verbatim)
> "Create a new phase to fully support ANTHROPIC_AUTH_TOKEN at the provider level. There should be a switch logic to on when to use bearer token and when to use x-api-key. There should also be a user defined config in json to decide which to use too."

## Decisions Locked
- D-01: ANTHROPIC_AUTH_TOKEN → Bearer mode (locked)
- D-02: config.env injection at startup (locked)
- D-03: provider_configs.anthropic.use_bearer_auth JSON flag (locked)
- D-04: existing request-layer bearer branching is sufficient (locked)
- D-05: regression test required (locked)
