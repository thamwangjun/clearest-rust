# Phase 03: ANTHROPIC_AUTH_TOKEN Bearer Auth Support - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-06
**Phase:** 03-anthropic-auth-token-bearer-auth-support
**Areas discussed:** use_bearer_auth flag semantics, Uncommitted changes strategy, Test coverage scope

---

## use_bearer_auth flag semantics

### When use_bearer_auth: true + ANTHROPIC_API_KEY both present

| Option | Description | Selected |
|--------|-------------|----------|
| Force Bearer with ANTHROPIC_API_KEY | Use API key as Bearer token — flag changes header format, not token source | |
| Require ANTHROPIC_AUTH_TOKEN | use_bearer_auth: true means the user wants the AUTH_TOKEN path; API_KEY is still for x-api-key mode | |
| (Free-text) | Mutual exclusivity — throw an error when both are set | ✓ |

**User's choice:** There must be no conflict. API_KEY and AUTH_TOKEN are mutually exclusive. Throw an error when there is both, with a message highlighting this.
**Notes:** User clarified this applies to all conflict combinations, not just the env-var pair.

### What exactly triggers the conflict error

| Option | Description | Selected |
|--------|-------------|----------|
| Both env vars set simultaneously | Error when ANTHROPIC_API_KEY and ANTHROPIC_AUTH_TOKEN are both non-empty in env | |
| use_bearer_auth: true + ANTHROPIC_API_KEY set | Error only when config flag is explicitly true while API_KEY is also present | |
| Either combination | Error on: (1) both env vars, (2) use_bearer_auth:true + API_KEY env, (3) use_bearer_auth:true + api_key in config | ✓ |

**User's choice:** Either combination, and also api_key is set in settings.json when use_bearer_auth: true. The error is thrown when both conditions (api key and auth token) in all scenarios are met.

### Where should conflict error be caught

| Option | Description | Selected |
|--------|-------------|----------|
| In resolve_anthropic_auth_async | Return Result from the resolver; caller in main.rs handles via ? | ✓ |
| In main.rs before calling the resolver | Precheck in main.rs; resolver stays simple with no error path | |

**User's choice:** In resolve_anthropic_auth_async (Recommended)

---

## Uncommitted changes strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Fold into the main plan | Plan modifies these files anyway; treat working tree diffs as starting delta | ✓ |
| Commit as-is first, then extend | Stage baseline commit, then second commit adds conflict check and ProviderConfig field | |
| Discard and rewrite cleanly | git restore both files, implement everything fresh | |

**User's choice:** Fold into the main plan (Recommended)
**Notes:** The ANTHROPIC_AUTH_TOKEN env check in lib.rs will be superseded by the conflict-aware version anyway.

---

## Test coverage scope

### What to test

| Option | Description | Selected |
|--------|-------------|----------|
| ANTHROPIC_AUTH_TOKEN env path | Happy path: set AUTH_TOKEN, clear API_KEY, assert bearer=true | ✓ |
| Conflict detection error paths | Three conflict scenarios each returning Err | ✓ |
| config.env injection | Config env block makes AUTH_TOKEN visible to resolver | ✓ |

**User's choice:** All three selected.

### Where tests live

| Option | Description | Selected |
|--------|-------------|----------|
| Inline in crates/core/src/lib.rs | #[cfg(test)] module at bottom of file | |
| Separate crates/core/tests/ file | Integration test file | ✓ |
| You decide | Claude picks based on existing patterns | |

**User's choice:** Separate crates/core/tests/ file

### Env var isolation strategy

| Option | Description | Selected |
|--------|-------------|----------|
| serial_test crate | Add serial_test as dev-dependency; #[serial] on env-mutating tests | ✓ |
| No isolation | Accept risk of parallel interference | |
| You decide | Claude picks safest approach | |

**User's choice:** Reset env vars before each test and add serial_test.

---

## Claude's Discretion

- Exact error type (anyhow::Error with context string vs. dedicated AuthConflictError enum variant)
- Error message wording
- Whether to add ANTHROPIC_AUTH_TOKEN to the import_config migration allowlist

## Deferred Ideas

- Bearer support in named-command and ACP paths (crates/commands/src/lib.rs)
- Bearer mode toggle in onboarding provider setup UI
- Adding ANTHROPIC_AUTH_TOKEN to api_key_env_vars_for_provider return slice
