# Coding Conventions

**Analysis Date:** 2026-05-05

## Naming Patterns

**Files:**
- `snake_case` for all Rust source files: `bash_classifier.rs`, `error_handling.rs`, `provider_types.rs`
- Files named after their primary type or module responsibility
- Provider implementations in subdirectory: `crates/api/src/providers/`
- No `mod.rs` barrel files in most crates; `lib.rs` serves as the crate root

**Functions:**
- `snake_case` for all functions and methods: `classify_bash_command`, `is_auto_approvable`, `fetch_flags_async`
- Predicate functions prefixed with `is_` or `has_`: `is_fork_bomb`, `has_flag`, `is_pipe_to_shell`
- Async functions suffixed with `_async` when a sync variant also exists: `fetch_flags_async`
- Private helpers end with action verbs: `split_command`, `extract_error_message`, `parse_shell_state_block`

**Variables:**
- `snake_case` everywhere: `cache_path`, `api_endpoint`, `lower`, `after_pipe`
- Single-letter iterators (`i`, `c`) acceptable in short closures
- Boolean variables use is/has prefix style: `is_network_fetch`, `writes_to_disk`, `has_r`, `has_f`

**Types (structs, enums, traits):**
- `PascalCase` for all types: `BashRiskLevel`, `FeatureFlagManager`, `ProviderError`, `RetryConfig`
- Input parameter structs named `<ToolName>Input`: `BashInput`, `ApplyPatchInput`, `MonitorInput`
- Response structs named `<Source>ApiResponse`: `GrowthBookApiResponse`
- Private structs use `PascalCase` without pub: `CachedFlags`, `Hunk`, `FilePatch`

**Constants:**
- `SCREAMING_SNAKE_CASE` for statics and constants: `SHELL_STATE_SENTINEL`, `OVERFLOW_PATTERNS`, `DEFAULT_MAX_TOKENS`
- Numeric literals use underscore separators for readability: `32_000`, `65_536`, `120_000`, `10_000_000`

**Modules:**
- `snake_case` module names: `message_utils`, `bash_classifier`, `session_storage`
- Each module corresponds to one file; no nested `mod` blocks except `#[cfg(test)] mod tests`

## Code Style

**Formatting:**
- Rust edition 2021 (`edition = "2021"` in workspace `Cargo.toml`)
- No `rustfmt.toml` found — default `rustfmt` settings apply
- 4-space indentation (Rust default)
- Trailing commas in multi-line struct/enum definitions and function args
- Long match arms formatted with explicit blocks `{ ... }` when needed

**Linting:**
- No `clippy.toml` detected — default Clippy rules apply
- `#[allow(non_snake_case)]` used sparingly in `crates/tui/src/prompt_input.rs` for UI field names that mirror JS conventions

## Section Separators

Files consistently use a distinctive comment separator style for logical groupings:

```rust
// ---------------------------------------------------------------------------
// Section Name
// ---------------------------------------------------------------------------
```

This pattern appears in nearly every file and is the standard for organizing code into named sections within a module.

## Import Organization

**Order (observed pattern):**
1. Standard library (`std::`)
2. External crates (alphabetical, matching workspace dep order)
3. Workspace crates (`claurst_core::`, `claurst_api::`, etc.)
4. Local crate (`crate::`)
5. Intra-module re-exports

**Example from `crates/api/src/error_handling.rs`:**
```rust
use std::time::Duration;

use claurst_core::provider_id::ProviderId;

use crate::provider_error::ProviderError;
```

**Path Aliases:**
- None — all imports use full module paths; no `use` aliases to rename types
- Trait imports sometimes grouped: `use async_trait::async_trait;`

**Test imports:**
- Test modules always begin with `use super::*;` to import all items from the parent module
- Additional imports added inline in the `mod tests` block

## Error Handling

**Two-tier strategy:**

1. **`anyhow::Result`** for async I/O and application-layer code (feature_flags, session_storage, API fetch):
   ```rust
   use anyhow::{anyhow, Context, Result};

   pub async fn fetch_flags_async(&self) -> Result<()> {
       // ...
       .context("Failed to fetch from GrowthBook API")?;
   }
   ```

2. **`Result<T, String>`** for pure logic/parsing functions (tools internal helpers):
   ```rust
   fn parse_unified_diff(patch: &str) -> Result<Vec<FilePatch>, String> { ... }
   fn parse_hunk_header(line: &str) -> Result<usize, String> { ... }
   fn find_cell_index(cells: &[Value], cell_id: &str) -> Result<usize, String> { ... }
   ```

3. **`thiserror`-derived custom error enums** for domain error types that cross crate boundaries:
   - `crates/api/src/provider_error.rs`: `ProviderError` enum with structured variants
   - `crates/core/src/error.rs`: `ClaudeError` domain error

**`From` trait** implemented for upcast errors across crate boundaries:
```rust
impl From<ProviderError> for ClaudeError { ... }
```

**`unwrap()` policy:** Allowed in tests and `#[cfg(test)]` blocks; in production code, `.unwrap_or_default()` and `.ok()` patterns are preferred for fallible operations where failure is non-critical.

**Error propagation:** Use `?` operator throughout; `.ok()` to silently discard errors in "best-effort" I/O (cache saves, analytics).

## Logging

**Framework:** `tracing` crate (workspace dependency)

**Import pattern:**
```rust
use tracing::{debug, warn};
```

**Patterns:**
- `debug!()` for operational flow and expected-path events
- `warn!()` for degraded-but-recoverable situations (cache misses, API failures with fallback)
- Structured key=value format with `%` (Display) and `?` (Debug) specifiers:
  ```rust
  debug!(path = %path.display(), "Editing file");
  warn!(file = ?cache_file, error = %e, "Failed to write cache file");
  ```
- `tracing` calls are never used inside `#[cfg(test)]` blocks
- No `println!` or `eprintln!` in library code; `debug!` preferred

## Comments

**File-level doc comments:**
- Every file starts with a brief `//` block comment stating file name, purpose, and what it provides:
  ```rust
  // error_handling.rs — Provider-aware error detection and retry utilities
  // (Phase 6).
  //
  // Provides:
  //  - `is_context_overflow`: ...
  ```

**Module-level doc comments (integration test files):**
- Use `//!` doc comments at the top of integration test files:
  ```rust
  //! T5-1 parity smoke tests.
  //! Verifies that core data structures are usable as the TS CLI would use them.
  ```

**Public API docs:**
- All `pub fn`, `pub struct`, `pub enum`, and `pub trait` items documented with `///`
- Doc comments include argument descriptions, return value semantics, and behavior notes
- Multi-sentence doc comments formatted with blank `///` lines between paragraphs

**Inline comments:**
- Section separators (`// ──`) used to label logical groupings within longer functions
- Implementation reasoning documented inline when non-obvious

## Function Design

**Size:** Functions kept focused; helper functions extracted with private visibility. The overall codebase has a small number of large files (`commands/src/lib.rs` at 8 576 lines) but individual functions remain bounded.

**Parameters:** Prefer `&str` over `&String`, `&[T]` over `&Vec<T>`. Tool input parameters deserialized into dedicated input structs via `serde::Deserialize`.

**Return Values:**
- Prefer `Option<T>` over sentinel values
- Return `Result<T, E>` for fallible operations; never panic in library code
- `ToolResult` (a project-specific struct) returned from all `Tool::execute()` implementations — never raw `Result`

## Module Design

**Exports:**
- `pub mod` for sub-modules; `pub use` to re-export key types at the crate root
  ```rust
  pub mod provider_id;
  pub use provider_id::{ProviderId, ModelId};
  ```
- Internal helpers are `fn` (not `pub fn`) to keep the API surface minimal

**Feature Flags:**
- Cargo feature flags used extensively in `claurst-core` and `claurst-tui` to gate experimental capabilities
- Feature names use `snake_case`: `ultraplan`, `bash_classifier`, `agent_triggers`
- A `dev_full` feature aggregates all features for local development

**Trait Objects:**
- `async_trait::async_trait` macro used on all async trait definitions
- `Arc<dyn Trait + Send + Sync>` pattern for shared handlers (permission handler, etc.)

**Derive order convention:**
```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
```
Order: `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`, `Serialize`, `Deserialize`, `Default`

---

*Convention analysis: 2026-05-05*
