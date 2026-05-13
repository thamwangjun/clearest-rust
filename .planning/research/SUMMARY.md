# Project Research Summary

**Project:** claurst — v1.1 Codebase Refactor
**Domain:** Behavior-preserving refactoring of an AI-generated 12-crate Rust workspace
**Researched:** 2026-05-13
**Confidence:** HIGH

## Executive Summary

This milestone is a structured, behavior-preserving refactoring of the claurst workspace — an AI-generated Rust CLI totaling ~355K lines across 12 crates. The codebase has significant but well-catalogued code smells typical of AI-generated Rust: three god files exceeding 4,000–8,000 lines individually (`commands/lib.rs`, `tui/app.rs`, `core/lib.rs`), ~410 `.unwrap()` calls in production paths, 32 `#[allow(dead_code)]` suppressions, and widespread primitive obsession (domain concepts stored as raw `String`). The refactoring approach is characterization-first, bottom-up by dependency layer: establish a safety net of snapshot and behavioral tests before touching production code, then work leaf crates upward.

The recommended approach adds four new dev tools (`insta` for snapshot tests, `cargo-nextest` for fast parallel test runs, `cargo-llvm-cov` for coverage baselines, `cargo-machete` for dead dependency sweeps), tightens `clippy.toml` thresholds, and works through the crates in topological order: `claurst-core` first (both the foundation layer and the worst grab-bag), then Layer 1 protocol crates, then tools, then query/bridge orchestration, and finally the three highest-risk files (`tui/app.rs`, `commands/lib.rs`, `cli/main.rs`). Each phase follows the same four-step loop: clippy sweep to identify smells, coverage baseline, write characterization tests, extract and refactor, verify snapshots unchanged.

The central risk is the interplay between Rust's borrow checker and method extraction on ownership-tangled structs (particularly `App` and `run_query_loop`). Extractions that worked inline because the compiler could see field-level split borrows will break at function-call boundaries. The mitigation is to map all field accesses before extracting, prefer cloning at boundaries during refactoring, and optimize later. The characterization test suite is non-negotiable as the first deliverable — no code moves without it.

---

## Key Findings

### Recommended Stack — New Dev Tooling

The workspace already has a sound production dependency set (tokio, ratatui, reqwest, rmcp, rusqlite). The v1.1 refactoring milestone adds exclusively dev tooling; no production dependencies change.

**Essential additions (Cargo.toml `[dev-dependencies]` + CLI installs):**
- `insta` 1.47 — snapshot/characterization tests; `.snap` files committed to git; CI uses `cargo insta test --check` to fail on behavioral drift
- `cargo-nextest` 0.9.116 — 3x faster than `cargo test`; process-per-test isolation catches global-state leaks that `cargo test`'s thread model masks
- `cargo-llvm-cov` (latest via brew tap) — macOS-compatible LLVM coverage; per-crate baselines before refactoring; CI gate via `--fail-under-lines`
- `proptest` 1.9.0 + `proptest-derive` 0.8.0 — property tests for pure function extractions where invariant-based assertions survive implementation changes

**Nice-to-have:**
- `cargo-machete` 0.9.2 — dead dependency scan (text-search, runs in ~1s)
- `cargo-mutants` 27.0.0 — mutation testing to validate test quality; defer until test suite stabilizes

**Clippy configuration changes:**

New `clippy.toml` at workspace root:
```toml
cognitive-complexity-threshold = 15   # default 25
too-many-arguments-threshold = 6      # default 7
too-many-lines-threshold = 80         # default 100
type-complexity-threshold = 200       # default 250
```

Add to `[workspace.lints.clippy]`:
```toml
pedantic = "warn"
unwrap_used = "warn"
expect_used = "warn"
wildcard_imports = "warn"
```

Expect 50–200 new warnings on first run. Triage per-crate with targeted `#[allow]` plus justification comments; never suppress wholesale at crate root.

**Skip:** `cargo-tarpaulin` (Linux-only), `cargo-udeps` (requires nightly), `quickcheck` (proptest supersedes it), `criterion` (benchmarking, out of scope).

Full rationale and install commands: `.planning/research/STACK.md`

---

### Expected Features — Code Smells to Fix

Research catalogued smells across all 12 crates from live `cargo clippy` output and direct code inspection.

**Table stakes (codebase is unmaintainable without these):**

| Smell | Worst Instance | Why Blocking |
|---|---|---|
| Large Class / Divergent Change | `commands/lib.rs` 8,657 lines; `tui/app.rs` 5,990 lines; `core/lib.rs` 4,291 lines | Adding any command, UI state, or type requires touching the same file as unrelated changes |
| Long Method | `run_query_loop` cognitive complexity 156/25; multiple 300-line `execute` methods in commands | Impossible to reason about control flow; multi-concern functions resist safe extraction |
| Long Parameter List | `run_interactive` takes 11 params; 4 functions in `core/import_config.rs` take 8-9 each | Wrong-order argument bugs compile silently |
| Dead Code suppression | 32 `#[allow(dead_code)]` across workspace | Hides whether code is intentional scaffolding or garbage |
| Duplicate Code | `error_marker` triplicated in `tui/message_copy.rs`; `if args.is_empty()` guard in all 30 command `execute` fns | Bug fixes must be applied in N places |
| Primitive Obsession | `session_id: String`, `model: String`, provider names dispatched via string match | Argument-swap bugs are invisible to the compiler |

**Differentiators (do after tests are solid):**

| Smell | Fix | Value |
|---|---|---|
| Primitive Obsession (type codes) | `ToolStatus` enum; `ModelId`/`SessionId` newtypes | Compiler-enforced transitions; compile-time typo detection |
| Data Class | `ToolUseBlock::mark_complete()`, `Message::has_tool_use()` | Behavior lives where the data lives |
| Feature Envy | `EffortLevel::to_api_str()` in core; command logic to `CommandContext` methods | Eliminates 6 copies of the same match in `query` |
| Shotgun Surgery | Single `Provider` registration point; eliminate provider string literals from 6 crates | New providers add one `impl`, not N match arms |
| Message Chains | `config.api_key_for("anthropic")` vs 4-step chain | Decouples `App` from internal `Config`/`Settings` layout |
| Speculative Generality | `Arc<dyn Fn>` callbacks replaced with concrete handler structs where one call site exists | Removes indirection; easier to trace |

**Anti-features (do not build):**
- God `Utils` module — creates a new Divergent Change target
- Blanket `#[allow(clippy::too_many_arguments)]` — hides the symptom
- Breaking up small crates (`acp`, `buddy`) into smaller units — adds build overhead without benefit
- Replacing `Box<dyn SlashCommand>` with enum — legitimate polymorphism, leave it

Full catalog with detection commands: `.planning/research/FEATURES.md`

---

### Architecture Approach

The codebase has a clear six-layer dependency graph. Safe refactoring order follows topological sort from leaf to root. Every extraction follows: `cargo clippy` to find smells → `cargo llvm-cov` for baseline → write characterization tests → extract → run `cargo insta test --check` → verify coverage unchanged.

**Dependency layers (bottom to top):**

```
Layer 0: claurst-core, claurst-buddy (no workspace deps — foundation)
Layer 1: claurst-api, claurst-mcp, claurst-plugins, claurst-acp (depend on core only)
Layer 2: claurst-tools (core + api + mcp)
Layer 3: claurst-query (core + api + tools + plugins)
Layer 4: claurst-bridge (core + api + query)
Layer 5: claurst-tui, claurst-commands (commands depends on tui — tui must be stable first)
Layer 6: claurst-cli (binary; depends on all)
```

**Key extraction targets:**
- `core/lib.rs` (4,291 lines): `error.rs` → `message_types.rs` → `config.rs` → `auth.rs` in sequence; `lib.rs` becomes pure `pub use`
- `tui/app.rs` (5,990 lines): decompose supporting modules first, then extract `event_handler.rs`, `state.rs`, `render_dispatch.rs`; decompose App's 150 fields into `SessionState`, `UiState`, `RuntimeState`, `InfraHandles`
- `commands/lib.rs` (8,657 lines): one file per slash command under `commands/src/slash/`; wait until tui is stable
- `cli/main.rs` (3,732 lines): extract `McpToolWrapper`, OAuth flow, MCP init to library crates

**Borrow checker extraction rules (critical for `app.rs` and `run_query_loop`):**
1. Extract pure functions first
2. Clone at boundaries during refactoring, optimize later
3. Decompose structs into nested sub-structs to unlock split borrows
4. Never hold `std::sync::Mutex` or `parking_lot::Mutex` guards across `.await` points

Full dependency graph, extraction sequences, test patterns: `.planning/research/ARCHITECTURE.md`

---

### Critical Pitfalls

1. **Method extraction breaks split borrows** — Extracting a block into a new method loses the compiler's field-level borrow analysis. Prevention: map all field accesses before extracting; prefer direct `pub(crate)` field access over getter methods during extraction; extract pure functions first.

2. **Removing `.clone()` cascades lifetimes across multiple crates** — `.clone()` is a lifetime eraser. Removing one in `core` can force lifetime annotations through `query`, `tui`, and `commands`. Prevention: remove one clone at a time; if removal requires adding lifetime params to more than 2 function signatures in different modules, restructure ownership instead.

3. **`std::sync::Mutex` guard held across `.await` deadlocks Tokio** — The workspace uses `parking_lot::Mutex` in several paths; same rule applies. Prevention: audit every mutex for `.await` in the same scope; never expose a guard outside a synchronous newtype wrapper method.

4. **Characterization tests at the wrong granularity** — Too coarse misses internal regressions; too fine (private functions) breaks on every rename. Prevention: test at public crate interfaces; use `insta` snapshots for CLI output and `ratatui::backend::TestBackend` for TUI state.

5. **Big-bang multi-crate PR stalls CI** — Type renames ripple through 11 consumers; 2,000-line diffs are unreviable and unrollbackable. Prevention: `pub use` re-exports + `#[deprecated]` for incremental renames; hard limit of 3 crates and 500 lines diff per structural PR.

Additional: orphan rule prevents cross-crate Feature Envy fixes (use extension traits); unwrap replacement can paper over `Option<T>` type design issues (categorize each before replacing, never batch); crate API breakage invisible within workspace (run `cargo-semver-checks` as phase exit gate).

Full 15-pitfall catalog with phase mappings: `.planning/research/PITFALLS.md`

---

## Implications for Roadmap

### Phase 1: Characterization Test Infrastructure
**Rationale:** No code moves before behavior is anchored. This is the gating constraint across all four research files.
**Delivers:** `insta` + `cargo-nextest` + `cargo-llvm-cov` wired in workspace; `clippy.toml` tightened; per-crate coverage baselines; `App::new_for_test()` constructor; `crates/test-utils` with shared fakes (`FakeLlmProvider`, `FakeSessionStore`); CLI smoke tests via `assert_cmd`; TUI render snapshots via `TestBackend` + insta for each `AppState` variant
**Avoids:** Pitfall 6 (wrong-granularity tests), Pitfall 10 (deleting abstractions before tests reveal they are load-bearing for test doubles)
**Research flag:** Standard patterns — no deeper research needed

### Phase 2: claurst-core Decomposition (Layer 0)
**Rationale:** Most depended-on crate AND a leaf. Fixing it propagates benefits to all 11 dependents. Worst Divergent Change target.
**Delivers:** `core/lib.rs` split into `error.rs` → `message_types.rs` → `config.rs` → `auth.rs`; `lib.rs` becomes pure re-exports; `session_id`/`model` primitive obsession replaced with newtypes; parameter objects for 8+ param functions; 32 dead-code suppressions audited
**Avoids:** Pitfall 8 (use `pub use` re-exports during split), Pitfall 12 (one sub-module per PR), Pitfall 7 (`cargo-semver-checks` before any `pub` symbol removal)
**Research flag:** Standard patterns

### Phase 3: Layer 1 Protocol Crates (claurst-api, claurst-mcp, claurst-plugins, claurst-acp)
**Rationale:** Depend only on core (now stable). `claurst-api` is already well-structured. `claurst-mcp` has Mutex fragility. `claurst-acp` (285 lines, no tests) is a Lazy Class candidate for inlining into `cli`/`query`.
**Delivers:** `claurst-mcp` Mutex poison chains hardened; `claurst-acp` evaluated for inlining or given test coverage; `cargo-machete` dead dependency sweep
**Avoids:** Pitfall 3 (Mutex-across-await audit), Pitfall 4 (keep `dyn LlmProvider` — runtime-selected trait)
**Research flag:** Standard patterns

### Phase 4: claurst-tools Cleanup (Layer 2)
**Rationale:** ~370 `.unwrap()` in production paths — highest concentration in workspace.
**Delivers:** `.unwrap()` calls categorized and addressed per Pitfall 5 rules; Feature Envy fixes via extension traits (not method moves, due to orphan rule); tiny formatter/synthetic_output files evaluated for folding
**Avoids:** Pitfall 5 (unwrap categorization before replacing), Pitfall 9 (orphan rule)
**Research flag:** Standard patterns

### Phase 5: claurst-query and claurst-bridge (Layers 3-4)
**Rationale:** `run_query_loop` is 2,400 lines at cognitive complexity 156/25 — most complex function in workspace. `claurst-bridge` has zero tests and is a single file.
**Delivers:** `run_query_loop` decomposed into named step functions; duplicate tool-dispatch patterns extracted; bridge test coverage established; `AGENT_RUNNER OnceCell` panic characterized with `#[should_panic]` (do NOT fix the global pattern here — separate milestone)
**Avoids:** Pitfall 1 (split-borrow analysis before extraction), Pitfall 3 (async mutex audit for bridge shared state), Architecture Anti-Pattern 1 (do not fix `AGENT_RUNNER` global)
**Research flag:** NEEDS RESEARCH — the async ownership patterns in `run_query_loop` are complex; recommend `/gsd-research-phase` before execution planning for this phase

### Phase 6: claurst-tui Decomposition (Layer 5a)
**Rationale:** Must be stable before claurst-commands (which depends on tui). Decompose supporting modules before tackling `app.rs`.
**Delivers:** `prompt_input.rs` (3,719 lines) split into `input_history.rs`, `completion.rs`, `input_render.rs`; `overlays.rs` (2,103 lines) split into `overlays/` tree; `dialogs.rs` (1,621 lines) split; `app.rs` split into `event_handler.rs`, `state.rs`, `render_dispatch.rs`; `App` 150-field struct decomposed into `SessionState`/`UiState`/`RuntimeState`/`InfraHandles`
**Avoids:** Pitfall 1 (struct decomposition is prerequisite to method extraction — split borrows only work across distinct struct fields)
**Research flag:** Standard patterns — ratatui TestBackend + insta documented

### Phase 7: claurst-commands Decomposition (Layer 5b)
**Rationale:** Worst god file saved last because it depends on tui (stable after Phase 6).
**Delivers:** Each slash command to `commands/src/slash/<name>.rs`; `framework/` module for `CommandContext`/`CommandResult`; `lib.rs` reduced to dispatch registry (~200 lines); `require_args` helper eliminating 30 duplicate guards; `text_from_content_blocks` moved to `core::types`
**Avoids:** Pitfall 8 (one command file per PR using `pub use` bridges)
**Research flag:** Standard patterns

### Phase 8: claurst-cli Extraction (Layer 6)
**Rationale:** Binary crate last. Extract logic belonging in library crates; reduce `main.rs` to thin wiring.
**Delivers:** `McpToolWrapper` to `crates/tools`; OAuth dispatch to `cli/src/startup/auth.rs`; MCP init to `startup/mcp_init.rs`; CLI integration tests via `assert_cmd` + `insta_cmd`
**Research flag:** Standard patterns

### Phase Ordering Rationale

- Phases 2-8 follow strict topological order — refactoring a dependent before its dependencies are stable causes rework cascades.
- `claurst-commands` (worst god file) is Phase 7, not Phase 1, because it depends on `claurst-tui`. This is the critical ordering constraint confirmed in Cargo.toml.
- The characterization test phase is load-bearing — Pitfalls 6 and 10 both describe how skipping it causes silent regressions.
- Phase 5 (`run_query_loop`) is the highest semantic risk and the most likely to benefit from a dedicated research pass.

### Research Flags

Needs research before execution planning:
- **Phase 5 (claurst-query):** `run_query_loop` at 2,400 lines / complexity 156 with complex async ownership — recommend `/gsd-research-phase` on "extracting async methods from complex Tokio state machine"

Standard patterns (skip research-phase):
- **Phases 1, 2, 3, 4, 6, 7, 8:** All use well-documented Rust module decomposition, insta snapshot, and ratatui TestBackend patterns confirmed in official docs

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack (dev tooling) | HIGH | All versions verified via Context7, official docs, GitHub releases; macOS compatibility confirmed for llvm-cov |
| Features (smell catalog) | HIGH | All confirmed instances from live `cargo clippy` output and direct code inspection — not inferred |
| Architecture (phase order) | HIGH | Dependency graph from actual Cargo.toml files; ordering constraints factual, not estimated |
| Pitfalls | HIGH (Rust-specific) / MEDIUM (AI-code patterns) | Borrow checker, async mutex, monomorphization pitfalls have authoritative sources; AI-code-specific patterns from community sources |

**Overall confidence:** HIGH

### Gaps to Address

- **`CLAURST_MOCK_PROVIDER` mode does not exist:** The highest-ROI integration test class (headless `-p` mode with a mock provider) requires adding a mock provider feature flag. Plan in Phase 1, not discovered in Phase 8.
- **36 Cargo feature flags not fully audited:** A full audit of which flags are ever toggled off in CI or production is a prerequisite for per-feature `cargo check` gates but was not completed in this research pass.
- **`run_query_loop` extraction plan missing:** Research confirmed the complexity (156/25, 2,400 lines) but did not produce a field-access map or extraction sequence. Phase 5 needs its own research pass before execution planning.

---

## Sources

### Primary (HIGH confidence)
- Codebase direct inspection (`crates/*/src/*.rs`, `Cargo.toml` files) — live analysis 2026-05-13
- Context7 `/mitsuhiko/insta` — insta 1.47 docs
- Context7 `/proptest-rs/proptest` — proptest 1.9.0 / proptest-derive 0.8.0
- Context7 `/taiki-e/cargo-llvm-cov` — coverage, macOS install
- [Clippy Lint Configuration](https://doc.rust-lang.org/clippy/lint_configuration.html)
- [cargo-nextest](https://nexte.st/) — 0.9.116
- [cargo-mutants GitHub](https://github.com/sourcefrog/cargo-mutants/releases) — v27.0.0
- [cargo-machete GitHub](https://github.com/bnjbvr/cargo-machete) — 0.9.2
- [ratatui TestBackend + insta](https://ratatui.rs/recipes/testing/snapshots/)
- [How to Deadlock a Tokio Application — Turso](https://turso.tech/blog/how-to-deadlock-tokio-application-in-rust-with-just-a-single-mutex)
- [cargo-semver-checks](https://crates.io/crates/cargo-semver-checks)

### Secondary (MEDIUM confidence)
- [How to Avoid Fighting the Rust Borrow Checker — qouteall](https://qouteall.fun/qouteall-blog/2025/How%20to%20Avoid%20Fighting%20Rust%20Borrow%20Checker)
- [Clone to Satisfy the Borrow Checker — Rust Design Patterns](https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html)
- [Item 12: Generics vs Trait Objects — Effective Rust](https://www.lurklurk.org/effective-rust/generics.html)
- [Long-term Rust Project Maintenance — corrode.dev](https://corrode.dev/blog/long-term-rust-maintenance/)
- [idiomatic-rust](https://github.com/mre/idiomatic-rust) — newtype pattern

---
*Research completed: 2026-05-13*
*Ready for roadmap: yes*
