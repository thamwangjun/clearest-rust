# Technical Concerns

**Analysis Date:** 2026-05-04

---

## High Priority Issues

### Credentials Stored in Plaintext on Disk

**Risk:** API keys and OAuth tokens (including `access`, `refresh` tokens) are written to `~/.claurst/auth.json` as plain JSON with no encryption and no restricted file permissions.
**Files:** `crates/core/src/auth_store.rs:52-60`
**Detail:** The `save()` method calls `std::fs::write(&path, json)` with default umask permissions. There is no `set_permissions(..., 0o600)` call. Any process running as the same user can read the credentials file.
**Fix approach:** Apply `std::os::unix::fs::PermissionsExt::set_mode(0o600)` after writing. Consider OS keychain (e.g., `keyring` crate) for long-term improvement.

### Unsafe `std::env::set_var` in Multi-Threaded Context

**Risk:** `std::env::set_var` / `std::env::remove_var` are called in production code paths that run inside an async Tokio runtime, which spawns multiple threads. This is unsound in Rust 1.81+ (the standard library now documents this as undefined behavior in multi-threaded programs).
**Files:**
- `crates/query/src/coordinator.rs:205-215` — coordinator-mode toggling at session resume
- `crates/core/src/lib.rs:3821-3858` — `ANTHROPIC_API_KEY` mutation in test helpers that run in parallel with `#[tokio::test]`
- `crates/core/src/import_config.rs:887-901` — `HOME` mutation in tests
- `crates/core/src/voice.rs:618-652` — kill-switch env var set/remove in voice tests
- `crates/mcp/src/lib.rs:1416-1439` — env var mutations in multiple MCP tests
**Fix approach:** Replace env var mutation with thread-local or `Arc<Mutex<Config>>` passing. For tests, use `serial_test` crate or `std::env` mutation only before any threads are spawned.

### Web Fetch: No Response Body Size Limit Before Buffering

**Risk:** `crates/tools/src/web_fetch.rs:351` calls `resp.text().await` with no byte limit before reading into memory. A malicious or large HTTP response (e.g., multi-GB download) will be fully buffered before the 100 KB text truncation at line 379 applies. This can exhaust heap memory.
**Files:** `crates/tools/src/web_fetch.rs:351-386`
**Fix approach:** Use `resp.bytes_limited(MAX_BYTES)` or stream with `reqwest::Response::chunk()` loop with a running total check before calling `.text()`.

### Panics in Production Match Arms

**Risk:** Multiple `match` arms in production code call `panic!()` for cases that are structurally expected to be unreachable but are not proven so by the type system. If an unexpected state occurs (e.g., a race, a serialization bug, or future refactor), these will crash the process.
**Files:**
- `crates/core/src/lib.rs:2628, 2654, 2708, 2738, 4142` — `panic!("Expected Ask, got {:?}", other)` in 5 permission manager tests that run in the same binary as integration code
- `crates/tui/src/elicitation_dialog.rs:641, 762` — `panic!("expected Submitted result")` and `panic!("expected array")`
- `crates/commands/src/named_commands.rs:1178, 1238` — `panic!("Expected Message")` and `panic!("Unexpected result")`
- `crates/query/src/lib.rs:2214, 2234, 2256, 2280, 2292` — `panic!("Expected SystemPrompt::Text")`
- `crates/api/src/providers/message_normalization.rs:112, 135, 138` — panics on unexpected message structure
**Fix approach:** Return `Err(...)` or a sentinel value instead of panicking. Use `debug_assert!` only for invariants proven impossible by construction.

---

## Technical Debt

### Monolithic Files Exceeding 4,000 Lines

**Issue:** Several files have grown to a size that makes them hard to reason about, review, or test in isolation.
**Files:**
- `crates/commands/src/lib.rs` — **8,576 lines** (all command handling in a single file)
- `crates/tui/src/app.rs` — **5,918 lines** (entire TUI application state machine)
- `crates/core/src/lib.rs` — **4,246 lines** (core types + permission manager + tests mixed)
- `crates/tui/src/prompt_input.rs` — **3,719 lines** (input widget + vim emulation)
- `crates/cli/src/main.rs` — **3,502 lines** (CLI argument parsing + all dispatch logic)
**Impact:** Changes to any of these files risk merge conflicts, accidental breakage of unrelated features, and poor test isolation.
**Fix approach:** Extract sub-modules. For example, `commands/src/lib.rs` should be split into per-command files under `commands/src/commands/`.

### Pervasive `unwrap()` in Non-Test Code

**Issue:** Approximately 410 `.unwrap()` calls exist outside test modules. While many are on `Mutex::lock()` (which poisons on panic, compounding failures) or `serde_json::to_string` (which should never fail for serializable types), a significant number are on fallible operations where errors are plausible.
**High-risk examples:**
- `crates/api/src/codex_adapter.rs:188` — `openai_req["messages"].as_array().unwrap()` (panics if upstream changes response shape)
- `crates/api/src/providers/codex.rs:96, 196, 210, 239, 1035` — `self.tokens.lock().unwrap()` (poisoned mutex → panic)
- `crates/bridge/src/lib.rs:1670-1710` — multiple `unwrap()` on serialization in test helpers that run alongside production code
**Fix approach:** Replace with `?` propagation or `expect()` with descriptive messages. Use `parking_lot::Mutex` (which never poisons) in place of `std::sync::Mutex` for lock sites.

### Mixed `std::sync::Mutex` and `parking_lot::Mutex` Usage

**Issue:** Both mutex implementations are used across the codebase. `std::sync::Mutex` can be poisoned (lock returns `Err` after a panic in a lock holder), whereas `parking_lot::Mutex` never poisons. The inconsistency makes reasoning about panic propagation harder.
**Files using `std::sync::Mutex`:** `crates/core/src/output_styles.rs:17`, `crates/core/src/prompt_history.rs:22`, `crates/api/src/providers/codex.rs`, `crates/tools/src/lib.rs:261`, `crates/cli/src/main.rs:596`
**Fix approach:** Standardize on `parking_lot::Mutex` across the codebase (already a workspace dependency).

### Hardcoded Model Snapshot

**Issue:** `crates/api/src/model_registry.rs` maintains a hand-coded list of models (Anthropic, OpenAI, Google, DeepSeek, Zai) with prices, context windows, and capabilities. This snapshot will go stale as providers release new models or change pricing.
**Files:** `crates/api/src/model_registry.rs:84-118`
**Impact:** Users cannot access newly released models without a code change and release. The Copilot provider already falls back to a hardcoded list when the API is unavailable (`crates/api/src/providers/copilot.rs:1200-1206`).
**Fix approach:** Add a periodic refresh that fetches a JSON manifest from a hosted URL, falling back to the bundled snapshot only on failure.

### Dead Code Suppression (`#[allow(dead_code)]`) Widespread

**Issue:** 20+ instances of `#[allow(dead_code)]` exist across the codebase, indicating structs/fields/variants that are defined but not currently used. This is often a sign of incomplete feature work or leftover code from refactors.
**Files:** `crates/core/src/settings_sync.rs` (9 occurrences), `crates/tui/src/app.rs:134, 183, 4706`, `crates/commands/src/lib.rs:4938, 6610`, `crates/tools/src/computer_use.rs:49-53`, `crates/tools/src/web_fetch.rs:18`, `crates/cli/src/oauth_flow.rs:58, 362`
**Fix approach:** Remove unused code or, if intentionally future-facing, add a `// used by feature X` comment.

### Large Number of Feature Flags (36 in `dev_full`)

**Issue:** `crates/core/Cargo.toml` defines 36 named feature flags controlling everything from UI widgets to memory systems. Most features are empty (no code behind them) or sparsely gated. This creates a combinatorial testing problem — the CI likely tests only a small subset of combinations.
**Files:** `crates/core/Cargo.toml:6-71`
**Impact:** A feature enabled in production but not tested in CI can break silently.
**Fix approach:** Audit which features are actually shipped in production builds. Consider replacing boolean feature flags with runtime configuration where possible.

---

## Missing Pieces

### No Tests for Critical Provider Integrations

**Issue:** The following provider files have zero test coverage:
- `crates/api/src/providers/copilot.rs` (1,247 lines, GitHub Copilot integration)
- `crates/api/src/providers/azure.rs` (Azure OpenAI)
- `crates/api/src/providers/bedrock.rs` (AWS Bedrock + SigV4 signing)
- `crates/api/src/providers/cohere.rs`
- `crates/api/src/providers/minimax.rs`
- `crates/api/src/providers/anthropic.rs`
- `crates/api/src/transformers/openai_chat.rs`, `crates/api/src/transformers/anthropic.rs`
**Impact:** Any regression in request formatting or response parsing is undetected until a user reports it.
**Fix approach:** Add unit tests using mock HTTP responses (e.g., `wiremock` or `httpmock` crate) for at least the happy path and common error cases.

### No Tests for CLI Entry Point

**Issue:** `crates/cli/src/main.rs` (3,502 lines) has zero test coverage. All argument parsing, dispatch logic, and session management are untested.
**Files:** `crates/cli/src/main.rs`
**Fix approach:** Extract dispatch logic into testable functions. Add integration tests using `assert_cmd` or `trycmd`.

### No Tests for Key Tool Implementations

**Issue:** The following tool files have no tests:
- `crates/tools/src/file_edit.rs` — file editing (highest risk for data loss)
- `crates/tools/src/web_fetch.rs`
- `crates/tools/src/pty_bash.rs`
- `crates/tools/src/send_message.rs`
- `crates/tools/src/ask_user.rs`
- `crates/tools/src/cron.rs`
- `crates/tools/src/config_tool.rs`
**Impact:** `file_edit.rs` in particular is high-risk — a bug in edit application logic causes irreversible file corruption.

### ACP Crate Untested

**Issue:** `crates/acp/src/lib.rs` (285 lines) implements the Agent Client Protocol server with no `#[test]` coverage. The JSON-RPC dispatch and session listing logic are untested.
**Files:** `crates/acp/src/lib.rs`

### `cron_scheduler` Has No Error Recovery for Failed Tasks

**Issue:** `crates/query/src/cron_scheduler.rs` spawns sub-agent tasks for due cron entries but does not track failure counts or disable runaway tasks. A perpetually failing cron task will fire every minute indefinitely, potentially racking up API costs.
**Files:** `crates/query/src/cron_scheduler.rs:37-121`
**Fix approach:** Track consecutive failures per task; disable after N failures with a user-visible warning.

---

## Security Considerations

### Credentials File Lacks Restricted Permissions

See "High Priority Issues" — `crates/core/src/auth_store.rs:52-60`. The `auth.json` file is written world-readable (subject to umask) on POSIX systems.

### SQLite `search_sessions` Constructs LIKE Pattern via `format!`

**Risk:** `crates/core/src/sqlite_storage.rs:125` builds a LIKE pattern with `format!("%{}%", query)` where `query` comes from user input. While the pattern is passed as a parameter (not interpolated into SQL), `%` and `_` wildcards inside `query` are not escaped. A user searching for `%` will match all sessions — an unintended behavior.
**Files:** `crates/core/src/sqlite_storage.rs:125`
**Fix approach:** Escape `%`, `_`, and `\` in the query string before wrapping it in `%`.

### Plugin Hook Execution Uses Shell (`sh -c`) with Unsanitized Input

**Issue:** `crates/plugins/src/hooks.rs:166, 300` spawns plugin hooks via `Command::new("sh").arg("-c").arg(hook_command)`. If `hook_command` is constructed from user-controlled config values, this is a shell injection vector.
**Files:** `crates/plugins/src/hooks.rs:166`, `crates/plugins/src/hooks.rs:300`
**Fix approach:** Verify that `hook_command` originates solely from the static plugin manifest (not from runtime user input). Add a comment documenting the trust boundary. Consider spawning without a shell shell for simple commands.

### OAuth State / PKCE Not Validated in `oauth_flow.rs`

**Issue:** `crates/cli/src/oauth_flow.rs:234` shows a comment parsing `?code=XXX&state=YYY` from the callback URL, but there is no visible assertion that the returned `state` matches the originally generated state. Missing state validation allows CSRF attacks against the OAuth flow.
**Files:** `crates/cli/src/oauth_flow.rs`
**Fix approach:** Confirm that `state` round-trips and is checked before exchanging the code. If already done elsewhere, add a comment citing the location.

### `web_fetch` Fetches Arbitrary URLs Without SSRF Protection

**Issue:** The `WebFetch` tool (`crates/tools/src/web_fetch.rs`) accepts any URL from the AI without checking whether it is a private/loopback/link-local address. An adversarial model response could instruct it to probe internal services (`http://169.254.169.254/`, `http://localhost:8080/`, etc.).
**Files:** `crates/tools/src/web_fetch.rs:327`
**Fix approach:** Resolve the hostname before connecting and reject RFC 1918, loopback, link-local, and APIPA ranges.

---

## Performance Notes

### Excessive `.clone()` Calls in Hot Paths

**Issue:** `crates/commands/src/lib.rs` contains 134 `.clone()` calls and `crates/tui/src/app.rs` contains 68. Many clone large `Vec<Message>` or `String` values on each UI render tick or command dispatch.
**Impact:** Increased allocator pressure; measurable latency in long sessions with many messages.
**Fix approach:** Audit clone sites in render and dispatch paths. Prefer `Arc<T>` for shared read-only data; pass `&T` references where ownership transfer is not required.

### New `reqwest::Client` Created Per `web_fetch` Call

**Issue:** `crates/tools/src/web_fetch.rs:317-320` constructs a fresh `reqwest::Client` on every invocation of `WebFetchTool::execute`. `reqwest::Client` maintains a connection pool; creating a new one per call discards all pooled connections.
**Files:** `crates/tools/src/web_fetch.rs:317`
**Fix approach:** Store a `reqwest::Client` as a lazily-initialized static (e.g., `once_cell::sync::Lazy`) or inject it via `ToolContext`.

### Bedrock SigV4 HMAC Computed Without Caching

**Issue:** `crates/api/src/providers/bedrock.rs:213-240` recomputes the SigV4 signing key (4 chained HMAC-SHA256 operations) on every request. The signing key is derived from the date and region, which change at most once per day.
**Files:** `crates/api/src/providers/bedrock.rs:205-245`
**Fix approach:** Cache the signing key (keyed on `date + region + secret`) with a 24-hour TTL.

### `SqliteSessionStore` Uses a Non-Shared Connection

**Issue:** `crates/core/src/sqlite_storage.rs` wraps a single `rusqlite::Connection`. If multiple tasks access the store concurrently (possible given Tokio's multi-threaded executor), the `Arc`-less connection will serialize all access and may create lock contention. SQLite in WAL mode with a connection pool would be more scalable.
**Files:** `crates/core/src/sqlite_storage.rs:11`

---

## Incomplete Features

### `settings_sync.rs` — Multiple `#[allow(dead_code)]` Fields on Wire Types

**Issue:** `UserSyncData`, `UploadResponse` in `crates/core/src/settings_sync.rs:66-86` suppress dead-code warnings on `user_id`, `version`, `last_modified`, `checksum` fields. These are deserialized from the API but never read. This suggests the sync feature is partially implemented — the ETag/version-based conflict detection is not yet used.
**Files:** `crates/core/src/settings_sync.rs:66-86`

### Voice Feature Only Partially Wired

**Issue:** `crates/core/src/voice.rs` and `crates/tui/src/voice_capture.rs` gate real audio capture behind `#[cfg(feature = "voice")]`. The feature is not in the `default` feature set and is not listed as enabled in the production `cli` Cargo.toml, meaning voice input is always a no-op in shipped binaries. Tests in `voice.rs` that mutate env vars also run in the default test suite despite the feature being off.
**Files:** `crates/core/src/voice.rs`, `crates/tui/src/voice_capture.rs`

### `DEFAULT_MAX_RETRIES` Defined but Never Used in `settings_sync.rs`

**Issue:** `crates/core/src/settings_sync.rs:30` defines `const DEFAULT_MAX_RETRIES: u32 = 3` marked `#[allow(dead_code)]`. The actual retry loop uses a hardcoded `3` at the call site. The constant is vestigial.
**Files:** `crates/core/src/settings_sync.rs:30`

### `computer_use.rs` — Three Enums Entirely Suppressed as Dead Code

**Issue:** `crates/tools/src/computer_use.rs:49-53` has three consecutive `#[allow(dead_code)]` attributes on enum variants. The computer-use feature appears incomplete — structs are defined but only used in `#[cfg(feature = "computer-use")]` blocks that are not enabled in default or `dev_full` builds.
**Files:** `crates/tools/src/computer_use.rs:49-53`

### `team_memory_sync.rs` — 412 Conflict Response Not Retried

**Issue:** `crates/core/src/team_memory_sync.rs:336` detects an ETag mismatch (HTTP 412) and returns `anyhow::bail!("... retry needed")` but the caller never retries. The sync will silently fail until the next session start.
**Files:** `crates/core/src/team_memory_sync.rs:336`

---

## Recommendations

1. **Restrict `~/.claurst/auth.json` permissions to 0o600** immediately — this is a low-effort, high-impact security fix. (`crates/core/src/auth_store.rs`)

2. **Audit and fix `std::env::set_var` in async test code** before upgrading to Rust editions/toolchains that make this undefined behavior stricter. Use `serial_test` or `#[serial]` for tests that mutate process-global state.

3. **Add a byte cap before `resp.text().await` in `web_fetch`** to prevent OOM from unbounded HTTP responses. (`crates/tools/src/web_fetch.rs:351`)

4. **Replace `panic!` in `match` arms with `Result` returns** in non-test code, starting with `crates/core/src/lib.rs`, `crates/api/src/providers/message_normalization.rs`, and `crates/query/src/lib.rs`.

5. **Add integration tests for all provider adapters** using mock HTTP servers. The zero-coverage provider files represent the largest regression risk surface.

6. **Split `crates/commands/src/lib.rs` (8,576 lines) and `crates/tui/src/app.rs` (5,918 lines)** into smaller modules — this is the most important maintainability improvement.

7. **Standardize on `parking_lot::Mutex`** to eliminate mutex poisoning as a failure mode.

8. **Escape LIKE wildcards in `search_sessions`** to prevent unexpected behavior with user-supplied search terms.

9. **Add SSRF protection to `WebFetchTool`** by blocking loopback, RFC 1918, and link-local destinations before making the outbound request.

10. **Reuse `reqwest::Client` in `web_fetch`** via a process-level singleton to benefit from connection pooling.

---

*Concerns audit: 2026-05-04*
