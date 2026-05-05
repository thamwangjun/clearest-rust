# Codebase Concerns

**Analysis Date:** 2026-05-05

## Tech Debt

**God-file modules — commands/lib.rs and tui/app.rs:**
- Issue: `crates/commands/src/lib.rs` is 8,576 lines; `crates/tui/src/app.rs` is 5,990 lines; `crates/core/src/lib.rs` is 4,246 lines; `crates/cli/src/main.rs` is 3,502 lines. Each mixes unrelated responsibilities and is effectively unmaintainable at its current size.
- Files: `crates/commands/src/lib.rs`, `crates/tui/src/app.rs`, `crates/core/src/lib.rs`, `crates/cli/src/main.rs`
- Impact: High friction for any change — impossible to reason about the full state machine, merge conflicts are frequent, and tests cannot target isolated units.
- Fix approach: Extract sub-modules by responsibility. For `commands/lib.rs` — split named commands into their own files per command group. For `tui/app.rs` — extract event handler, render pipeline, and state update into separate modules. For `core/lib.rs` — move error types, config types, and message types into dedicated files.

**Pervasive `.unwrap()` in production paths:**
- Issue: ~370 `.unwrap()` calls exist across the codebase (after filtering test code). Notable production-path examples: `crates/tools/src/pty_bash.rs:91`, `crates/tools/src/bash.rs:82`, `crates/core/src/team_memory_sync.rs:99`, `crates/core/src/team_memory_sync.rs:374`, `crates/core/src/keybindings.rs:418`, `crates/core/src/system_prompt.rs:572`, `crates/core/src/settings_sync.rs:493`.
- Files: Widespread — highest density in `crates/tools/src/`, `crates/core/src/`
- Impact: Any `None`/`Err` on those paths panics the process. The keybindings unwrap at line 418 (`exact.last().unwrap()`) runs on every keypress matching. The system_prompt unwrap at line 572 panics if the dynamic boundary marker is ever absent from the prompt string.
- Fix approach: Replace with `?` propagation or `unwrap_or_else`. For invariants that should never fail (e.g. regexes compiled from string literals), document with a comment or use a `const`-initialised `OnceLock`.

**Regex recompiled on every call:**
- Issue: Several hot paths recompile regex patterns on every invocation instead of caching them in a static. Affected: `crates/tools/src/bash.rs:81` (`extract_exports_from_command` — called per bash command), `crates/tools/src/pty_bash.rs:88` (called per PTY output chunk), `crates/tui/src/messages/markdown.rs:14,20` (URL and email detection, called per render), `crates/tui/src/messages/markdown_enhanced.rs:14,20` (table detection).
- Files: `crates/tools/src/bash.rs`, `crates/tools/src/pty_bash.rs`, `crates/tui/src/messages/markdown.rs`, `crates/tui/src/messages/markdown_enhanced.rs`
- Impact: Measurable CPU overhead on every bash execution and every TUI render frame. `Regex::new` is not cheap.
- Fix approach: Use `once_cell::sync::Lazy<Regex>` statics (pattern already used elsewhere in the codebase, e.g., `crates/bridge/src/lib.rs:233`).

**Dependency injection via process-global panic on double-init:**
- Issue: `crates/tools/src/team_tool.rs:60-68` uses a `OnceCell<AgentRunFn>` static and panics if `register_agent_runner` is called more than once. The comment acknowledges this is a circular-dependency workaround.
- Files: `crates/tools/src/team_tool.rs`
- Impact: Makes testing multi-agent paths fragile; integration test suites that reinitialise would panic. Documents an architectural coupling that cannot be resolved without refactoring the crate dependency graph.
- Fix approach: Pass the runner through `ToolContext` instead of a global, breaking the circular dependency at the type level.

**`#[allow(dead_code)]` suppression without cleanup:**
- Issue: 15+ `#[allow(dead_code)]` annotations across the codebase. Heavy concentration in `crates/core/src/settings_sync.rs` (6 annotations), `crates/cli/src/codex_oauth_flow.rs` (file-level `#![allow(dead_code)]`), `crates/tui/src/app.rs` (multiple).
- Files: `crates/core/src/settings_sync.rs`, `crates/cli/src/codex_oauth_flow.rs`, `crates/tui/src/app.rs`, `crates/tools/src/computer_use.rs`, `crates/tools/src/web_fetch.rs`
- Impact: Unmaintained dead code accumulates; future refactors may activate suppressed-but-broken code paths.
- Fix approach: Delete unreachable code or make it reachable. Remove `#![allow(dead_code)]` from `codex_oauth_flow.rs` and either integrate or delete the module.

**Error type inconsistency — `anyhow` vs `ClaudeError`:**
- Issue: ~374 function signatures use `anyhow::Result` while a typed `ClaudeError` (`crates/core/src/lib.rs:97-144`) exists for structured error handling. The two error systems are mixed without a clear boundary.
- Files: Across all crates; `ClaudeError` defined in `crates/core/src/lib.rs:97`
- Impact: Callers cannot match on structured error variants from functions returning `anyhow::Error`. Loss of actionable error distinctions (e.g., `RateLimit` vs `AuthenticationError`).
- Fix approach: Establish a rule — `anyhow::Result` for leaf/internal functions, `ClaudeError` at public API boundaries. Migrate crate-public functions incrementally.

**No CI/CD pipeline:**
- Issue: No `.github/` directory, no CI configuration files (`.yml`/`.yaml`) found anywhere in the repository. No `deny.toml` (cargo-deny) or `audit.toml` (cargo-audit) for supply chain security.
- Files: Repository root
- Impact: No automated test gate on pull requests, no dependency vulnerability scanning, no enforced lint pass. Regressions can merge without detection.
- Fix approach: Add a minimal GitHub Actions workflow with `cargo test`, `cargo clippy -- -D warnings`, and `cargo audit`.

## Known Bugs

**`env::set_var` called from async context without synchronisation:**
- Symptoms: Tests in `crates/core/src/lib.rs` and `crates/mcp/src/lib.rs` call `std::env::set_var` without holding any global lock. Rust 1.80+ emits a lint for this; in multithreaded test runs (default for `cargo test`) this is a data race.
- Files: `crates/core/src/lib.rs:3828,3843,3850,3858`, `crates/mcp/src/lib.rs:1416,1439,1440,1449`
- Trigger: `cargo test` with the default parallel test runner; any test that reads the same env var concurrently.
- Workaround: The `crates/query/src/coordinator.rs` tests correctly serialise via a `Mutex<()>` guard (`ENV_LOCK`). The `core/lib.rs` API-key tests do not.

**`system_prompt` panics on missing dynamic boundary:**
- Symptoms: `crates/core/src/system_prompt.rs:572` calls `.unwrap()` on `prompt.find(SYSTEM_PROMPT_DYNAMIC_BOUNDARY)`. If any code path produces a system prompt without the boundary marker, the process crashes.
- Files: `crates/core/src/system_prompt.rs:572`
- Trigger: A custom or override system prompt that does not include the expected sentinel string.
- Workaround: None at runtime — the crash is immediate.

**Mutex poison not handled in `rmcp_backend`:**
- Symptoms: `crates/mcp/src/rmcp_backend.rs` calls `.lock().expect("... mutex poisoned")` at lines 284, 287, 309, 316, 324, 340, 350, 367, 420, 479. If any task panics while holding these locks, all subsequent callers will panic with "mutex poisoned" rather than recovering.
- Files: `crates/mcp/src/rmcp_backend.rs`
- Trigger: Any panic inside an MCP background task that holds the endpoint or task mutex.
- Workaround: Replace `.expect()` with `.unwrap_or_else(|p| p.into_inner())` to recover poisoned locks, as the coordinator tests already do.

## Security Considerations

**`std::env::set_var` / `remove_var` in multithreaded production code:**
- Risk: `crates/query/src/coordinator.rs:206,211` mutates environment variables inside `unsafe` blocks in production code (not test code). `std::env::set_var` is documented as unsound in multithreaded programs because it can corrupt the process environment (glibc `putenv` is not thread-safe).
- Files: `crates/query/src/coordinator.rs:203-215`
- Current mitigation: Comment notes mutation "only happens at session resume time before any worker threads are spawned" — but this is not enforced by the type system.
- Recommendations: Store coordinator mode in a `AtomicBool` static rather than an environment variable. If env-var propagation to child processes is required, set it before spawning.

**HTTP clients instantiated without timeouts:**
- Risk: Multiple sites call `reqwest::Client::new()` without setting connect or read timeouts. If an external server hangs, the calling async task will block indefinitely, potentially exhausting the Tokio thread pool.
- Files: `crates/tools/src/remote_trigger.rs:71`, `crates/core/src/team_memory_sync.rs:140,294`, `crates/tools/src/web_search.rs:81,141`, `crates/core/src/oauth_config.rs:228`, `crates/core/src/remote_session.rs:63,87`, `crates/core/src/device_code.rs:28,57`
- Current mitigation: Some callers set timeouts (`crates/core/src/update_check.rs:65`, `crates/core/src/remote_settings.rs:111` use `.builder()`). No consistent policy.
- Recommendations: Create a shared `build_http_client(timeout: Duration) -> reqwest::Client` helper and use it everywhere. Enforce a maximum timeout (e.g., 30 s connect / 60 s read).

**`xcap` screen capture dependency at version 0.0.13:**
- Risk: `xcap = "0.0.13"` is a pre-1.0 crate with no stability guarantees. Screen capture APIs have platform-level permissions implications (macOS screen recording permission, Wayland restrictions). The version `0.0.13` may have unpatched CVEs or capability-escalation bugs.
- Files: `Cargo.toml:74`
- Current mitigation: ComputerUse feature is gated behind `#[cfg(feature = "computer-use")]`, so it is not compiled by default.
- Recommendations: Pin to a specific patch version in the lockfile (already locked via `Cargo.lock`). Monitor for security advisories on this crate.

**`MCP client backend` panics when accessed before connection:**
- Risk: `crates/mcp/src/lib.rs:734` calls `.expect("MCP client backend missing")` on a public method `subscribe_to_notifications`. If a caller invokes this before the MCP client has completed its handshake, the process crashes rather than returning an error.
- Files: `crates/mcp/src/lib.rs:733-736`
- Current mitigation: None — the caller is responsible for sequencing.
- Recommendations: Return `Result<..., McpError>` with a `NotConnected` variant instead of panicking.

## Performance Bottlenecks

**Unbounded `.clone()` on large `Arc<>` message structures:**
- Problem: 1,491 `.clone()` / `Arc::clone` calls appear across the codebase. Many are clones of `Arc<ToolContext>` or message history vectors passed through async task boundaries. Message history is unbounded and grows with conversation length.
- Files: Widespread; heaviest in `crates/query/src/lib.rs`, `crates/tui/src/app.rs`, `crates/commands/src/lib.rs`
- Cause: Shared ownership across async tasks without a clear ownership boundary forces cloning at every hand-off.
- Improvement path: Audit whether full history clones are needed or whether an index/slice reference suffices. For `ToolContext`, pass `Arc<ToolContext>` directly rather than cloning on each tool dispatch.

**SQLite accessed synchronously via `block_in_place`:**
- Problem: `crates/commands/src/named_commands.rs:217,246,290,321,358,380,401,411` calls `tokio::task::block_in_place` to run synchronous SQLite operations (`rusqlite`) inside async functions. Each `block_in_place` stalls the current Tokio worker thread.
- Files: `crates/commands/src/named_commands.rs`
- Cause: `rusqlite` is synchronous-only; integrating it requires either `block_in_place` or a dedicated blocking thread pool.
- Improvement path: Move SQLite operations to a dedicated `tokio::task::spawn_blocking` thread pool (a bounded `rayon` or `tokio` blocking pool) rather than stalling worker threads inline.

**TUI markdown regex recompiled per render frame:**
- Problem: `crates/tui/src/messages/markdown.rs:14,20` and `crates/tui/src/messages/markdown_enhanced.rs:14,20` call `Regex::new(...)` inside functions that are invoked on every TUI render cycle.
- Files: `crates/tui/src/messages/markdown.rs`, `crates/tui/src/messages/markdown_enhanced.rs`
- Cause: No caching layer; each call to the detection helpers allocates and compiles a new `Regex`.
- Improvement path: Wrap in `once_cell::sync::Lazy<Regex>` statics. This is a 3-line change per pattern.

## Fragile Areas

**`team_tool` / `AGENT_RUNNER` global singleton:**
- Files: `crates/tools/src/team_tool.rs:60-68`
- Why fragile: `register_agent_runner` must be called exactly once at startup. Calling it twice panics. Never calling it causes `TeamCreateTool` to return a stub result silently — the error is not surfaced to the user as an error, just a placeholder string.
- Safe modification: Any refactor of the startup sequence in `crates/cli/src/main.rs` or `crates/query/src/lib.rs` must preserve the single registration call.
- Test coverage: No test for the "runner not registered" silent-failure path.

**`coordinator.rs` environment variable mode flag:**
- Files: `crates/query/src/coordinator.rs:195-215`
- Why fragile: Coordinator mode is signalled via `std::env::set_var` and read via `std::env::var`. Any child process that inherits the environment will also appear to be in coordinator mode. Tests that run in parallel and mutate this variable race against each other.
- Safe modification: Always hold `ENV_LOCK` (the `Mutex<()>` defined in the test module) when setting or reading this variable in tests. Production code relies on a comment-level guarantee about spawn ordering.
- Test coverage: Tests use `ENV_LOCK` correctly, but the guarantee is unenforced in production code.

**`rmcp_backend` mutex poison chain:**
- Files: `crates/mcp/src/rmcp_backend.rs`
- Why fragile: 10+ `.expect("... mutex poisoned")` calls create a cascade: one panicking background task poisons all mutexes, making every subsequent MCP operation panic. The MCP subsystem becomes permanently non-functional until the process restarts.
- Safe modification: Replace `.expect()` with `.unwrap_or_else(|p| p.into_inner())` throughout.
- Test coverage: No test exercises mutex-poison recovery.

**`system_prompt` dynamic boundary assumption:**
- Files: `crates/core/src/system_prompt.rs:572`
- Why fragile: `build_system_prompt` always embeds the boundary marker, but any custom `--append-system-prompt` or test override that replaces the whole prompt bypasses that guarantee.
- Safe modification: Replace `.unwrap()` with `ok_or_else(...)` and propagate an error, or make the boundary marker insertion mandatory by changing the type.
- Test coverage: Tests use `default_opts()` which always produces a prompt with the boundary; no adversarial test.

## Scaling Limits

**Unbounded in-memory task registry:**
- Current capacity: The `TASK_STORE` global (`crates/tools/src/tasks.rs:112`) is a `DashMap<String, Task>` with no eviction policy.
- Limit: Grows without bound over the process lifetime; long-running sessions with many background tasks will accumulate entries indefinitely.
- Scaling path: Add a configurable max-size or TTL-based eviction. Completed tasks should be pruned after a retention window.

**In-memory REPL session registry:**
- Current capacity: `REPL_SESSIONS` (`crates/tools/src/repl_tool.rs:44`) is a `DashMap` with no limit.
- Limit: Each REPL session holds an open subprocess handle. Many sessions left open will exhaust file descriptors.
- Scaling path: Add a session idle timeout and cleanup mechanism.

## Dependencies at Risk

**`xcap = "0.0.13"` — pre-alpha screen capture:**
- Risk: Version `0.0.13` signals an experimental crate with no stability contract. API breaking changes or yanks are likely. macOS screen recording permission behaviour can change across OS updates.
- Impact: `crates/tools/src/computer_use.rs` will fail to compile if the crate is yanked or its API changes.
- Migration plan: Consider replacing with a more mature alternative or vendoring the specific version if computer-use is a required feature.

**`schemars = "0.8"` while 0.9+ exists with breaking changes:**
- Risk: `schemars` 0.9 has breaking schema generation changes. Pinned at `0.8` means the project will lag behind and face a forced migration eventually.
- Impact: Tool JSON schemas generated via `schemars::schema_for!` are used in API calls to Anthropic — schema format differences could cause API validation failures on upgrade.
- Migration plan: Upgrade to `schemars 0.9` in a dedicated PR; audit all `JsonSchema` derive usages and schema output.

## Missing Critical Features

**No supply-chain security tooling:**
- Problem: No `cargo-deny` config (`deny.toml`) and no `cargo-audit` config (`audit.toml`) are present. The workspace has 12 crates and a large transitive dependency tree.
- Blocks: Cannot detect known CVEs in dependencies automatically. Any CI pipeline added later will need to retrofit this.

**No test coverage measurement:**
- Problem: No `cargo-tarpaulin` or `llvm-cov` configuration exists. Test coverage is unknown. 85 out of ~205 non-lib source files have no `#[test]` functions at all (41%).
- Blocks: Cannot enforce coverage requirements or identify regressions in coverage.

## Test Coverage Gaps

**`crates/bridge/` — zero unit tests:**
- What's not tested: The HTTP bridge polling loop, session registration/deregistration, event batching, and reconnection logic in `crates/bridge/src/lib.rs` (1,713 lines).
- Files: `crates/bridge/src/lib.rs`
- Risk: Reconnection and event-ordering bugs are invisible until production load.
- Priority: High

**`crates/query/src/lib.rs` — only 5 integration-style tests at the bottom:**
- What's not tested: Core query loop logic, token budget enforcement, tool dispatch orchestration, streaming response handling — all in the 2,410-line `run_query_loop` function.
- Files: `crates/query/src/lib.rs`
- Risk: The most critical runtime path has the fewest tests.
- Priority: High

**`crates/tools/src/computer_use.rs` — feature-gated tests only:**
- What's not tested: All `#[cfg(feature = "computer-use")]` action dispatch paths (mouse, keyboard, screenshot) have no tests that run in the default build.
- Files: `crates/tools/src/computer_use.rs`
- Risk: Computer-use builds can ship with broken action handlers undetected.
- Priority: Medium

**`crates/acp/` — entire crate has no visible tests:**
- What's not tested: The ACP (Agent Communication Protocol) crate at `crates/acp/` is listed as a workspace member but has no test files discoverable.
- Files: `crates/acp/`
- Risk: Protocol-level bugs in agent-to-agent communication are undetected.
- Priority: Medium

---

*Concerns audit: 2026-05-05*
