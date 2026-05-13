# Roadmap: claurst

## Milestones

- ✅ **v1.0 Initial Bug-Fix + Bearer Auth** — Phases 1–3 (shipped 2026-05-09)
- 🚧 **v1.1 Codebase Refactor** — Phases 4–13 (in progress)

## Phases

<details>
<summary>✅ v1.0 Initial Bug-Fix + Bearer Auth (Phases 1–3) — SHIPPED 2026-05-09</summary>

- [x] Phase 1: Welcome Screen Fix (1/1 plan) — completed 2026-05-05
- [x] Phase 2: Fix UAT Gaps — thinking block collapsed leak and welcome dialog startup routing (2/2 plans) — completed 2026-05-07
- [x] Phase 3: ANTHROPIC_AUTH_TOKEN Bearer Auth Support (6/6 plans) — completed 2026-05-09

Full details: `.planning/milestones/v1.0-ROADMAP.md`

</details>

### 🚧 v1.1 Codebase Refactor (In Progress)

**Milestone Goal:** Transform the AI-written codebase into maintainable, idiomatic Rust by eliminating all major code smells across all 12 crates — anchored by a characterization test suite written before any code is moved.

- [ ] **Phase 4: Characterization Test Infrastructure** - Add insta, nextest, llvm-cov; write CLI smoke tests, TUI snapshots, MockProvider; tighten clippy.toml — zero production code changes
- [ ] **Phase 5: core Crate Decomposition** - Split core/lib.rs into error/message_types/config/auth submodules; introduce SessionId/ModelId/ConversationId newtypes
- [ ] **Phase 6: Layer 1 Protocol Crates** - Harden api/mcp/acp/plugins; introduce boundary types mediating cross-crate coupling; Mutex poison chain audit
- [ ] **Phase 7: tools Crate Cleanup** - Replace ~370 .unwrap() calls with expect/? in tools; extract buddy sprite data from 426-line function body
- [ ] **Phase 8: query and bridge Orchestration** - Decompose run_query_loop; fix long accessor chains in query/bridge message handling
- [ ] **Phase 9: tui Crate Decomposition** - Decompose app.rs/prompt_input.rs/overlays.rs/dialogs.rs; refactor render.rs Feature Envy; decompose App 150-field struct
- [ ] **Phase 10: commands Crate Decomposition** - Split 8,657-line lib.rs into per-command files; extract shared guard/format helper
- [ ] **Phase 11: cli Crate Extraction** - Extract McpToolWrapper/OAuth/MCP-init from main.rs into library crates; add provider registration point
- [ ] **Phase 12: Long Method Final Pass** - Verify no function exceeds cognitive complexity 15 across all 12 crates after all prior extractions
- [ ] **Phase 13: Cross-Cutting Cleanup** - Remove all dead code suppressions; audit and document every remaining #[allow(dead_code)]

## Phase Details

### Phase 4: Characterization Test Infrastructure
**Goal**: A complete behavioral safety net exists before any production code is moved — test failures are the only mechanism that can catch refactoring regressions
**Depends on**: Nothing (first v1.1 phase; continues from v1.0 Phase 3)
**Requirements**: TEST-01, TEST-02, TEST-03, TEST-04
**Success Criteria** (what must be TRUE):
  1. Developer can run `cargo test --workspace` and see the existing 1,221 tests pass, plus new MockProvider unit tests in `crates/test-utils`
  2. Developer can run `cargo insta test --check` and get snapshot regression failures when any `AppState` screen render changes — one `.snap` file per `AppState` variant committed in `crates/tui/tests/snapshots/`
  3. Developer can run `cargo test -p claurst-cli` and get CLI smoke tests for `--help`, `--version`, and startup routing that fail if stdout changes
  4. Developer can run `cargo clippy --workspace` and get warnings on functions exceeding cognitive complexity 15, length 80 lines, or argument count 6 — enforced by `clippy.toml` at workspace root
**Plans**: TBD

### Phase 5: core Crate Decomposition
**Goal**: The most-depended-on crate becomes navigable — types live in semantically-named modules and compile-time newtypes replace raw String domain identifiers
**Depends on**: Phase 4
**Requirements**: BLOT-03, BLOT-05
**Success Criteria** (what must be TRUE):
  1. Developer can find any `core` type (error, message, config, auth) by navigating to its named submodule without reading `lib.rs`; `lib.rs` contains only `pub use` re-exports
  2. Developer gets a compile error when assigning a `SessionId` value to a `ModelId` variable — the wrong-ID bug is a type error, not a silent runtime bug
  3. `cargo test -p claurst-core` passes and `cargo insta test --check` shows no snapshot drift
  4. `cargo-semver-checks` passes — no accidental removal of public symbols from `claurst-core`
**Plans**: TBD

### Phase 6: Layer 1 Protocol Crates
**Goal**: The api, mcp, acp, and plugins crates have hardened cross-crate boundaries — changing `core::TaskStatus` internals does not require updating `tools` call sites
**Depends on**: Phase 5
**Requirements**: COUP-01
**Success Criteria** (what must be TRUE):
  1. Developer can change `core::TaskStatus` internals without needing to update `tools` crate call sites — boundary types mediate the coupling
  2. Developer can change `api::AnthropicStreamEvent` without touching `tui/src/app.rs` — boundary types absorb the API surface change
  3. `claurst-mcp` Mutex poison chain is audited; no Mutex guard crosses an `.await` point
  4. `cargo test --workspace` passes and `cargo insta test --check` shows no snapshot drift
**Plans**: TBD

### Phase 7: tools Crate Cleanup
**Goal**: The tools crate is safe to read — every `.unwrap()` either has an explanatory `expect("reason")` or has been replaced with `?`; buddy sprite data is findable without reading a 426-line function
**Depends on**: Phase 6
**Requirements**: BLOT-06, DISP-03
**Success Criteria** (what must be TRUE):
  1. Developer can `grep -r '\.unwrap()' crates/tools/` and get zero results — all ~370 sites are replaced with `expect("reason: ...")` or `?`
  2. Developer can find `buddy` sprite frame data in a named `const` or `sprites.rs` module; `get_sprite_frames` is not a 426-line function body
  3. `cargo test -p claurst-tools` passes and `cargo insta test --check` shows no snapshot drift
**Plans**: TBD

### Phase 8: query and bridge Orchestration
**Goal**: The query loop is decomposable — `run_query_loop` is split into named step functions; call sites in query/bridge do not require following accessor chains longer than 2 hops
**Depends on**: Phase 7
**Requirements**: COUP-04
**Success Criteria** (what must be TRUE):
  1. Developer can read any call site in `tui/src/messages/mod.rs` or `query/src/lib.rs` and understand intent without following an accessor chain longer than 2 hops — intent-revealing methods replace the chains
  2. `run_query_loop` is decomposed into named step functions; no single extracted function exceeds cognitive complexity 25 in `query/src/lib.rs`
  3. `claurst-bridge` has characterization tests; the `AGENT_RUNNER` double-init panic is documented with `#[should_panic]`
  4. `cargo test --workspace` passes and `cargo insta test --check` shows no snapshot drift
**Reference**: `.planning/phases/08-query-and-bridge-orchestration/REFACTORING-REFERENCE.md` — Message Chains, Hide Delegate, Remove Middle Man, Extract Function (with `run_query_loop` sequencing guide)
**Plans**: TBD

### Phase 9: tui Crate Decomposition
**Goal**: The TUI codebase is navigable — `app.rs` is split into focused modules, the App struct fields are grouped into domain sub-structs, and render functions operate on focused sub-state rather than the whole App
**Depends on**: Phase 8
**Requirements**: BLOT-02, COUP-03
**Success Criteria** (what must be TRUE):
  1. Developer can modify a single TUI state domain (input, render, session) in `tui/src/app.rs` without touching unrelated fields — `App` is decomposed into `InputState`, `RenderState`, `SessionState` (or equivalent) nested sub-structs
  2. Developer can find render functions that take focused sub-state arguments rather than whole-App params — `render.rs` functions no longer receive whole-`App` where only a sub-state is needed
  3. `prompt_input.rs`, `overlays.rs`, and `dialogs.rs` are each split into named sub-modules under their respective directories
  4. `cargo insta test --check` passes with all existing TUI snapshots stable — no visual regression
**Reference**: `.planning/phases/09-tui-crate-decomposition/REFACTORING-REFERENCE.md` — Large Struct, Extract Module/Struct, Move Method, Move Field, Feature Envy (with App decomposition sequencing guide)
**Plans**: TBD
**UI hint**: yes

### Phase 10: commands Crate Decomposition
**Goal**: The 8,657-line commands/lib.rs is replaced by a directory of per-command files — adding a new slash command requires writing one new file, not editing a monolith
**Depends on**: Phase 9
**Requirements**: BLOT-01, DISP-02
**Success Criteria** (what must be TRUE):
  1. Developer can find any slash command implementation in `commands/src/slash/<name>.rs` — one file per command; `lib.rs` is a pure dispatch registry (~200 lines)
  2. Developer can add a new slash command `execute()` body without copy-pasting the guard/format boilerplate — the pattern lives in one shared helper in `commands/src/`
  3. `cargo test -p claurst-commands` passes and `cargo insta test --check` shows no snapshot drift
**Plans**: TBD

### Phase 11: cli Crate Extraction
**Goal**: `cli/src/main.rs` is a thin wiring file — logic that belongs in library crates has been extracted, and adding a new LLM provider requires editing one registration file
**Depends on**: Phase 10
**Requirements**: COUP-02
**Success Criteria** (what must be TRUE):
  1. Developer can add a new LLM provider by editing one provider registration file instead of making changes in 5+ scattered locations across `api` and `cli`
  2. `McpToolWrapper` lives in `crates/tools/`; OAuth dispatch and MCP init live in `cli/src/startup/` — `main.rs` is thin wiring
  3. `cargo test -p claurst-cli` passes; CLI smoke tests (`--help`, `--version`) still match committed snapshots
**Plans**: TBD

### Phase 12: Long Method Final Pass
**Goal**: Every function across all 12 crates is readable without scrolling — cognitive complexity 15, length 80, and argument count 6 thresholds are met workspace-wide
**Depends on**: Phase 11
**Requirements**: BLOT-04
**Success Criteria** (what must be TRUE):
  1. `cargo clippy --workspace` reports zero violations of the cognitive complexity 15, function length 80, and argument count 6 thresholds set in `clippy.toml`
  2. Developer can read any single function in `render.rs`, `query/src/lib.rs`, `cli/src/main.rs`, or `commands/src/` and understand it without scrolling
  3. `cargo test --workspace` passes and `cargo insta test --check` shows no snapshot drift
**Plans**: TBD

### Phase 13: Cross-Cutting Cleanup
**Goal**: The workspace has zero unexplained suppressed warnings — every `#[allow(dead_code)]` is either deleted (unused code removed) or has a documented rationale
**Depends on**: Phase 12
**Requirements**: DISP-01
**Success Criteria** (what must be TRUE):
  1. `grep -r '#\[allow(dead_code)\]' crates/` returns zero results for suppressions without a `// load-bearing:` or `// intentional:` comment
  2. `cargo check --workspace` produces zero `dead_code` warnings without suppression attributes
  3. `cargo test --workspace` passes and `cargo insta test --check` shows no snapshot drift — final behavioral anchor confirms refactoring is complete
**Plans**: TBD

## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1. Welcome Screen Fix | v1.0 | 1/1 | Complete | 2026-05-05 |
| 2. Fix UAT gaps: thinking block and welcome routing | v1.0 | 2/2 | Complete | 2026-05-07 |
| 3. ANTHROPIC_AUTH_TOKEN Bearer Auth Support | v1.0 | 6/6 | Complete | 2026-05-09 |
| 4. Characterization Test Infrastructure | v1.1 | 0/? | Not started | - |
| 5. core Crate Decomposition | v1.1 | 0/? | Not started | - |
| 6. Layer 1 Protocol Crates | v1.1 | 0/? | Not started | - |
| 7. tools Crate Cleanup | v1.1 | 0/? | Not started | - |
| 8. query and bridge Orchestration | v1.1 | 0/? | Not started | - |
| 9. tui Crate Decomposition | v1.1 | 0/? | Not started | - |
| 10. commands Crate Decomposition | v1.1 | 0/? | Not started | - |
| 11. cli Crate Extraction | v1.1 | 0/? | Not started | - |
| 12. Long Method Final Pass | v1.1 | 0/? | Not started | - |
| 13. Cross-Cutting Cleanup | v1.1 | 0/? | Not started | - |

---
*Last updated: 2026-05-13 — v1.1 roadmap created*
