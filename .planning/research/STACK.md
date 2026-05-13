# Technology Stack Research: claurst Milestone

**Project:** claurst — Rust rewrite of Claude Code
**Researched:** 2026-05-04 (original); refactoring toolchain section added 2026-05-13
**Scope:** Crates and patterns to adopt/avoid for missing features; existing codebase analyzed

---

## [NEW] v1.1 Refactoring Toolchain

This section covers tools needed for the systematic code smell refactoring milestone.
The milestone requires three capability types: (1) detecting smells via static analysis,
(2) anchoring behavior with characterization tests before any code moves, and
(3) validating that tests actually cover what they claim to cover.

### Overview: Essential vs Nice-to-Have

| Category | Tool | Version | Essential? |
|----------|------|---------|-----------|
| Lint configuration | `clippy.toml` thresholds (built-in) | — | **Essential** |
| Snapshot/characterization tests | `insta` | 1.47 | **Essential** |
| Test runner (workspace-aware) | `cargo-nextest` | 0.9.116 | **Essential** |
| Coverage (baseline + regression gate) | `cargo-llvm-cov` | latest via brew/cargo | **Essential** |
| Property-based tests | `proptest` + `proptest-derive` | 1.9.0 / 0.8.0 | Nice-to-have |
| Dead dependency detection | `cargo-machete` | 0.9.2 | Nice-to-have |
| Mutation testing | `cargo-mutants` | 27.0.0 | Nice-to-have |
| Unused dep detection (accurate) | `cargo-udeps` | latest (nightly required) | Skip for now |

---

### Tool 1: `clippy.toml` — Code Smell Detection (Essential)

Clippy is already in the workspace. The refactoring work needs it **configured** to
surface the Fowler smells, not just run at defaults. Defaults are too permissive for this
purpose (e.g., `cognitive-complexity-threshold = 25` passes most problematic functions).

**What to configure in `clippy.toml` at the workspace root:**

```toml
# Bloater detection
cognitive-complexity-threshold = 15   # default 25; catches real multi-concern functions
too-many-arguments-threshold = 6      # default 7; nudges toward builder/config-struct patterns
too-many-lines-threshold = 80         # default 100; flags long methods for extraction

# Primitive obsession / type complexity
type-complexity-threshold = 200       # default 250; surface overly-complex type expressions

# Structural smells (via Cargo.toml [lints.clippy] section)
# These must be enabled because they are allow-by-default:
#   pedantic = "warn"
#   clippy::struct_excessive_bools — too many bool fields (data clumps smell)
#   clippy::fn_params_excessive_bools — bool params instead of enums
#   clippy::wildcard_imports — hides dependencies, makes Inappropriate Intimacy hard to spot
#   clippy::must_use_candidate — surfaces ignored return values
```

**Clippy lint groups and their mapping to Fowler smells:**

| Clippy Group | Enabled by Default | Fowler Smells Detected |
|---|---|---|
| `clippy::complexity` | warn | Long Method (cognitive complexity, nesting) |
| `clippy::pedantic` | allow (must enable) | Primitive Obsession, Feature Envy, Message Chains |
| `clippy::suspicious` | warn | wrong-looking code that typically signals Divergent Change |
| `clippy::nursery` | allow (experimental) | optional; catches some Dispensable patterns |
| `clippy::restriction` | allow (cherry-pick only) | unwrap-heavy code, dead code patterns |

**How to enable in `Cargo.toml` workspace lints section:**

```toml
[workspace.lints.clippy]
pedantic = "warn"          # enables the full pedantic group
# Individual restriction cherry-picks:
unwrap_used = "warn"       # surfaces where panics hide missing error paths
expect_used = "warn"       # same; prefer anyhow/thiserror patterns
wildcard_imports = "warn"  # smells like Feature Envy or Inappropriate Intimacy
```

**Note on false positives:** After enabling pedantic, expect 50–200 new warnings on first
run across 12 crates. The workflow is: run clippy, triage per-crate, add targeted
`#[allow(clippy::...)]` with justification comments for intentional patterns. Do NOT
suppress wholesale with `#![allow(clippy::pedantic)]` at crate root — that defeats the
purpose.

**Confidence: HIGH** — Clippy configuration is documented at
https://doc.rust-lang.org/clippy/lint_configuration.html; all thresholds and group names
verified against the live Clippy docs.

---

### Tool 2: `insta` — Snapshot/Characterization Tests (Essential)

**Version:** `1.47` (verified via Context7 / mitsuhiko/insta)

**Why essential for refactoring:** Characterization tests must capture *current* behavior
before any code moves. `insta` stores output in `.snap` files committed to git. After a
refactor, `cargo insta test --check` (used in CI) fails if behavior changed, even subtly.
This is the exact semantics needed for safe refactoring.

**Cargo.toml addition:**

```toml
[dev-dependencies]
insta = { version = "1.47", features = ["yaml", "json", "redactions", "filters"] }

# Speed optimization (insta's diff engine is slow in debug mode)
[profile.dev.package.insta]
opt-level = 3
[profile.dev.package.similar]
opt-level = 3
```

**Install the companion CLI:**

```bash
cargo install cargo-insta --locked
```

**Workflow for characterization tests:**

```bash
# 1. Write tests using assert_snapshot! with no existing .snap files
# 2. Run once to generate snapshots from current (possibly smelly) behavior:
cargo insta test --accept

# 3. Commit the .snap files — these are the behavior anchors

# 4. During refactoring, CI runs:
cargo insta test --check     # fails if any snapshot changed
```

**What to snapshot for this codebase:**

- CLI command output (stdout/stderr for each command path in `crates/cli`)
- Serialized structs that cross crate boundaries (serde Debug/Display output)
- TUI render outputs for key screens (ratatui Buffer snapshots)
- Error messages from `anyhow`/`thiserror` chains

**Integration with cargo test:** `insta` tests are regular `#[test]` functions using
`assert_snapshot!` / `assert_yaml_snapshot!` / `assert_json_snapshot!`. They run
with `cargo test` or `cargo nextest run` without any special flags.

**Confidence: HIGH** — verified via Context7 docs (/mitsuhiko/insta).

---

### Tool 3: `cargo-nextest` — Test Runner (Essential)

**Version:** `0.9.116` (released ~3 days before this research, 2026-05-10)

**Why essential for refactoring:** This workspace has 12 crates and 1,227 existing tests.
During refactoring, tests will run frequently (after each extraction). `cargo-nextest`
runs each test in its own process (no shared state), reports per-crate results cleanly,
and runs ~3x faster than `cargo test` on large workspaces.

The process-per-test model also catches a class of bugs that are invisible to the
shared-process `cargo test` runner: global state leaks between tests, which are common
in AI-generated code.

**Install:**

```bash
cargo install cargo-nextest --locked
```

**Usage:**

```bash
cargo nextest run --workspace          # run all tests across all 12 crates
cargo nextest run -p claurst-core      # single crate during focused refactoring
cargo nextest run --test-threads 8     # control parallelism
```

**Integration with insta:**

```toml
# .config/nextest.toml
[profile.default]
# insta requires INSTA_UPDATE env var to accept; nextest passes it through
```

**Note:** `serial_test` (already used in the codebase for bearer auth tests) works with
nextest — the `#[serial]` attribute is honored.

**Confidence: HIGH** — version from crates.io search result; usage patterns from
official nextest docs (https://nexte.st/).

---

### Tool 4: `cargo-llvm-cov` — Coverage (Essential)

**Why essential for refactoring:** Before refactoring begins, establish a coverage
baseline. After refactoring, coverage must not decrease. This is the mechanical
enforcement of "characterization tests cover the code we're moving."

**LLVM source-based coverage is the correct choice over cargo-tarpaulin:**
- tarpaulin is Linux-only (ptrace-based); this repo is developed on macOS (darwin,
  per env context). cargo-llvm-cov works on macOS, Linux, and Windows.
- LLVM coverage tracks at region granularity (not just line), catching cases where
  one branch of a complex expression is never exercised.

**Install (macOS, as per env):**

```bash
brew install taiki-e/tap/cargo-llvm-cov
# OR
cargo +stable install cargo-llvm-cov --locked  # requires rustc 1.87+
```

**Usage:**

```bash
# Generate HTML report for human review
cargo llvm-cov --workspace --html

# Generate LCOV for CI coverage gate
cargo llvm-cov --workspace --lcov --output-path lcov.info

# Run only for a specific crate during focused refactoring
cargo llvm-cov -p claurst-core --html
```

**Recommended workflow:**

1. Before refactoring: `cargo llvm-cov --workspace --json > coverage-baseline.json`
2. Write characterization tests until coverage reaches acceptable level per crate
3. During refactoring: `cargo llvm-cov --workspace --fail-under-lines 70` in CI

**Confidence: HIGH** — verified via Context7 docs (/taiki-e/cargo-llvm-cov); platform
restriction for tarpaulin verified via community comparison.

---

### Tool 5: `proptest` + `proptest-derive` — Property-Based Tests (Nice-to-Have)

**Versions:** `proptest = "1.9.0"`, `proptest-derive = "0.8.0"`

**Why useful for refactoring:** Property tests are superior to example-based
characterization tests for catching edge cases when *refactoring pure functions*.
When extracting a function from a bloated method, write a property test asserting
the invariants (not the specific output). This survives implementation changes while
still catching regressions.

**When to use:**
- Extracting pure functions from bloated methods (parsers, transformers, validators)
- Testing `LlmProvider` request/response serialization contracts
- Verifying that refactored `PermissionManager` logic preserves authorization invariants

**When NOT to use:**
- Testing I/O-heavy async code (use insta snapshots instead)
- Testing TUI rendering (use insta buffer snapshots instead)
- As a replacement for characterization tests (write those first with insta)

**Cargo.toml addition:**

```toml
[dev-dependencies]
proptest = "1.9.0"
proptest-derive = "0.8.0"   # enables #[derive(Arbitrary)] on structs/enums
```

**Basic usage pattern:**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn parse_roundtrip(s in "\\PC*") {
        // extracted function must roundtrip
        let parsed = parse_something(&s);
        prop_assert_eq!(format_something(parsed), s);
    }
}
```

**Async note:** Use `test-strategy` crate (not included here) if async proptest tests
are needed — it supports `#[proptest(async = "tokio")]`. For this milestone the pure
function extractions will be sync; skip `test-strategy` for now.

**Confidence: HIGH for proptest** — version verified via Context7 (/proptest-rs/proptest)
and search results showing 1.9.0 as current. **MEDIUM for proptest-derive** — 0.8.0 is
current per Context7; versioning is independent of main proptest crate and still
experimental per project's own documentation.

---

### Tool 6: `cargo-machete` — Unused Dependency Detection (Nice-to-Have)

**Version:** `0.9.2` (released 2026-04-15)

**Why useful for refactoring:** The Dispensable smell category includes speculative
dependencies added "just in case" during AI-written code generation. `cargo-machete`
finds crates listed in `Cargo.toml` that are never referenced in source.

**Install:**

```bash
cargo install cargo-machete --locked
```

**Usage:**

```bash
cargo machete             # scan workspace for unused deps
cargo machete --with-metadata  # use cargo metadata for more accurate detection
```

**Trade-off to understand:** cargo-machete uses text search, not compiler analysis.
Dependencies used only through proc-macros (e.g., `serde_derive` invoked via
`#[derive(Serialize)]`) show as false positives. Suppress known false positives in
`Cargo.toml`:

```toml
[package.metadata.cargo-machete]
ignored = ["serde_derive", "tokio-macros"]
```

**Alternative: `cargo-udeps`** is more accurate (uses compiler output) but requires
nightly and is significantly slower. Skip it for this milestone; cargo-machete's
speed (runs in ~1 second on large workspaces) makes it practical as a refactoring
sweep tool.

**Confidence: HIGH** — version from search results; behavior documented at GitHub
(https://github.com/bnjbvr/cargo-machete).

---

### Tool 7: `cargo-mutants` — Mutation Testing (Nice-to-Have, Defer to Later Phase)

**Version:** `27.0.0` (released 2026-03-07)

**Why useful:** Mutation testing answers "do the characterization tests actually catch
bugs, or do they just run?" It injects small bugs (swap operators, return defaults)
and verifies tests catch them. A "surviving mutant" means a test exists but doesn't
actually constrain behavior.

**Why to defer:** Mutation testing is slow — it recompiles and reruns the test suite
once per mutant, and a 12-crate workspace with 1,227 tests will generate thousands of
mutants. It is most valuable *after* the characterization test suite is written and
stable, as a quality gate on the tests themselves.

**Recommended workflow (later phase):**

```bash
cargo install cargo-mutants --locked

# Run on a single crate to limit scope
cargo mutants -p claurst-core --timeout 60

# Or limit to recently-changed files during active refactoring
cargo mutants --diff HEAD~1
```

**Zero configuration required** — cargo-mutants needs no source tree changes.

**Confidence: HIGH** — version verified via GitHub releases page (v27.0.0, 2026-03-07);
tool listed on Thoughtworks Technology Radar as "Adopt."

---

### Tools to Skip

| Tool | Why Skip |
|------|---------|
| `cargo-tarpaulin` | Linux-only (ptrace). Development env is macOS. Use cargo-llvm-cov instead. |
| `cargo-udeps` | Requires nightly toolchain. cargo-machete is sufficient for the dispensable smell sweep; nightly adds operational friction to a 12-crate workspace. |
| `quickcheck` | Proptest supersedes it. Proptest has better shrinking, richer strategy combinators, and derive macro support. The existing test suite doesn't use quickcheck; no reason to introduce it. |
| `arbitrary` (standalone) | The `arbitrary` crate is the fuzzer-integration trait; `proptest-derive` covers the same derive-based test input generation without requiring a fuzzer harness. Use proptest instead. |
| `criterion` | Benchmarking, not refactoring. Out of scope for this milestone. |

---

### Integration: How These Tools Work Together

The recommended sequence for each crate being refactored:

```
1. cargo clippy --workspace 2>&1 | grep "crate-name"
   → Identify smells in the crate (long functions, complex types, bool params)

2. cargo llvm-cov -p crate-name --html
   → Establish coverage baseline; identify uncovered code paths

3. Write insta snapshot tests for uncovered paths until coverage is acceptable
   cargo insta test --accept  (first run to generate .snap files)

4. cargo machete              → Remove unused deps found during step 1

5. Perform extractions/splits guided by clippy findings

6. cargo insta test --check   → Verify no behavioral change
   cargo llvm-cov -p crate-name --fail-under-lines <baseline>

7. (Later) cargo mutants -p crate-name
   → Verify characterization tests actually constrain behavior
```

---

### Cargo.toml Changes Summary

**Add to workspace `[dev-dependencies]`:**

```toml
[dev-dependencies]
insta = { version = "1.47", features = ["yaml", "json", "redactions", "filters"] }
proptest = "1.9.0"
proptest-derive = "0.8.0"
```

**Add profile optimizations to workspace `Cargo.toml`:**

```toml
[profile.dev.package.insta]
opt-level = 3
[profile.dev.package.similar]
opt-level = 3
```

**Add to workspace `Cargo.toml` lints section:**

```toml
[workspace.lints.clippy]
pedantic = "warn"
unwrap_used = "warn"
expect_used = "warn"
wildcard_imports = "warn"
```

**Add `clippy.toml` at workspace root (new file):**

```toml
cognitive-complexity-threshold = 15
too-many-arguments-threshold = 6
too-many-lines-threshold = 80
type-complexity-threshold = 200
```

**Install as cargo extensions (not Cargo.toml — these are CLI tools):**

```bash
cargo install cargo-nextest --locked
cargo install cargo-insta --locked
brew install taiki-e/tap/cargo-llvm-cov  # macOS
cargo install cargo-machete --locked
cargo install cargo-mutants --locked     # defer to later phase
```

---

### Confidence Assessment

| Tool | Confidence | Basis |
|------|------------|-------|
| clippy.toml thresholds | HIGH | Official docs verified; all config keys confirmed |
| insta 1.47 | HIGH | Context7 docs; version current |
| cargo-nextest 0.9.116 | HIGH | crates.io search confirmed; 3 days old |
| cargo-llvm-cov | HIGH | Context7 docs; macOS confirmed via brew tap |
| proptest 1.9.0 | HIGH | Context7 docs + search results; version current |
| proptest-derive 0.8.0 | MEDIUM | Context7 docs; project notes it's still "experimental" versioning |
| cargo-machete 0.9.2 | HIGH | GitHub releases + crates.io search |
| cargo-mutants 27.0.0 | HIGH | GitHub releases page confirmed; Thoughtworks Radar "Adopt" |

---

### Sources (Refactoring Toolchain)

- [Clippy Lint Configuration — official docs](https://doc.rust-lang.org/clippy/lint_configuration.html)
- [Clippy Lints Index](https://rust-lang.github.io/rust-clippy/master/index.html)
- Context7 `/mitsuhiko/insta` — insta 1.47 docs
- Context7 `/proptest-rs/proptest` — proptest 1.9.0 / proptest-derive 0.8.0 docs
- Context7 `/taiki-e/cargo-llvm-cov` — cargo-llvm-cov installation and usage
- [cargo-nextest home](https://nexte.st/) — 0.9.116 current
- [cargo-mutants GitHub releases](https://github.com/sourcefrog/cargo-mutants/releases) — v27.0.0
- [cargo-machete GitHub](https://github.com/bnjbvr/cargo-machete) — 0.9.2
- [cargo-tarpaulin vs cargo-llvm-cov comparison](https://rustprojectprimer.com/measure/coverage.html) — macOS support confirmed
- [Snapshot Testing Rust with cargo-insta](https://www.mutorium.com/blog/cargo-insta-snapshot-testing/)
- [cargo-mutants Thoughtworks Radar](https://www.thoughtworks.com/radar/tools/cargo-mutants)

---

## Existing Stack (Do Not Change)

The workspace already has a well-chosen dependency set. All crates below are
inherited; this document only discusses additions and patterns on top of them.

| Category | Crate | Version |
|----------|-------|---------|
| Async runtime | tokio (full) | 1.44 |
| HTTP client | reqwest (stream, native-tls) | 0.13 |
| SSE raw streaming | hand-rolled `sse_parser` in `crates/api/src/lib.rs` | — |
| TUI | ratatui | 0.29 |
| Terminal backend | crossterm (event-stream) | 0.28 |
| MCP SDK | rmcp | 1.4.0 |
| Concurrency | parking_lot, dashmap | 0.12 / 6 |
| Storage | rusqlite (bundled) | 0.31 |

---

## Domain 1: Async SSE / Streaming LLM Responses

### Current state

`crates/api/src/lib.rs` contains a hand-rolled `sse_parser::SseLineParser` that
operates by calling `resp.bytes_stream()` on a `reqwest::Response`, splitting
incoming `Bytes` chunks on `'\n'`, and accumulating event/data fields. This
works for Anthropic. The `stream_parser.rs` file declares `SseStreamParser` and
`JsonLinesStreamParser` trait objects with stub bodies (deferred to Phase 2A).
Google and OpenAI providers implement their own inline SSE parsing using the same
`bytes_stream()` + manual split pattern.

### Recommendation: Do NOT add eventsource-stream or reqwest-eventsource

**Rationale:**

The hand-rolled parser is sufficient and already used in production for the
Anthropic provider. Adding `eventsource-stream` (v0.2.3) or `reqwest-eventsource`
(v0.6.0) would introduce a new abstraction on top of `reqwest::Response` that
conflicts with the existing `StreamParser` trait. The `SseStreamParser` stub in
`stream_parser.rs` is the right seam — implement the `StreamParser::parse()`
method for each provider using the same pattern already used in `lib.rs`.

**Pattern to follow (HIGH confidence — directly observed in codebase):**

```rust
// Inside StreamParser::parse() for a new provider:
let mut byte_stream = response.bytes_stream();
let mut parser = SseLineParser::new();   // already exists in lib.rs mod
let mut leftover = String::new();

while let Some(chunk) = byte_stream.next().await {
    // split on '\n', feed lines to parser, emit StreamEvents
}
```

The `tokio-util` codec `LinesCodec` could replace the manual split but adds no
correctness benefit for SSE (SSE frames are newline-delimited, not
length-prefixed) and the current approach already handles split-chunk boundaries
via the `leftover` string. Do not change it.

**For non-SSE providers (JSON lines, chunked JSON):** use the `JsonLinesStreamParser`
stub with the same `bytes_stream()` pattern but accumulate until valid JSON rather
than until blank line.

### What actually needs doing

- Implement `SseStreamParser::parse()` for Anthropic/Google (Phase 2A marker).
- The Minimax provider uses `AnthropicClient` with `use_bearer_auth: true` which
  already sends `Authorization: Bearer <key>`. Issue #117 (minimax Authorization
  header) appears already implemented; validate against the live API.
- Ollama provider already reads `OLLAMA_HOST` env var from
  `crates/api/src/providers/openai_compat_providers.rs`. Issue #86 (remote Ollama)
  is a bug in the Settings UI not forwarding `OLLAMA_HOST` to the provider
  factory, not a missing crate dependency.
- Custom OpenAI base URL (#106): `OPENAI_BASE_URL` env var is already wired in
  `api_base_env_var_for_provider()`. The bug is in the TUI settings screen
  (`settings_screen.rs`) not exposing an input field for it.

---

## Domain 2: Terminal UI Patterns

### Current state

`crates/tui/` already has:
- `virtual_list.rs` — a `VirtualList<T: VirtualItem>` with `scroll_offset` and
  `viewport_height`, rendering only items whose row ranges intersect the viewport.
  This is the correct pattern for the message pane.
- `app.rs` — full `handle_mouse_event()` implementing scroll, text selection,
  double/triple-click detection.
- All major dialogs exist as separate files.

ratatui 0.30.0 is available (current is 0.29). The 0.30 release added
`scrolling-regions` for flicker-free `insert_before`, better `Table` scrolling
APIs. **Do not upgrade yet** — ratatui is pinned via workspace dep and a minor
upgrade would need validation across all 12 crates.

### Mouse capture issue (#104)

`crates/tui/src/lib.rs` unconditionally calls `EnableMouseCapture` on startup.
This intercepts all mouse events at the terminal level, preventing the native OS
text selection from working. No new crate is needed — the fix is conditional
mouse capture: only enable `EnableMouseCapture` when the user has mouse mode
turned on (via a settings flag), and disable it when they turn it off. This is a
logic change in `lib.rs`, not a dependency change.

### Recommendation: Do NOT add tui-input or tui-textarea

`crates/tui/src/prompt_input.rs` and `crates/tui/src/input.rs` are hand-rolled.
`tui-textarea` (a popular third-party crate) would conflict with the existing
multi-line prompt input that already handles Vim-mode keybindings and history.
Adding it is more work than fixing the existing code.

### For missing UI components (agents view, settings parity)

The `agents_view.rs` file exists in `crates/tui/src/`. The pattern for new views
is established:
- State machine with a `Mode` enum (list / detail / edit / confirm-delete)
- `StatefulWidget` with `ListState` for navigable lists
- Wrap in an overlay using the existing `overlays.rs` machinery

No new TUI crates are needed. The `VirtualList<T>` type in `virtual_list.rs` is
the correct primitive for any view with unbounded items.

---

## Domain 3: MCP Sandbox / Permission Enforcement (Security Issue #123)

### Current state

`claurst-core` has a complete `permissions` module (inline in `lib.rs`) with:
- `PermissionLevel` (Read / Write / Execute / Network)
- `PermissionRule` with tool name + glob path pattern matching
- `PermissionManager` with session and persistent scopes
- `InteractivePermissionHandler` that sends a `PermissionRequest` over a
  `tokio::sync::oneshot` channel and awaits TUI approval

The security vulnerability (#123) is **not** a missing crate issue. The Claude
Code TypeScript spec allows MCP server configurations to be placed in
`.claude/mcp.json` inside any project directory. If claurst loads and executes
these without validating that the MCP server binary path is on an allowlist,
arbitrary code runs. The fix is policy enforcement in `crates/mcp/src/backend.rs`
/ `connection_manager.rs` before spawning child processes.

### What to add for OS-level sandboxing (optional, Linux-only)

If deeper sandboxing of MCP child processes is desired beyond the existing
permission rules, use:

| Crate | Version | What it does |
|-------|---------|--------------|
| `landlock` | 0.4.4 | Linux Landlock LSM — restricts filesystem access for a process without root. Requires Linux 5.13+. |

**Confidence: MEDIUM** — landlock is well maintained (landlock-lsm/rust-landlock,
the official Rust bindings). The API is stable. Applying Landlock to a spawned
MCP child before `exec` requires `nix::unistd::fork` + `exec` which the codebase
already uses via `portable-pty` patterns. On macOS this path does not exist
(Sandbox.kext / `sandbox_init()` requires entitlements). Gate behind
`#[cfg(target_os = "linux")]`.

**Do NOT add `seccompiler` (v0.5.0)** for the MCP security fix. Seccomp BPF
policy writing is complex, brittle across kernel versions, and the permission
issue is an allowlist problem (which binary paths can MCP configs reference), not
a syscall filtering problem. The existing `PermissionManager` + a path allowlist
in `McpServerConfig` is the right fix and requires zero new crates.

**The minimal correct fix for #123 (no new crates):**
1. Add an `allowed_mcp_server_paths: Vec<String>` field (glob patterns) to
   `Settings` in `claurst-core`.
2. In `crates/mcp/src/connection_manager.rs`, before spawning a child process,
   check the binary path against the allowlist.
3. Require explicit user confirmation (via the existing `PermissionManager`) when
   a project-level MCP config introduces a new server not in the global allowlist.

---

## Domain 4: Cross-Provider Model Routing

### Current state

The `ProviderRegistry` in `crates/api/src/registry.rs` holds named
`Arc<dyn LlmProvider>` instances. The routing logic in
`crates/query/` selects a provider by name from the registry. All providers
listed in `openai_compat_providers.rs` (20+ entries) follow the
`OpenAiCompatProvider` pattern with `ProviderQuirks` for per-provider
idiosyncrasies.

### Recommendation: No new crates needed for routing

The existing `LlmProvider` trait + `ProviderRegistry` pattern handles routing
correctly. The pattern for adding a new provider is established and documented:

1. Create `providers/myprovider.rs` implementing `LlmProvider`.
2. Register in `crates/api/src/providers/mod.rs` and `registry.rs`.
3. Expose env-var configuration in `api_base_env_var_for_provider()` and
   `api_key_env_vars_for_provider()` in `claurst-core/src/lib.rs`.

The `model_registry.rs` in `crates/api/` handles model aliases. No new crate
is needed for routing.

### For Managed Agents (plan.md)

The `AgentTool` stub in `crates/tools/src/agent_tool.rs` is the correct
insertion point. The existing `run_query_loop` in `crates/query/` is the
primitive. The plan.md approach (manager is a query loop with a delegation system
prompt) is correct and needs no new crates.

---

## Domain 5: Voice / ALSA (#88)

### Current state

`crates/tui/src/voice_capture.rs` and `crates/core/src/voice.rs` implement PTT
audio capture via `cpal` (feature-gated). The bug is that `cpal`'s default ALSA
host on Linux requires `libasound2-dev` at compile time and `libasound2` at
runtime. When the user's system lacks ALSA configured correctly, `cpal` returns
no input devices. The toggle "off" bug is a state management issue in `app.rs`.

**No new audio crate is needed.** `cpal` 0.15 is the correct choice — it
abstracts over ALSA (Linux), CoreAudio (macOS), WASAPI (Windows). The fix is:
1. Better error messages when `default_input_device()` returns `None` (partially
   done — error message mentions ALSA/PulseAudio).
2. Graceful voice toggle that checks `check_voice_availability()` before enabling
   the UI state.

**Transcription:** The codebase calls the OpenAI Whisper API (POST
`/v1/audio/transcriptions`) via reqwest. No Whisper crate needed; the API is
plain HTTP multipart. This is the correct approach.

**Do NOT add `whisper-rs` or Sherpa-ONNX.** Local ASR is explicitly out of scope
per PROJECT.md.

---

## Domain 6: YAML Frontmatter for Agent Files

### Current state

`crates/core/src/skill_discovery.rs` hand-parses YAML frontmatter by splitting
on `---` and parsing `key: value` lines manually. This works for the simple case
(name, description). The agent system (`.claude/agents/*.md` per spec) uses
richer YAML front matter including `tools`, `model`, `memory`, `effort` fields.

### Recommendation: Add `serde_yml` for agent file parsing

`serde_yaml` (v0.9.34) is deprecated. Its maintained successor is `serde_yml`
(v0.0.12, maintained by the dtolnay ecosystem, same API surface).

**Confidence: MEDIUM** — serde_yml is young (0.0.x) but tracks serde_yaml API
closely. The alternative is keeping hand-rolled front-matter parsing, which is
correct for simple key-value pairs but will become fragile as agent YAML includes
nested structures (tool lists, permission modes).

**Only add this if implementing the full Agents subsystem from spec/05.** For
simple `name:` / `description:` parsing, the existing hand-roller in
`skill_discovery.rs` is sufficient.

---

## Gaps Summary: What to Add vs What to Leave

### Add (with justification)

| Crate | Version | Crate for | Confidence | Condition |
|-------|---------|-----------|------------|-----------|
| `insta` | 1.47 | Characterization/snapshot tests | HIGH | v1.1 refactoring milestone — add now |
| `proptest` | 1.9.0 | Property-based tests for pure function extractions | HIGH | v1.1 refactoring milestone — add now |
| `proptest-derive` | 0.8.0 | Derive Arbitrary for proptest inputs | MEDIUM | With proptest |
| `landlock` | 0.4.4 | MCP child process filesystem restriction | MEDIUM | Linux only, only if deeper MCP sandbox desired beyond allowlist fix |
| `serde_yml` | 0.0.12 | Agent YAML frontmatter (spec/05 agents subsystem) | MEDIUM | Only when implementing full agents CRUD from spec |

### Do NOT Add

| Crate | Reason |
|-------|--------|
| `cargo-tarpaulin` | Linux-only; development env is macOS; use cargo-llvm-cov instead |
| `cargo-udeps` | Requires nightly; cargo-machete is sufficient for dispensable smell sweep |
| `quickcheck` | Proptest supersedes it with better shrinking and derive support |
| `arbitrary` (standalone) | Fuzzer-integration trait; proptest-derive covers the same use case |
| `criterion` | Benchmarking; out of scope for refactoring milestone |
| `eventsource-stream` / `reqwest-eventsource` | Hand-rolled SSE parser in lib.rs is sufficient; adding this creates conflicting abstraction layers |
| `tui-textarea` | Conflicts with existing hand-rolled prompt_input.rs; more work to integrate than to fix |
| `tui-input` | Same reason as tui-textarea |
| `seccompiler` | Wrong tool for the MCP security problem; the fix is a path allowlist, not syscall filtering |
| `async-openai` | Would duplicate all of crates/api; the LlmProvider trait is the abstraction layer |
| `whisper-rs` / `sherpa-onnx` | Explicitly out of scope per PROJECT.md |
| `serde_yaml` | Deprecated as of v0.9.34+; use serde_yml if YAML is needed |
| `tower-lsp` / `lsp-types` | claurst-core already has a hand-rolled LSP client over JSON-RPC; adding tower-lsp would require a complete rewrite of lsp.rs for no behavioral gain |
| `keyring` | Auth is stored in `~/.claurst/auth.json` (plaintext JSON). A keyring would improve security but requires platform-specific library linking (libsecret on Linux) and is not required for parity |

---

## Pattern Recommendations

### Pattern 1: Per-Provider SSE Implementation (HIGH confidence)

Each provider's `create_message_stream()` should follow the existing `AnthropicClient::process_sse_stream()` pattern:

```rust
let (tx, rx) = tokio::sync::mpsc::channel(256);
tokio::spawn(async move {
    let mut byte_stream = response.bytes_stream();
    let mut leftover = String::new();
    while let Some(chunk) = byte_stream.next().await {
        // parse SSE frames, send StreamEvent via tx
    }
});
Ok(rx)
```

The `SseStreamParser` struct in `stream_parser.rs` should be filled in using this
pattern — it already has the right trait signature.

### Pattern 2: MCP Security Fix Without New Crates (HIGH confidence)

The correct fix for #123 is enforcement in the existing permission stack, not OS
sandboxing:

```
McpServerConfig loaded from project .claude/mcp.json
  → check server.command against allowlist in Settings.allowed_mcp_commands
  → if not in allowlist: send PermissionRequest to TUI
  → TUI shows bypass_permissions_dialog.rs (already exists)
  → User approves → add to session allowlist (PermissionScope::Session)
```

No new crates. The entire permission infrastructure already exists.

### Pattern 3: Provider Registry Extension for Custom Base URL (HIGH confidence)

Issue #106 (custom OpenAI API URL) is a TUI settings gap, not a missing crate.
The env var `OPENAI_BASE_URL` is already read by `OpenAiCompatProvider`. The fix
is adding an input field in `settings_screen.rs` that writes to a
`provider_base_urls: HashMap<String, String>` field in `Settings`, which then
gets forwarded when constructing providers in the registry.

### Pattern 4: Ratatui Upgrade Path (MEDIUM confidence)

ratatui 0.30.0 is available. The 0.29→0.30 changelog shows mostly additive
changes (new widget methods, scrolling-regions stabilization). **Defer the
upgrade** until a phase that touches the TUI heavily — then upgrade in one step
and run the existing TUI test suite. The only behavioral change that matters is
`scrolling-regions` for `insert_before`, which would fix potential flicker in the
message pane, but it requires opt-in via a crate feature flag.

---

## Confidence Assessment

| Area | Confidence | Reasoning |
|------|------------|-----------|
| Refactoring toolchain (new) | HIGH | All tools verified via Context7, official docs, or GitHub releases |
| SSE streaming patterns | HIGH | Directly read the implementation; hand-rolled parser is proven |
| MCP permission fix approach | HIGH | Permission infrastructure fully exists; issue is application logic |
| Ollama/minimax/custom-URL bugs | HIGH | Root causes confirmed in source; no crate changes needed |
| landlock for MCP sandbox | MEDIUM | Crate is well-maintained official bindings; Linux-only constraint limits scope |
| serde_yml for agent YAML | MEDIUM | serde_yml is young (0.0.x); API stable but not battle-tested at scale |
| ratatui 0.30 upgrade | MEDIUM | API is mostly additive; risk is in the 12 crates that use ratatui |
| Voice/ALSA fix | HIGH | Bug is in configuration/state management, not audio stack |

---

## Sources

**Refactoring toolchain (new):**
- [Clippy Lint Configuration](https://doc.rust-lang.org/clippy/lint_configuration.html)
- [Clippy Lints Index](https://rust-lang.github.io/rust-clippy/master/index.html)
- Context7 `/mitsuhiko/insta` — insta docs and version
- Context7 `/proptest-rs/proptest` — proptest 1.9.0 docs
- Context7 `/taiki-e/cargo-llvm-cov` — coverage tool docs
- [cargo-nextest home](https://nexte.st/) — 0.9.116
- [cargo-mutants GitHub releases](https://github.com/sourcefrog/cargo-mutants/releases) — v27.0.0
- [cargo-mutants Thoughtworks Radar](https://www.thoughtworks.com/radar/tools/cargo-mutants)
- [cargo-machete GitHub](https://github.com/bnjbvr/cargo-machete) — 0.9.2
- [Coverage comparison: tarpaulin vs llvm-cov](https://rustprojectprimer.com/measure/coverage.html)

**Original (feature work):**
- Codebase: `/Users/thamw/development/local/clearest-rust/crates/` (directly read)
- [eventsource-stream on crates.io](https://crates.io/crates/eventsource-stream) — v0.2.3
- [reqwest-eventsource on crates.io](https://crates.io/crates/reqwest-eventsource) — v0.6.0
- [landlock on crates.io](https://crates.io/crates/landlock/0.4.1) — v0.4.4
- [rust-landlock GitHub](https://github.com/landlock-lsm/rust-landlock) — official bindings
- [serde_yml on crates.io](https://crates.io/crates/serde_yml) — v0.0.12
- [ratatui v0.29 highlights](https://ratatui.rs/highlights/v029/)
- [ratatui on crates.io](https://crates.io/crates/ratatui) — v0.30.0 available
- [seccompiler on crates.io](https://crates.io/crates/seccompiler) — v0.5.0

---

*Research date: 2026-05-04 (original); 2026-05-13 (refactoring toolchain addendum)*
