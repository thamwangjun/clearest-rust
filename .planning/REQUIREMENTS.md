# Requirements: claurst v1.1 Codebase Refactor

## Overview

Systematic elimination of code smells across all 12 crates of the claurst Rust workspace, following Fowler's refactoring taxonomy translated to Rust idioms. All changes are strictly behavior-preserving: CLI flags, TUI output, JSON protocols, and file formats must be identical before and after every phase. A characterization test suite is the prerequisite gate — no code is moved until behavior is anchored.

**Constraint:** All refactoring phases must leave `cargo test` and `cargo clippy` green before merging.

---

## v1.1 Requirements

### Test Infrastructure (prerequisite gate)

- [ ] **TEST-01**: Developer can run tests against a `MockProvider` that returns canned LLM responses without network access, via a shared `crates/test-utils` crate implementing the `LlmProvider` trait
- [ ] **TEST-02**: Developer can run `cargo insta test --check` and get TUI snapshot regression failures when any `AppState` screen output changes — one snapshot per `AppState` variant using `ratatui::TestBackend`
- [ ] **TEST-03**: Developer can run CLI smoke tests (`--help`, `--version`, startup routing) via `assert_cmd` that snapshot stdout and fail on deviation
- [ ] **TEST-04**: Developer can run `cargo clippy` and get warnings on all functions exceeding cognitive complexity 15, length 80 lines, or argument count 6 — enforced via `clippy.toml` and `[workspace.lints.clippy]`

### Bloaters

- [ ] **BLOT-01**: Developer can navigate `crates/commands/src/` as one file per slash command instead of a single 8,657-line monolith (`lib.rs`); `lib.rs` becomes a pure re-export module
- [ ] **BLOT-02**: Developer can modify a single TUI state domain (input, render, session) in `tui/src/app.rs` without touching unrelated fields, enabled by the `App` struct being decomposed into nested sub-structs (`InputState`, `RenderState`, `SessionState`, etc.)
- [ ] **BLOT-03**: Developer can find any `core` type in a semantically-named submodule (`error`, `message_types`, `config`, `auth`) instead of `core/src/lib.rs`; `lib.rs` becomes a pure `pub use` re-export
- [ ] **BLOT-04**: Developer can read any single function in `render.rs`, `query/src/lib.rs`, `cli/src/main.rs`, or `commands/src/` and understand it without scrolling; no function exceeds cognitive complexity 15 after extraction passes
- [ ] **BLOT-05**: Developer can distinguish `SessionId`, `ModelId`, `ConversationId` at compile time; assigning the wrong string ID is a type error — enforced via newtypes replacing raw `String` fields
- [ ] **BLOT-06**: Developer can find sprite frame data for the `buddy` companion in a dedicated `const` data structure or `sprites.rs` module; `get_sprite_frames` is not a 426-line function body

### Dispensables

- [ ] **DISP-01**: Developer can build with `cargo check` and see zero `#[allow(dead_code)]` suppressions on genuinely unused items; all 32 suppressions are either deleted (unused code removed) or replaced with a documented rationale (`// load-bearing test double`)
- [ ] **DISP-02**: Developer can add a new slash command `execute()` body without copy-pasting the guard/format boilerplate pattern; the pattern lives in one shared helper in `commands/src/`
- [ ] **DISP-03**: Developer can read any `tools` crate function and trust that `.unwrap()` either has an `expect("reason: ...")` explanation or has been replaced with `?`; no silent panics on ~370 sites

### Couplers & Change Preventers

- [ ] **COUP-01**: Developer can change `core::TaskStatus` internals without needing to update `tools` crate call sites; and can change `api::AnthropicStreamEvent` without touching `tui/src/app.rs` — boundary types mediate both cross-crate couplings
- [ ] **COUP-02**: Developer can add a new LLM provider by editing one provider registration file instead of making changes in 5+ scattered locations across `api` and `cli`
- [ ] **COUP-03**: Developer can find methods that primarily operate on `core` or `api` types in the crate that owns those types; `render.rs` functions taking whole-`App` params are refactored to accept focused sub-state arguments; `tools/src/bash.rs` methods operating on `core` types are moved or mediated
- [ ] **COUP-04**: Developer can read any call site in `tui/src/messages/mod.rs` or `query/src/lib.rs` without following an accessor chain longer than 2 hops to understand the intent; long chains are replaced with intent-revealing methods on intermediate types

---

## Future Requirements (deferred from v1.1)

- **Mutation testing pass** (`cargo-mutants`) — validates that characterization tests actually constrain behavior; too slow during active refactoring, deferred to post-stabilization
- **`Arc<Mutex<T>>` audit** — check if any mutex guards cross `.await` boundaries (potential silent deadlocks); flagged as concern in research but not confirmed present; investigate in v1.2
- **`run_query_loop` deep refactor** — 2,400-line async function at complexity 156 needs a field-access map before extraction; scoped to BLOT-04 extract in v1.1 but full decomposition may require v1.2
- **36 Cargo feature flag audit** — determine which feature flags are ever toggled off in production; prune dead feature combinations
- **Upstream parity sync workflow** — formalize how new Claude Code features are discovered and brought in as milestones

## Out of Scope

- New user-facing features — v1.1 is behavior-preserving refactoring only
- Performance optimization — unless a refactoring incidentally improves performance; no profiling work
- Security hardening (#123, #79, #96) — separate future milestone
- New provider integrations — separate future milestone
- GUI / non-terminal interface — out of scope for this project

---

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| TEST-01 | — | Pending |
| TEST-02 | — | Pending |
| TEST-03 | — | Pending |
| TEST-04 | — | Pending |
| BLOT-01 | — | Pending |
| BLOT-02 | — | Pending |
| BLOT-03 | — | Pending |
| BLOT-04 | — | Pending |
| BLOT-05 | — | Pending |
| BLOT-06 | — | Pending |
| DISP-01 | — | Pending |
| DISP-02 | — | Pending |
| DISP-03 | — | Pending |
| COUP-01 | — | Pending |
| COUP-02 | — | Pending |
| COUP-03 | — | Pending |
| COUP-04 | — | Pending |
