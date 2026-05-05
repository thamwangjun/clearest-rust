# Testing

**Analysis Date:** 2026-05-04

## Test Strategy

The project uses **co-located unit tests** (the vast majority — 116 `#[cfg(test)]` modules, ~1,218 `#[test]` functions) alongside a small set of **integration tests** in dedicated `tests/` directories per crate. There are no E2E tests or benchmark harnesses found.

Tests are self-contained: they construct the types under test directly, use `tempfile` for filesystem isolation, and assert on observable outputs. No mocking framework is used — real implementations are exercised.

## Test Types Present

### 1. Inline Unit Tests (`#[cfg(test)] mod tests`)

The dominant pattern. Found in 116 source files. Each source module ends with:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hunk_header() {
        assert_eq!(parse_hunk_header("@@ -12,5 +12,6 @@").unwrap(), 11);
    }
}
```

Files with significant inline test coverage include:
- `crates/tools/src/apply_patch.rs` — 5 tests for patch parsing and application
- `crates/tools/src/computer_use.rs` — 16 tests for coordinate/key parsing
- `crates/tools/src/bash.rs` — 4 tests for env-var parsing and sanitisation
- `crates/tools/src/bundled_skills.rs` — 12 tests for skill text/content helpers
- `crates/tools/src/monitor_tool.rs` — 3 sync + 4 async tests for pattern matching
- `crates/core/src/session_storage.rs` — 3 async tests for session dir encoding
- `crates/core/src/team_memory_sync.rs` — 4 async tests for sync logic
- `crates/core/src/feature_flags.rs`, `keybindings.rs`, `skill_discovery.rs`, `system_prompt.rs`, `feature_gates.rs` — all have inline test modules
- `crates/core/src/lib.rs` — tests at line 3737 including `ClaudeError::is_retryable()` assertions
- `crates/mcp/src/rmcp_backend.rs` — 4 async tests for MCP backend behaviour
- `crates/plugins/src/lib.rs` — 2 async tests

### 2. Integration Tests (`crates/*/tests/`)

Located in the `tests/` directory at the crate level — Rust's standard integration test location (each file is compiled as a separate crate with access to the public API only).

**`crates/core/tests/`:**
- `parity_smoke.rs` — 7 tests verifying core data structures (`TranscriptEntry`, `FileHistory`, `HistoryEntry`, `Message`, token estimation, AGENTS.md loading). Uses `tempfile::TempDir` for filesystem tests.
- `test_mcp_templates.rs` — 6 tests for `TemplateRenderer::render()` covering substitution, nested paths, missing variables, multi-occurrence, and type coercion.

**`crates/tui/tests/`:**
- `render_snapshots.rs` — 28 tests covering every message render function (`render_assistant_text`, `render_tool_use`, `render_tool_result_success`, `render_thinking_block`, `render_rate_limit_banner`, `render_hook_progress`, `render_code_block`, `render_user_command`, `render_user_memory_input`, `render_user_local_command_output`, etc.). Tests use a `flatten()` helper to collapse `ratatui::text::Line` spans into a single `String` for assertion.
- `markdown_enhancements.rs` — table detection, multi-row tables, alignment detection, inline formatting (`bold`, `italic`, `strikethrough`, `code`) tests.
- `diff_viewer.rs` — 8 tests for `parse_unified_diff()` covering single/multi-file diffs, added/removed/context lines, hunk ranges, and stats.

### 3. Async Tests (`#[tokio::test]`)

Used wherever the function under test is `async`. Found in 19 locations across:
- `crates/tools/src/monitor_tool.rs` (4 tests)
- `crates/core/src/session_storage.rs` (3 tests)
- `crates/core/src/team_memory_sync.rs` (4 tests)
- `crates/core/src/session_tracing.rs` (1 test)
- `crates/core/src/lsp.rs` (1 test)
- `crates/plugins/src/lib.rs` (2 tests)
- `crates/mcp/src/rmcp_backend.rs` (4 tests)

Pattern:
```rust
#[tokio::test]
async fn test_session_write_read() {
    let tmp = TempDir::new().unwrap();
    // ...
}
```

## Test Coverage (estimated)

**High coverage areas:**
- `crates/tools/src/apply_patch.rs` — core patch parsing and application logic is well tested
- `crates/tools/src/computer_use.rs` — coordinate/key parsing, click/drag/type actions
- `crates/core/` data structures (error types, message utils, session storage, feature flags)
- `crates/tui/` rendering functions (render_snapshots.rs covers almost every public render function)

**Low/no coverage areas:**
- `crates/api/` — no test files found
- `crates/cli/` — no test files found; the binary entry point (`crates/cli/src/main.rs`) is untested
- `crates/query/` — no test files found
- `crates/commands/` — no test files found
- `crates/bridge/` — no test files found
- `crates/acp/` — no test files found
- `crates/buddy/` — no test files found
- Most tool implementations (file_edit, file_read, file_write, glob_tool, grep_tool, web_fetch, web_search, cron, etc.) have no unit tests

**Total test function count:** ~1,218 `#[test]` instances (dominated by inline unit tests).

## Test Utilities / Helpers

**`tempfile` crate (`tempfile::TempDir`, `tempfile::tempdir()`):**
Used in ~18 locations for filesystem-dependent tests. Pattern:
```rust
let tmp = TempDir::new().unwrap();
let path = tmp.path().join("file.txt");
std::fs::write(&path, "content").unwrap();
```

**`flatten()` helper in TUI integration tests:**
Defined locally in each TUI test file — collapses `ratatui::text::Line` spans into a `String`:
```rust
fn flatten(lines: &[ratatui::text::Line<'_>]) -> String {
    lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect()
}
```
Defined in both `crates/tui/tests/render_snapshots.rs` and `crates/tui/tests/markdown_enhancements.rs` (duplicated, not shared via a helper crate).

**`serde_json::json!` macro:**
Used in MCP template integration tests for constructing `Value` contexts inline:
```rust
let context = json!({ "name": "Database", "description": "Query operations" });
```

**`use super::*`:**
Standard import in every inline `mod tests` block — pulls all items from the parent module into scope without needing to re-qualify.

**No mock framework** (e.g., `mockall`, `mockito`) is used anywhere in the workspace. Tests rely on real implementations or construct minimal in-memory state.

## CI / Quality Gates

**No `.github/workflows/` directory found.** There is no CI pipeline configuration in this repository.

**No `cargo-tarpaulin` or coverage tooling** configured.

**Run commands:**
```bash
cargo test                        # Run all tests in the workspace
cargo test -p claurst-core        # Run tests for a specific crate
cargo test -p claurst-tools       # Run tools tests only
cargo test -p claurst-tui         # Run TUI integration tests
cargo test -- --nocapture         # Show println! output during tests
```

**Formatting / linting (manual):**
```bash
cargo fmt                         # Format (uses default rustfmt settings; no rustfmt.toml)
cargo clippy                      # Lint (no clippy.toml; default lints + inline allows)
```

No coverage target, no required passing gates, no enforced test-before-merge policy is detectable from the repository configuration alone.
