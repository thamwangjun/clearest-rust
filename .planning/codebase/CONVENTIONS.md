# Code Conventions

**Analysis Date:** 2026-05-04

## Naming Conventions

**Crates:** `kebab-case` with `claurst-` prefix in `Cargo.toml` names (e.g., `claurst-core`, `claurst-tools`), but `snake_case` for Rust module paths (e.g., `claurst_core`).

**Files:** `snake_case.rs` — always. No exceptions observed. Examples: `session_storage.rs`, `team_memory_sync.rs`, `prompt_history.rs`.

**Structs / Enums:** `PascalCase` throughout.
- `ClaudeError`, `ToolResult`, `ToolContext`, `MessageContent`, `ProviderConfig`
- Error enum variants also `PascalCase`: `PermissionDenied`, `RateLimit`, `ContextWindowExceeded`

**Traits:** `PascalCase`. Examples: `Tool`, `PermissionHandler`.

**Functions / Methods:** `snake_case`. Constructor helpers use descriptive names rather than `new` when there are multiple variants: `user()`, `assistant()`, `user_blocks()`, `user_local_command_output()`.

**Constants / Statics:** `SCREAMING_SNAKE_CASE` for statics. Examples: `REPL_SESSIONS`, `CRON_STORE`, `ACTIVE_TEAMS`, `SUPPORTED_SETTINGS`.

**Type Aliases:** `PascalCase`. The project-wide result alias is:
```rust
pub type Result<T> = std::result::Result<T, ClaudeError>;
```
Located in `crates/core/src/lib.rs` (the `error` submodule, re-exported at crate root).

**Modules:** `snake_case`. Submodules within a single large file use nested `pub mod name { }` blocks (see `crates/core/src/lib.rs` — `error`, `types`, `config`, etc. are all inline modules in one file).

## Error Handling Strategy

**Primary error type:** `ClaudeError` — a `thiserror`-derived enum defined in `crates/core/src/lib.rs` (inside `pub mod error`), re-exported as `claurst_core::error::ClaudeError` and `claurst_core::ClaudeError`.

```rust
#[derive(Error, Debug)]
pub enum ClaudeError {
    #[error("API error: {0}")]         Api(String),
    #[error("API error {status}: {message}")] ApiStatus { status: u16, message: String },
    #[error("Authentication error: {0}")] Auth(String),
    #[error("Permission denied: {0}")] PermissionDenied(String),
    #[error("Tool error: {0}")]        Tool(String),
    #[error("IO error: {0}")]          Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]        Json(#[from] serde_json::Error),
    #[error("HTTP error: {0}")]        Http(#[from] reqwest::Error),
    #[error("Rate limit exceeded")]    RateLimit,
    #[error("Context window exceeded")] ContextWindowExceeded,
    #[error("Max tokens reached")]     MaxTokensReached,
    #[error("Cancelled")]              Cancelled,
    #[error("Configuration error: {0}")] Config(String),
    #[error("MCP error: {0}")]         Mcp(String),
    #[error("{0}")]                    Other(String),
}
```

**Helpers on the error type:** `is_retryable()` and `is_context_limit()` are impl'd directly on `ClaudeError` (`crates/core/src/lib.rs:151-170`).

**Tool-level errors:** The `Tool` trait's `execute()` returns `ToolResult` (not `Result<_, _>`). Errors are signalled via `ToolResult::error(msg)` — a plain struct with `is_error: bool` and `content: String`. Individual tool helper functions use `Result<T, String>` (string errors, not `ClaudeError`), with `map_err(|e| e.to_string())` or `map_err(|e| format!("...: {}", e))` at the boundary.

**Propagation pattern inside tools:**
```rust
// Convert early and return ToolResult::error at the tool execute() boundary
let val = serde_json::from_value(input)
    .map_err(|e| format!("Invalid input: {}", e))?;  // inside helper returning Result<_, String>
// ...
Err(e) => return ToolResult::error(format!("Failed to spawn command: {}", e)),
```

**`anyhow` and `thiserror`** are both workspace dependencies. `thiserror` is used for the canonical `ClaudeError` enum. `anyhow` is declared as a dependency in multiple crates but usage is sparse — the project favours the typed `ClaudeError` at boundaries exposed to callers.

**No `unwrap()` on production paths** — only in tests where `.unwrap()` is acceptable to surface panics.

## Common Patterns

**`Tool` trait (object-safe async trait):**
Every tool implements the `Tool` trait from `crates/tools/src/lib.rs`:
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn permission_level(&self) -> PermissionLevel;
    fn input_schema(&self) -> Value;
    async fn execute(&self, input: Value, ctx: &ToolContext) -> ToolResult;
    fn to_definition(&self) -> ToolDefinition { /* default */ }
}
```
Tool structs are zero-sized (`struct BashTool;`) and are boxed into `Box<dyn Tool>`.

**`ToolResult` constructor helpers:**
```rust
ToolResult::success("output text")
ToolResult::error("error message")
```

**Global singletons via `once_cell::sync::Lazy`:**
Shared state (REPL sessions, cron store, message inbox, agent runners) is stored in module-level statics:
```rust
static CRON_STORE: Lazy<Arc<RwLock<HashMap<String, CronTask>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));
```
`DashMap` is used for concurrent maps without locking overhead.

**Builder pattern:** Used for complex configs. Example: `ContextBuilder` (`crates/core/src/lib.rs:1736`). Field structs implement `Default`, and struct-update syntax (`..Default::default()`) is used for partial overrides.

**Serde conventions:** API types derive `Serialize, Deserialize`. Tagged enums use `#[serde(tag = "type", rename_all = "snake_case")]`. Optional fields use `#[serde(skip_serializing_if = "Option::is_none")]`. Field renames use `#[serde(rename = "camelCase")]` for wire compatibility.

**`Arc<T>` sharing:** `Arc` is pervasive for shared ownership across async task boundaries (333 usages). `parking_lot::Mutex`/`RwLock` used inside `Arc` for mutation.

**Feature flags:** Tools that require platform capabilities (e.g., computer-use screenshot tools) are gated with `#[cfg(feature = "computer-use")]`.

**`impl From<X> for Y`:** Used to convert between serialized/deserialized forms. Example: `From<&PermissionRule> for SerializedPermissionRule`.

## Code Style / Formatting

**No `rustfmt.toml` or `.rustfmt.toml` found** — default `rustfmt` settings are in effect. Run `cargo fmt` to format.

**No `clippy.toml` found.** Clippy is used but configured only at the attribute level:
- `#[allow(clippy::too_many_arguments)]` appears on several functions in `crates/tui/src/prompt_input.rs` (lines 543, 834, 919).
- Total `allow(clippy::*)` suppressions: ~39 occurrences across the workspace.

**Section separators** in large files use dashed comment lines:
```rust
// ---------------------------------------------------------------------------
// Section Name
// ---------------------------------------------------------------------------
```
Observed consistently in `crates/core/src/lib.rs` and `crates/tools/src/lib.rs`.

**Imports:** Grouped by standard library, then third-party, then workspace crates — no enforced tool found, but the pattern is consistent in most files. Re-exports from submodules are centralised at the crate root (`pub use submod::Type;` in `lib.rs`).

## Documentation Conventions

**Crate-level docs:** `//!` module doc comments appear at the top of files. Example: `crates/tui/tests/render_snapshots.rs` starts with `//! T5-2: Message renderer snapshot tests.`

**Public items:** Documented with `/// ` triple-slash doc comments. All public trait methods, structs, and enums in `crates/core/src/lib.rs` have doc comments. Total doc comment lines: ~2,888 across the workspace.

**Inline comments:** Use `//` with a space. Multi-line prose explanations precede complex logic rather than being mixed in.

**Variant documentation on enums:** Each variant of `MessageContent`, `ContentBlock`, and `ClaudeError` has a doc comment explaining its semantics and render behaviour.

**`TODO`/`FIXME`:** Not systematically tracked; use grep to audit.
