# Testing Patterns

**Analysis Date:** 2026-05-05

## Test Framework

**Runner:**
- Built-in Rust test harness (`cargo test`)
- No third-party test runner (no `nextest.toml` or `nextest` config detected)
- Async tests use `#[tokio::test]` from the `tokio` workspace dependency

**Assertion Library:**
- Built-in `assert!`, `assert_eq!`, `assert!(..., "message")` macros only
- No `rstest`, `proptest`, `mockall`, `insta`, or `criterion` detected in any `Cargo.toml`

**Run Commands:**
```bash
cargo test                        # Run all tests in workspace
cargo test -p claurst-core        # Run tests for a single crate
cargo test -p claurst-tui         # Run TUI integration tests
cargo test -- --test-output immediate  # Show output as it happens
cargo test <test_name>            # Run a specific test by name
```

## Test File Organization

**Location:**
- **Unit tests:** Co-located in the same source file using `#[cfg(test)] mod tests { ... }` — the dominant pattern
- **Integration tests:** Separate `tests/` directories at the crate root for `claurst-core` and `claurst-tui`

**Crates with integration test directories:**
- `crates/core/tests/` — `parity_smoke.rs`, `test_mcp_templates.rs`
- `crates/tui/tests/` — `diff_viewer.rs`, `markdown_enhancements.rs`, `render_snapshots.rs`

**Naming:**
- Unit test modules are always named `mod tests`
- Integration test files named after the feature or module being tested: `diff_viewer.rs`, `render_snapshots.rs`
- Test functions use `snake_case` with descriptive names: `test_safe_commands`, `parse_diff_returns_hunks`, `assistant_text_renders_lines`
- Parity/smoke tests prefixed `test_` or named as assertions: `session_dir_encoding`, `file_history_record_and_query`

**Structure:**
```
crates/
├── core/
│   ├── src/
│   │   ├── bash_classifier.rs      ← #[cfg(test)] mod tests inside
│   │   ├── error_handling.rs       ← #[cfg(test)] mod tests inside
│   │   ├── feature_flags.rs        ← #[cfg(test)] mod tests inside
│   │   └── ...
│   └── tests/
│       ├── parity_smoke.rs         ← integration tests
│       └── test_mcp_templates.rs   ← integration tests
├── tui/
│   ├── src/
│   │   ├── stats_dialog.rs         ← #[cfg(test)] mod tests inside
│   │   └── ...
│   └── tests/
│       ├── diff_viewer.rs          ← integration tests
│       ├── markdown_enhancements.rs
│       └── render_snapshots.rs
└── tools/
    └── src/
        ├── bash.rs                 ← #[cfg(test)] mod tests inside
        ├── apply_patch.rs          ← #[cfg(test)] mod tests inside
        ├── computer_use.rs         ← #[cfg(test)] mod tests inside
        └── ...
```

## Test Structure

**Unit test module boilerplate:**
```rust
// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_commands() {
        assert_eq!(classify_bash_command("ls -la"), BashRiskLevel::Safe);
        assert_eq!(classify_bash_command("grep foo bar.txt"), BashRiskLevel::Safe);
    }

    #[test]
    fn test_medium_commands() {
        assert_eq!(classify_bash_command("rm -r ./build"), BashRiskLevel::Medium);
    }
}
```

**Integration test boilerplate:**
```rust
//! T5-1 parity smoke tests.
//! Verifies that core data structures are usable as the TS CLI would use them.

use claurst_core::{
    session_storage::{TranscriptEntry, transcript_dir},
    message_utils::{estimate_tokens, get_message_text},
};
use tempfile::TempDir;

#[test]
fn session_dir_encoding() {
    let root = PathBuf::from("/home/user/project");
    let dir = transcript_dir(&root);
    assert!(dir.to_string_lossy().contains("projects"));
}
```

**Patterns:**
- Unit test modules always placed at the bottom of the source file, after all production code
- `use super::*;` as the first statement in every `mod tests` block
- Section divider comments (`// ---`) used within large test modules to group related tests
- Test names are descriptive English phrases: `test_cache_path`, `parse_diff_has_added_lines`, `thinking_block_collapsed`

## Async Tests

**Pattern:** `#[tokio::test]` for async tests (45 async tests vs 1 221 sync tests across the workspace):

```rust
#[tokio::test]
async fn monitor_list_empty() {
    let tool = MonitorTool;
    let input = json!({"action": "list"});
    let ctx = make_test_ctx();
    let result = tool.execute(input, &ctx).await;
    assert!(!result.is_error, "list action should not return an error: {}", result.content);
}
```

Async tests appear primarily in `crates/tools/src/monitor_tool.rs` for tests that exercise `async fn execute()` on tool implementations.

## Mocking

**Framework:** None — no `mockall`, `mockito`, or similar frameworks used.

**Patterns for isolation:**
- **`tempfile::TempDir`** for filesystem isolation in integration tests:
  ```rust
  let tmp = TempDir::new().unwrap();
  let files = load_all_memory_files(tmp.path());
  ```
- **Inline construction** of test contexts using `make_test_ctx()` helper functions defined within the test module:
  ```rust
  fn make_test_ctx() -> ToolContext {
      let handler = Arc::new(AutoPermissionHandler {
          mode: claurst_core::config::PermissionMode::Default,
      });
      ToolContext {
          working_dir: PathBuf::from("."),
          permission_mode: claurst_core::config::PermissionMode::Default,
          permission_handler: handler,
          cost_tracker: claurst_core::cost::CostTracker::new(),
          session_id: "test-monitor".to_string(),
          // ...
      }
  }
  ```
- **`serde_json::json!` macro** to construct tool inputs without a full builder:
  ```rust
  let input = json!({"action": "list"});
  let input = json!({"action": "status"});
  ```
- **Direct struct construction** for domain types:
  ```rust
  let msg = Message {
      role: Role::User,
      content: MessageContent::Text("hello world".to_string()),
      uuid: None,
      cost: None,
  };
  ```

**What to Mock:**
- File system access: use `TempDir` from `tempfile` crate
- Tool contexts: construct `ToolContext` manually with `AutoPermissionHandler`

**What NOT to Mock:**
- HTTP calls — tests avoid exercising network code paths; async fetch logic is tested only at the unit level (e.g., `test_flag_default_false` does not call the API)
- Database access — `TempDir` + real SQLite used when needed

## Fixtures and Factories

**Test Data:**
- Multi-line string constants for parser tests:
  ```rust
  const SIMPLE_DIFF: &str = r#"diff --git a/src/main.rs b/src/main.rs
  --- a/src/main.rs
  +++ b/src/main.rs
  @@ -1,5 +1,6 @@
   fn main() {
  -    println!("hello");
  +    println!("hello, world");
   }
  "#;
  ```
- `serde_json::json!` macro for structured data:
  ```rust
  let context = json!({
      "name": "Database",
      "description": "Query operations"
  });
  ```

**Location:**
- Fixtures defined as `const` or `let` bindings within each test function or at the top of the test module
- No shared fixture files (`fixtures/`, `testdata/`) detected
- `tempfile::TempDir` used as a factory for temporary filesystem state

## Coverage

**Requirements:** None enforced — no coverage threshold configuration detected in any `Cargo.toml` or CI config.

**View Coverage:**
```bash
# Using cargo-llvm-cov (if installed):
cargo llvm-cov --workspace

# Using grcov (alternative):
RUSTFLAGS="-C instrument-coverage" cargo test
```

No coverage tooling is configured in the workspace; adding it would require separate setup.

## Test Types

**Unit Tests:**
- Scope: Pure functions and deterministic logic — risk classifiers, parsers, formatters, cache validity checks
- Location: `#[cfg(test)] mod tests` block at bottom of each source file
- Approach: Call the function under test directly, assert on return value with `assert_eq!` / `assert!`
- Count: 1 221 sync + 45 async across the workspace

**Integration Tests:**
- Scope: Cross-module public API surface — message rendering, diff parsing, core data structure round-trips
- Location: `crates/core/tests/` and `crates/tui/tests/`
- Approach: Import public types from the crate, construct realistic inputs, assert on observable behavior
- Example crates with integration tests: `claurst-core`, `claurst-tui`

**E2E Tests:**
- Not used — no end-to-end or UI automation framework detected

## Common Patterns

**Assertion style — prefer descriptive failure messages:**
```rust
// Preferred: include context in assertion failure messages
assert!(!result.is_error, "list action should not return an error: {}", result.content);
assert!(has_hunks, "should have at least one hunk");

// Also common: simple equality without message for obvious assertions
assert_eq!(classify_bash_command("ls -la"), BashRiskLevel::Safe);
```

**Flatten helper for TUI span content:**
Integration tests in `crates/tui/tests/` define a local `flatten()` helper to extract text from rendered `ratatui::text::Line` spans:
```rust
fn flatten(lines: &[ratatui::text::Line<'_>]) -> String {
    lines
        .iter()
        .flat_map(|l| l.spans.iter().map(|s| s.content.as_ref()))
        .collect()
}
```
This helper is duplicated across test files (not shared); copy it when adding new TUI tests.

**Unwrap in tests:**
`unwrap()` and `expect()` are acceptable in test code. Use `.unwrap()` for values that must be present (a missing value is a test failure). Use `expect("description")` when the failure message needs context:
```rust
let (table, _) = detect_table(&lines, 0).expect("Table should be detected");
let json = serde_json::to_string(&entry).unwrap();
```

**Error testing:**
```rust
#[test]
fn test_parse_error_response_auth() {
    let pid = ProviderId::new("anthropic");
    let err = parse_error_response(401, r#"{"error":{"message":"Invalid API key"}}"#, &pid);
    assert!(matches!(err, ProviderError::AuthFailed { .. }));
}
```
Use `matches!` macro for enum variant matching without binding fields.

**Boundary / edge case coverage pattern:**
Tests are grouped by risk or classification level — each group covers one representative scenario per class:
```rust
#[test] fn test_safe_commands() { ... }     // one test per level
#[test] fn test_low_commands() { ... }
#[test] fn test_medium_commands() { ... }
#[test] fn test_high_commands() { ... }
#[test] fn test_critical_commands() { ... }
```

**Serialization round-trip pattern:**
```rust
#[test]
fn history_entry_roundtrip() {
    let entry = HistoryEntry { display: "test prompt".to_string(), ... };
    let json = serde_json::to_string(&entry).unwrap();
    let decoded: HistoryEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.display, "test prompt");
}
```

---

*Testing analysis: 2026-05-05*
