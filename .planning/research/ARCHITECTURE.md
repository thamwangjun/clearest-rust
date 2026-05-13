# Architecture Patterns: v1.1 Codebase Refactor

**Domain:** Large-scale behavior-preserving Rust codebase refactoring
**Researched:** 2026-05-13
**Confidence:** HIGH — based on live codebase analysis, official Rust/ratatui/cargo docs, and verified tooling

---

## Actual Dependency Graph (Derived from Cargo.toml files)

```
                        claurst-cli (binary)
                        ┌──────────────────────────────────────────┐
                        │ depends on: ALL 11 library crates        │
                        └──────────────────────────────────────────┘
                                        │
             ┌──────────────────────────┼──────────────────────────┐
             ▼                          ▼                          ▼
     claurst-commands            claurst-tui                 claurst-acp
     (depends on: core,          (depends on: core,          (depends on:
      api, tools, query,          api, tools, query,          core, api)
      mcp, tui, plugins,          mcp)
      bridge)
             │                          │
             └──────────────────────────┘
                          │
                          ▼
               claurst-query ◄─────────────────────────────────────┐
               (depends on: core, api, tools, plugins)             │
                    │              │                               │
                    ▼              ▼                               │
             claurst-tools   claurst-plugins                       │
             (depends on:    (depends on: core)                    │
              core, api, mcp)                                      │
                    │                                              │
                    ▼                                              │
             claurst-api                            claurst-bridge
             (depends on: core)                     (depends on: core, api, query)
                    │
                    ▼
             claurst-mcp
             (depends on: core)
                    │
                    ▼
             claurst-core  ◄── claurst-buddy (isolated, no other workspace deps)
             (no workspace
              crate deps —
              foundation layer)
```

### Topological Layers (bottom = leaf/foundation, top = dependent)

```
Layer 0 — Foundation (no workspace deps):
  claurst-core, claurst-buddy

Layer 1 — Protocol/Adapter (depends only on core):
  claurst-api, claurst-mcp, claurst-plugins, claurst-acp (+ core + api)

Layer 2 — Tool Layer (depends on core + api + mcp):
  claurst-tools

Layer 3 — Orchestration (depends on core + api + tools + plugins):
  claurst-query

Layer 4 — Bridge (depends on core + api + query):
  claurst-bridge

Layer 5 — UI/Command (depends on query + most others):
  claurst-tui, claurst-commands

Layer 6 — Binary (depends on all):
  claurst-cli
```

---

## Codebase Size Reality Check

| Crate | Lines | Files | Primary Smell |
|-------|-------|-------|---------------|
| `claurst-tui` | 38,402 | 51 | `app.rs` = 5,990 lines; god class |
| `claurst-core` | 21,334 | 47 | `lib.rs` = 4,291 lines; mixed responsibilities |
| `claurst-api` | 13,879 | 29 | Well-structured; providers/ already split |
| `claurst-tools` | 11,494 | 39 | ~370 `.unwrap()` in prod paths; reasonable file sizes |
| `claurst-commands` | 9,906 | 2 | `lib.rs` = 8,657 lines — worst god-file in workspace |
| `claurst-query` | 7,069 | 12 | `lib.rs` = 2,410 lines; `run_query_loop` is 2,400 lines |
| `claurst-cli` | ~3,732 | 3 | `main.rs` = 3,732 lines; logic that belongs in lib crates |
| `claurst-mcp` | 4,345 | 6 | Mutex poison chain fragility |
| `claurst-bridge` | 1,704 | 1 | Zero tests; entire crate = one file |
| `claurst-plugins` | 2,651 | 7 | Moderate |
| `claurst-buddy` | 1,118 | 1 | Small, isolated |
| `claurst-acp` | 285 | 1 | Small, no tests |

---

## Recommended Refactoring Phase Order

### Principle: Bottom-up, characterization-first

Refactor leaf crates before dependent crates. Every refactoring step must keep `cargo test` green on the affected crate before moving up the dependency chain. The characterization test suite is the pre-condition for starting any code movement.

```
Phase A: Characterization Test Infrastructure
  → Build the safety net before touching production code

Phase B: Layer 0 — claurst-core (foundation; all others depend on it)
  → Highest leverage: fixes here benefit all 11 dependents

Phase C: Layer 1 — claurst-api, claurst-mcp, claurst-plugins, claurst-acp
  → Protocol/adapter layer; well-bounded responsibilities

Phase D: Layer 2 — claurst-tools
  → Tool implementations; depends on B and C being stable

Phase E: Layer 3+4 — claurst-query, claurst-bridge
  → Orchestration; most complex async flows

Phase F: Layer 5 — claurst-tui, claurst-commands
  → Largest god-files; highest risk; needs stable lower layers

Phase G: Layer 6 — claurst-cli
  → Binary wiring; extract McpToolWrapper to a library crate
```

**Why leaf-first over most-depended-on-first:** `claurst-core` is both the most depended-on crate AND a leaf. For all other crates, refactoring a dependent before its dependencies are stable means every refactoring of the dependency invalidates the work above. Leaf-first prevents rework cascades.

**Critical ordering constraint:** `claurst-commands` depends on `claurst-tui` (confirmed from Cargo.toml). This means `claurst-tui` must be stable before `claurst-commands` refactoring, even though `claurst-commands` has the worst god-file. Plan for `claurst-commands` to be the final library crate refactored.

---

## Question 1: Characterization Tests for Async Rust and TUI State Machines

### For Pure Functions and Sync State

Use inline `#[cfg(test)] mod tests` with `assert_eq!`. The existing codebase already does this for 1,221 sync tests. No new tooling needed for this class.

### For Async Logic (`claurst-query`, `claurst-bridge`, `claurst-api`)

**Pattern:** `#[tokio::test]` with a test-scoped `ToolContext` constructed manually.

```rust
#[tokio::test]
async fn characterize_tool_execute_bash_safe() {
    let ctx = make_test_ctx();  // local helper, returns ToolContext
    let tool = BashTool;
    let input = serde_json::json!({"command": "echo hello"});
    let result = tool.execute(input, &ctx).await;
    assert!(!result.is_error, "safe bash should not error: {}", result.content);
    // Pin the exact output as a characterization anchor:
    assert_eq!(result.content.trim(), "hello");
}
```

**Key rule:** Async tests that touch `std::env::set_var` MUST use a `Mutex<()>` serialization guard to prevent races. The existing `ENV_LOCK` pattern in `crates/query/src/coordinator.rs` is the model to follow. Add `serial_test` crate for simpler serialization on env-var-dependent tests.

**What to avoid:** Do not spawn real HTTP calls in characterization tests. The network is not available in CI. If an existing code path calls the API unconditionally, wrap it behind a `MockProvider` implementing `LlmProvider` trait:

```rust
struct MockProvider { responses: Vec<String> }

#[async_trait]
impl LlmProvider for MockProvider {
    async fn create_message(&self, _req: CreateMessageRequest) -> Result<ProviderResponse> {
        Ok(ProviderResponse { content: self.responses[0].clone(), ..Default::default() })
    }
    // ...
}
```

### For TUI State Machines (`claurst-tui`)

**Pattern:** `ratatui::backend::TestBackend` + `insta` snapshot assertions.

Ratatui ships `TestBackend` in the core crate (no extra dependency). Pair it with `insta` for snapshot management.

```rust
// In crates/tui/tests/render_characterization.rs
use ratatui::{backend::TestBackend, Terminal};

#[test]
fn characterize_welcome_screen_render() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new_for_test();  // constructor that bypasses I/O init

    terminal.draw(|frame| app.render(frame)).unwrap();

    // First run creates snapshot; subsequent runs assert against it
    insta::assert_snapshot!(terminal.backend());
}
```

**App state isolation:** `App::new_for_test()` must be added — a constructor that:
- Uses `TestBackend` instead of `CrosstermBackend`
- Does not call `crossterm::terminal::enable_raw_mode()`
- Sets a deterministic `AppState` (e.g., `AppState::Welcome`)
- Injects a `MockProvider` instead of a real provider

**Snapshot files** go in `crates/tui/tests/snapshots/`. Commit the `.snap` files. Use `cargo insta review` to accept initial snapshots.

**State machine coverage strategy:** Write one characterization test per `AppState` variant (Welcome, ProviderSetup, Chat, Settings, etc.). Each test:
1. Constructs `App` in that state with minimal mock data
2. Calls `terminal.draw()`
3. Asserts snapshot

This gives a regression baseline for every screen before any refactoring touches rendering code.

### For CLI Output (`claurst-cli`)

**Pattern:** `assert_cmd` + `predicates` for process-level integration tests. Add as `dev-dependencies` in `crates/cli/Cargo.toml`.

```rust
// In crates/cli/tests/cli_integration.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn characterize_version_flag() {
    Command::cargo_bin("claurst")
        .unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("claurst"));
}

#[test]
fn characterize_help_flag() {
    Command::cargo_bin("claurst")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"USAGE:").unwrap());
}
```

For headless `-p` mode output characterization, use `insta_cmd`:

```rust
use insta_cmd::assert_cmd_snapshot;

#[test]
fn characterize_print_mode_status_command() {
    assert_cmd_snapshot!(Command::cargo_bin("claurst").unwrap()
        .args(["-p", "/status", "--no-color"])
        .env("ANTHROPIC_API_KEY", "test-key")
    );
}
```

**What to characterize at the CLI level:**
- `--version` output (exact string)
- `--help` output (structural; contains expected subcommands)
- ACP `initialize` response structure (JSON schema)
- Headless `-p "/status"` JSON output shape

**What NOT to characterize at the CLI level:** Any path that makes network calls. These belong in unit/integration tests with mocks, not process-level tests.

---

## Question 2: Crate Refactoring Order

See "Recommended Refactoring Phase Order" above. The tiebreakers within each layer:

### Layer 0: `claurst-core` first

`claurst-core/src/lib.rs` is 4,291 lines mixing module re-exports, error types, config types, auth types, and message types. The safe decomposition sequence:

1. Extract `error.rs` — `ClaudeError`, `ToolError` (these have no dependencies on other types in lib.rs)
2. Extract `message_types.rs` — `Message`, `MessageContent`, `Role`, `ContentBlock` (depend on error types only)
3. Extract `config.rs` — `Settings`, `Config`, `PermissionMode` (depend on message types)
4. Extract `auth.rs` — `AuthStore`, credential types (depend on config)
5. Leave `lib.rs` as pure re-exports (`pub use`) pointing to the new modules

Each step: run `cargo test -p claurst-core`. If green, commit. Never move more than one responsibility per commit.

### Within `claurst-tui`: `app.rs` last

`app.rs` is 5,990 lines. Decompose supporting modules first (they are smaller and isolated), then extract from `app.rs`:

1. Decompose `prompt_input.rs` (3,719 lines) — extract `input_history.rs`, `completion.rs`, `input_render.rs`
2. Decompose `overlays.rs` (2,103 lines) — extract per-overlay modules under `overlays/`
3. Decompose `dialogs.rs` (1,621 lines) — extract per-dialog modules under `dialogs/`
4. Then tackle `app.rs`: extract `event_handler.rs`, `state_transitions.rs`, `render_pipeline.rs`

### `claurst-commands/src/lib.rs` — treat as final library crate

At 8,657 lines, this is the highest-risk file. It depends on `claurst-tui`, meaning `claurst-tui` must be stable first. When tackling it:
1. Extract each slash command into `commands/src/slash/command_name.rs`
2. Keep `lib.rs` as dispatch only (match on command name → call handler)
3. Move `named_commands.rs` logic into a `named/` module tree

---

## Question 3: Extracting Methods/Modules Without Fighting the Borrow Checker

### Rule 1: Extract pure functions first

Functions that take only `&self` or immutable references, have no side effects, and return owned values are zero-risk extractions. They compile immediately. Start here.

### Rule 2: Clone at the boundary, optimize later

When extracting a method that needs multiple `&mut` fields, the borrow checker will object if you pass `&mut self.field_a` and `&mut self.field_b` simultaneously. The safe pattern during refactoring:

```rust
// Instead of fighting:
fn process(&mut self) {
    self.helper(&mut self.messages, &mut self.state)  // borrow error
}

// Clone to unblock:
fn process(&mut self) {
    let messages = self.messages.clone();
    let new_state = Self::compute_state(&messages);
    self.state = new_state;
}

// After refactoring is complete and tests pass, optimize the clone away
```

The constraint is behavior-preserving refactoring. Performance optimization is a separate subsequent pass.

### Rule 3: Use struct decomposition to resolve split-borrows

When `App` or another god struct holds logically independent sub-states, extract them into nested structs. The borrow checker allows simultaneous mutable borrows of distinct struct fields:

```rust
// Before: can't borrow both
struct App {
    messages: Vec<Message>,
    input: InputState,
    render_cache: RenderCache,
}

// After: borrow checker happy
struct App {
    conversation: ConversationState,  // owns messages, history
    input: InputState,                // owns prompt, cursor
    view: ViewState,                  // owns render_cache, scroll_pos
}

// Now allowed:
let (conv, input) = (&mut self.conversation, &mut self.input);
```

### Rule 4: Use `Arc<Mutex<T>>` only when shared ownership is genuinely needed

The codebase already uses `Arc<parking_lot::Mutex<T>>` for shared state. During extraction, resist the temptation to wrap everything in `Arc<Mutex>` to silence borrow errors. Instead:
- If two modules both need a value, ask: can one own it and the other borrow it?
- If one module mutates and another reads, `Arc<RwLock<T>>` is appropriate
- `Arc<Mutex<T>>` is only needed when two tasks mutate concurrently

### Rule 5: `async fn` extraction — lifetime pitfalls

Rust's async/await has a subtle rule: captured references in `async fn` bodies must outlive the `Future`. When extracting an `async fn` that borrows from `self`:

```rust
// This may not compile if the extracted fn captures &self across an .await:
async fn process(&self) -> Result<()> {
    let result = self.some_field.do_something().await;  // &self held across await
    self.update(result);  // borrow still held
}

// Safe pattern: clone or move what you need before the await point:
async fn process(&self) -> Result<()> {
    let input = self.some_field.clone();  // clone before await
    let result = input.do_something().await;
    self.update(result);
}
```

### Rule 6: `#[allow(clippy::too_many_arguments)]` is a temporary extraction aid

When moving a long method to a free function, parameter lists will be large until structs are cleaned up. Add `#[allow(clippy::too_many_arguments)]` during extraction; remove it once parameter lists are consolidated into structs. This prevents clippy from blocking CI during the refactoring window.

---

## Question 4: Cargo Workspace Features for Refactoring Risk Isolation

### Use `-p` to test one crate at a time

```bash
cargo test -p claurst-core           # test only core; fails fast without noise from other crates
cargo clippy -p claurst-core -- -D warnings  # lint only core
cargo check -p claurst-core          # fast type-check only core
```

This is the primary risk isolation mechanism: verify each crate independently before letting changes propagate up the dependency graph.

### Use `cargo build --no-deps` for compile checking

```bash
cargo build -p claurst-core --no-deps  # build only that crate, not its deps
```

Confirms the crate compiles without rebuilding the full workspace.

### Use workspace-level `[dev-dependencies]` for test tooling

Add test-only dependencies to `[workspace.dependencies]` with `optional = false` and include them in each crate's `[dev-dependencies]` as `{ workspace = true }`. This ensures consistent versions of `insta`, `assert_cmd`, `serial_test` across all crates:

```toml
# In root Cargo.toml [workspace.dependencies]:
insta = { version = "1", features = ["json", "filters"] }
assert_cmd = "2"
serial_test = "3"
predicates = "3"
insta_cmd = "0.1"

# In each crate's Cargo.toml [dev-dependencies]:
insta = { workspace = true }
assert_cmd = { workspace = true }  # only in cli crate
```

### Use feature flags to gate incomplete refactoring

If a crate split requires an intermediate state where both old and new code paths exist, use a temporary feature flag:

```toml
[features]
refactor-commands-split = []  # temporary; removed after split is complete
```

This prevents half-done extractions from breaking the main build.

### cargo-nextest for faster feedback loops

`cargo nextest` (from nextest.dev) runs tests ~3x faster than `cargo test` by using process-per-test isolation and parallel execution. It is a drop-in replacement:

```bash
cargo install cargo-nextest
cargo nextest run -p claurst-core
```

The process isolation also catches shared global state bugs that `cargo test`'s thread-per-test model can mask — which is directly relevant to the `AGENT_RUNNER` OnceCell and `ENV_LOCK` issues documented in CONCERNS.md.

---

## Question 5: Integration Test Patterns for CLI Tools

### Pattern 1: `assert_cmd` — Process-level assertions

```toml
# crates/cli/Cargo.toml
[dev-dependencies]
assert_cmd = { workspace = true }
predicates = { workspace = true }
```

```rust
// crates/cli/tests/cli_smoke.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn version_exits_zero() {
    Command::cargo_bin("claurst").unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.0.9"));
}

#[test]
fn unknown_subcommand_exits_nonzero() {
    Command::cargo_bin("claurst").unwrap()
        .arg("notacommand")
        .assert()
        .failure();
}
```

`assert_cmd` automatically builds the binary via `cargo build` on first invocation in a test run. It finds the binary from the workspace's target directory.

### Pattern 2: `trycmd` — CLI snapshot tests from markdown/text fixtures

For commands with complex output, `trycmd` allows writing CLI tests as `.trycmd` files:

```
$ claurst --version
claurst 0.0.9
```

Stored in `crates/cli/tests/cmd/version.trycmd`. Run with:

```rust
#[test]
fn trycmd_tests() {
    trycmd::TestCases::new().case("tests/cmd/*.trycmd").run();
}
```

This is better than hardcoding expected output in test source because:
- Non-developers can read and update expected outputs
- Easy to update on version bumps: run `TRYCMD=overwrite cargo test` to regenerate

### Pattern 3: `insta_cmd` — Snapshot assertions on process output

For commands where output is large and structural (e.g., ACP JSON responses):

```rust
use insta_cmd::assert_cmd_snapshot;

#[test]
fn acp_initialize_response_shape() {
    assert_cmd_snapshot!(
        Command::cargo_bin("claurst").unwrap()
            .args(["acp"])
            .write_stdin(r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#)
    );
}
```

### Pattern 4: Headless `-p` mode for query loop characterization

The headless mode (`claurst -p "prompt"`) is the most testable code path for the query loop. For refactoring characterization, test its output format (not the content, which is non-deterministic), using env-var injection to select a mock provider:

```rust
#[test]
fn print_mode_accepts_json_output_flag() {
    Command::cargo_bin("claurst").unwrap()
        .args(["-p", "test", "--output-format", "json"])
        .env("CLAURST_MOCK_PROVIDER", "1")  // hypothetical mock-provider feature flag
        .assert()
        .success()
        .stdout(predicate::str::is_match(r#"\{"type":"result""#).unwrap());
}
```

**Note:** A `CLAURST_MOCK_PROVIDER` mode does not exist today. Adding it (a `MockProvider` that returns a canned response without HTTP) would unlock the most valuable class of integration tests. This is the highest-ROI test infrastructure investment.

---

## Component Boundaries for Safe Extraction

### What to extract from `crates/cli/src/main.rs` (3,732 lines)

| Currently in main.rs | Extract to |
|---------------------|------------|
| `McpToolWrapper` struct | `crates/tools/src/mcp_tool_wrapper.rs` |
| OAuth flow dispatch | `crates/cli/src/startup/auth.rs` |
| MCP initialization | `crates/cli/src/startup/mcp_init.rs` |
| Tool list assembly | `crates/tools/src/registry.rs` |
| Mode dispatch logic | `crates/cli/src/startup/mode_dispatch.rs` |

### What to extract from `claurst-commands/src/lib.rs` (8,657 lines)

The file mixes the `SlashCommand` trait definition with all ~40+ implementations. The clean split:

```
crates/commands/src/
├── lib.rs              (trait + registry dispatch only, ~200 lines target)
├── framework/
│   ├── mod.rs
│   ├── context.rs      (CommandContext)
│   └── result.rs       (CommandResult)
└── slash/
    ├── mod.rs           (register all commands)
    ├── help.rs
    ├── model.rs
    ├── compact.rs
    ├── clear.rs
    ├── status.rs
    ├── config.rs
    └── ... (one file per command group)
```

### What to extract from `claurst-tui/src/app.rs` (5,990 lines)

| Currently in app.rs | Extract to |
|--------------------|------------|
| Event handling match arms | `tui/src/event_handler.rs` |
| State transition logic | `tui/src/state.rs` |
| Command execution routing | `tui/src/command_router.rs` |
| Render call dispatch | `tui/src/render_dispatch.rs` (calls into existing `render.rs`) |
| `App` struct definition + constructors | Keep in `app.rs` (~500 lines target) |

---

## Architecture Anti-Patterns Specific to This Codebase

### Anti-Pattern 1: Fixing the `AGENT_RUNNER` global during refactoring

`crates/tools/src/team_tool.rs` has a `OnceCell<AgentRunFn>` that panics on double-init. Do not remove this pattern during the refactoring milestone. The fix (pass runner through `ToolContext`) requires changing the signature of `ToolContext` which would cascade through all 30+ `Tool::execute()` implementations. Defer this to a separate milestone. Instead, add a test that asserts the panic behavior is expected:

```rust
#[test]
#[should_panic(expected = "AgentRunner already registered")]
fn agent_runner_double_register_panics() {
    register_agent_runner(Box::new(|_| Box::pin(async { Ok(()) })));
    register_agent_runner(Box::new(|_| Box::pin(async { Ok(()) })));
}
```

This characterizes the current behavior without fixing it.

### Anti-Pattern 2: Refactoring and behavior-fixing in the same commit

The constraint is strictly behavior-preserving. If a refactoring reveals a latent bug (e.g., a `system_prompt.rs:572` panic path), do NOT fix it in the same PR. Log it as a separate issue. Mixing refactoring with bug fixes makes reverting impossible.

### Anti-Pattern 3: Adding dependencies during refactoring

Adding `insta`, `assert_cmd`, `serial_test`, `predicates` are the only acceptable new dependencies. Do not add `mockall`, `wiremock`, `axum` (for test servers), or any proc-macro framework during the refactoring milestone. Each new dependency risks compilation failures, feature flag conflicts, and review scope creep.

---

## Scalability Considerations for the Test Suite

| Test Class | At 100 tests | At 1,000 tests | At 10,000 tests |
|------------|-------------|----------------|-----------------|
| Unit tests (sync) | `cargo test`: fast | Still fast | Still fast |
| Unit tests (async `#[tokio::test]`) | Fast | ~2s overhead per test from runtime init | Use `cargo nextest` for parallel isolation |
| TUI snapshot tests | Review with `cargo insta review` | Same workflow | Snapshot files become a PR review burden; group by screen |
| CLI process tests | ~100ms per process spawn | ~10s for 100 tests | `cargo nextest` parallelizes spawns; budget ~5min for full suite |

**Recommendation:** Add `cargo nextest` as a workspace dev tool from Phase A. Its test isolation prevents the `ENV_LOCK` class of bugs and its parallel execution keeps the test suite fast even as it grows.

---

## Sources

- Codebase: `/Users/thamw/development/local/clearest-rust` (live analysis, 2026-05-13)
- [ratatui TestBackend + insta snapshot pattern](https://ratatui.rs/recipes/testing/snapshots/) — official ratatui docs
- [insta snapshot testing for Rust](https://insta.rs/) — official docs
- [assert_cmd crate](https://docs.rs/assert_cmd) — official docs
- [trycmd crate](https://docs.rs/trycmd) — official docs
- [Cargo workspace dependency resolution](https://doc.rust-lang.org/cargo/reference/workspaces.html) — official Cargo book
- CONCERNS.md, ARCHITECTURE.md, TESTING.md from `.planning/codebase/` (codebase audit 2026-05-05)
