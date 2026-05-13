# Domain Pitfalls

**Domain:** Rust rewrite of Claude Code CLI (claurst) — v1.1 Codebase Refactoring Milestone
**Researched:** 2026-05-13
**Confidence:** HIGH (codebase-verified for Rust-specific pitfalls; MEDIUM for AI-code-specific patterns)

---

> **Note:** This file replaces the v1.0 pitfalls file which covered security and feature parity. The v1.1 milestone is a structured refactoring of the AI-generated codebase. The pitfalls here are specific to that context: extracting methods from ownership-tangled code, removing clone/Arc<Mutex<>> spam, replacing dyn where generics suffice, eliminating unwrap(), and avoiding introducing new smells while fixing old ones. The original security pitfalls from v1.0 remain tracked in `.planning/milestones/v1.0-phases/`.

---

## Critical Pitfalls

Mistakes that cause test suite invalidation, crate interface breakage, or silent behavioral regressions.

---

### Pitfall 1: Method Extraction Breaks the Borrow Checker Across Function Boundaries

**What goes wrong:** Extracting a block of code into a new method causes the borrow checker to analyze only the method *signature*, not its implementation. Information about which specific fields are borrowed is lost at the function boundary. Code that compiled inline because the compiler could see field-level split borrows now fails because the extracted method signature forces a whole-struct borrow.

For example: if `fn get_children(&self) -> &Vec<Child>` exists on a struct, calling it borrows the entire `self` — even if the method only touches `self.children`. Any simultaneous mutable borrow of `self.other_field` in the same scope now fails, even though the underlying access patterns are non-conflicting.

**Why it happens:** Rust's borrow checker operates on signatures at call sites, not on bodies. The "split borrow" optimization is scope-local; it does not cross function call boundaries. AI-generated code is especially prone to this because LLMs frequently produce large monolithic functions (because context fits in one generation window), then later produce accessor methods that fight the very split borrows the monolithic functions relied on.

**Warning signs:**
- Refactoring a large function into helper methods produces `cannot borrow \`self\` as mutable because it is also borrowed as immutable` errors that did not exist before
- The original monolithic function accessed multiple fields of a struct in non-overlapping ways
- Getter/setter methods were added to "clean up" direct field access

**Prevention strategy:**
- Before extracting a method, map all field accesses in the target block. If multiple fields are accessed in the same scope, direct field access (possibly with `pub(crate)` visibility) is safer than getters
- When a method must return a reference to a field, document that it forces a whole-struct borrow; callers must drop the result before any mutable borrow
- Prefer extracting pure functions (taking owned or copied values) over methods taking `&self` when the extracted logic does not need struct context
- Use the pattern: extract the data you need into owned/copied locals *first*, then call the new method with those locals — instead of passing `&self` to the extracted method

**Phase mapping:** Bloater elimination (extract long methods) — every extraction in a struct with complex borrow patterns requires pre-analysis.

---

### Pitfall 2: Removing clone() Exposes Hidden Lifetime Complexity — Replacement May Be Worse

**What goes wrong:** AI code uses `.clone()` to resolve every borrow checker conflict. Removing a clone seems like a simple improvement, but it forces the compiler to track the original reference's lifetime through all callers — often requiring lifetime annotations to be added to multiple function signatures, struct fields, and trait implementations. The "fix" cascades across the codebase and takes far longer than expected.

In a 12-crate workspace, a lifetime annotation added to a struct in `crates/core` may propagate to `crates/query`, `crates/tui`, and `crates/commands`. What looks like a 5-minute change becomes a multi-crate refactor.

**Why it happens:** `.clone()` is a lifetime eraser — it terminates the borrow chain and starts fresh. When you remove it, you have to thread the actual lifetime through every site that previously relied on the clone to reset it. In AI-generated code, clones are often inserted specifically because the LLM could not solve the lifetime problem during generation; removing them re-surfaces the original problem.

**Warning signs:**
- A `.clone()` call is on a non-trivial type (String, Vec<T>, HashMap) inside a hot path (query loop, streaming handler, TUI render tick)
- The value being cloned is used in a function that returns a reference
- Removing the clone produces lifetime errors in callers you did not touch

**Prevention strategy:**
- Start with profiling-informed clone removal: use `cargo flamegraph` or `criterion` benchmarks to identify clones that are actually performance problems, not just style issues
- For clones that are "lifetime erasers," consider whether the real fix is restructuring ownership (e.g., storing the owned value in the struct rather than borrowing it) vs. simply propagating a reference
- Do clone removals one at a time. Batch removal is the fastest path to a broken build with unclear root causes
- The rule: if removing a clone requires adding lifetime parameters to more than 2 function signatures in different modules, reconsider whether the clone should stay or whether the struct design needs to change first

**Phase mapping:** Dispensable cleanup (remove clone spam) — every clone removal is a candidate for this cascade; treat as high-effort by default.

---

### Pitfall 3: Replacing Arc<Mutex<T>> Deadlocks Tokio Under async

**What goes wrong:** AI-generated Rust code that was ported from TypeScript/JavaScript commonly wraps shared state in `Arc<Mutex<T>>` or `Arc<RwLock<T>>`. The refactoring instinct is to replace this with owned state passed through function parameters. However, the refactoring may introduce a subtler problem: `std::sync::Mutex` held across an `.await` point.

If `std::sync::Mutex` (not `tokio::sync::Mutex`) is still used after refactoring, and a lock guard is held across an `.await` point, the Tokio executor can schedule another task on the same thread that then tries to acquire the same lock — causing a deadlock. Some `MutexGuard` implementations are `Send`, meaning this compiles without error but deadlocks at runtime.

**Why it happens:** Refactoring moves code around without tracking which mutexes are `std::sync` vs `tokio::sync`. The deadlock is runtime-only and non-deterministic — it may not appear in tests but manifests under load.

**Warning signs:**
- `std::sync::Mutex` or `std::sync::RwLock` lock guard is stored in a `let` binding above an `.await` expression in the same scope
- Tests pass but the application deadlocks intermittently in integration scenarios
- A refactor moved async code into a closure or helper function that now holds a guard across an await boundary it did not before

**Prevention strategy:**
- Audit every `std::sync::Mutex` (and `std::sync::RwLock`) in the codebase. For each: does any code path hold a guard across `.await`? If yes, migrate to `tokio::sync::Mutex` or restructure to drop the guard before the await
- The claurst workspace already uses `parking_lot::Mutex` in several places (noted in CONCERNS.md). `parking_lot::Mutex` is sync-only; same rules apply
- The safest pattern for shared state in Tokio: wrap `Arc<Mutex<T>>` in a newtype struct with synchronous methods only. Never expose the guard outside those methods. This guarantees no guard crosses an `.await`
- For state that only needs ownership transfer (not sharing), use message-passing via `tokio::sync::mpsc` or `tokio::sync::oneshot` instead of shared mutexes entirely

**Phase mapping:** Coupler fixes (removing excessive shared state), Bloater elimination (simplifying long parameter lists that hide Arc<Mutex<T>> passing) — async correctness audit required before and after.

---

### Pitfall 4: Replacing dyn Trait with Generics Causes Monomorphization Code Explosion and Compile Time Regression

**What goes wrong:** AI code frequently uses `Box<dyn Trait>` where static dispatch with generics would suffice — because LLMs are trained on examples that mix the two patterns. Replacing `dyn Trait` with `impl Trait` or `<T: Trait>` generics is the idiomatic fix. However, in a 12-crate workspace with `LlmProvider` trait used across `api`, `query`, `tui`, `commands`, `bridge`, and `acp` crates, converting the central trait to a generic parameter causes monomorphization: the compiler generates separate code for each concrete provider type. This can multiply compile times and binary size significantly.

The real cost appears at link time and in incremental build times — breaking the inner dev loop for contributors.

**Why it happens:** Monomorphization is zero-cost at runtime but not at compile time. In a workspace where the trait is used at many call sites across many crates, each new concrete type multiplies the code generated. The claurst `LlmProvider` trait, for example, is likely used in query loop hot paths — a good candidate for `dyn` (heterogeneous, runtime-selected provider) rather than generics (homogeneous, compile-time-selected).

**Warning signs:**
- A trait has only 2-5 concrete implementations but is used at dozens of call sites across many crates
- The concrete type implementing the trait is selected at runtime (from config/CLI args), not at compile time
- After replacing `dyn` with generics, `cargo build` time increases by more than 20%

**Prevention strategy:**
- Apply the decision rule: use `dyn Trait` when the concrete type is chosen at runtime (config-driven, user-selected); use generics when the concrete type is fixed at compile time or when you need multiple trait bounds on the same type
- For `LlmProvider` specifically: keep `dyn LlmProvider` — provider selection is always runtime-based (config, env vars, CLI flag). This is a correct use of dynamic dispatch
- Audit uses of `Box<dyn Trait>` where the trait has only one concrete implementation and the type is always known at compile time — these are legitimate targets for conversion
- After any `dyn`→generic conversion, run `cargo build --timings` to measure compile time impact before committing

**Phase mapping:** Dispensable cleanup (removing speculative over-abstraction) — but this pitfall runs in the opposite direction. Not all `dyn` usage is wrong; some is appropriate.

---

### Pitfall 5: Replacing unwrap() Surfaces Hidden Invariant Assumptions — Fixing the Wrong Layer

**What goes wrong:** ~410 `.unwrap()` calls exist in the production codebase (noted in CONCERNS.md). The naive replacement is: change every `.unwrap()` to `?` or `.expect("reason")`. But `.unwrap()` in AI code often masks an architectural assumption: the code assumes a value is always `Some` or `Ok` because of invariants maintained elsewhere. Replacing with `?` propagates an error that should be impossible — creating error paths that will never trigger and obscuring what the actual invariant was.

Worse: some `unwrap()` calls are on `Option` types in data structures that should be redesigned to not be optional at all. Replacing `.unwrap()` with `?` in those cases papers over the real fix (making the type `T` instead of `Option<T>`) and adds noise to error handling paths.

**Why it happens:** LLMs generate `Option<T>` and `Result<T>` fields defensively because it is "safe" code. Then they call `.unwrap()` everywhere because the defensive wrapping was unnecessary. The correct fix is two-layered: first determine if the wrapper type was needed, then remove unwrap.

**Warning signs:**
- `.unwrap()` is called on a field that is always set during construction and never unset (`Option<T>` field that is always `Some` after `new()`)
- `.unwrap()` is called immediately after an insertion (`map.insert(k, v); map.get(&k).unwrap()`)
- A batch of `unwrap()` → `?` replacements introduces new `Result` return types on functions that previously returned `()` or a plain type — requiring callsites to be updated

**Prevention strategy:**
- Before replacing an `.unwrap()`, determine *why* it exists: (a) genuine error handling deferred by the author, (b) invariant the author believed always holds, (c) `Option<T>` field that should be `T`
- For case (a): replace with `?` and add an appropriate error type
- For case (b): replace with `.expect("invariant: X because Y")` — this preserves the invariant documentation and makes panics informative rather than silent
- For case (c): redesign the type (remove `Option`), which is the correct fix but requires understanding all construction paths
- Never batch-replace all `unwrap()` with `?` in a single commit. Each replacement requires a reasoning step

**Phase mapping:** Bloater elimination and Dispensable cleanup — categorize each unwrap before replacing; do not use sed/find-replace for this class of change.

---

### Pitfall 6: Characterization Tests Written at the Wrong Granularity

**What goes wrong:** Characterization tests written before refactoring are the behavior anchor. If tests are written at too coarse a granularity (CLI snapshot only, end-to-end only), they will fail to catch internal regressions that do not affect the final CLI output but break intermediate state. If written at too fine a granularity (every private function), they couple the test suite to implementation details and every refactor breaks 50 tests.

**Why it happens:** Writing characterization tests for an AI-generated codebase is particularly tricky because AI code has many private helper functions that implement non-obvious behavior. It is tempting to test every helper directly. But if you do, renaming a function or changing its signature during refactoring invalidates those tests — defeating their purpose.

**Warning signs:**
- Characterization test suite has more than 50% test coverage on private functions (use `#[cfg(test)]` `pub` to expose them)
- Moving a function from one module to another during refactoring breaks tests that are not testing the behavior being changed
- End-to-end tests pass but unit tests fail — indicating the tests are testing internals rather than behavior

**Prevention strategy:**
- Write characterization tests at the public interface of each crate (pub functions, trait implementations). These are the interfaces that matter for correctness; refactoring internal details should not break them
- For complex internal logic that is hard to test through the public interface, write tests against the behavior as exercised through the public API — not by making private functions public just to test them
- Snapshot tests (using `insta` or similar) for CLI output and TUI render output are appropriate characterization tests; they capture "what the system does" without coupling to how
- Use `cargo test --doc` to ensure all doc-test examples are part of the characterization suite

**Phase mapping:** Characterization test phase (prerequisite to all refactoring) — this is the first phase and determines whether the subsequent refactoring can proceed safely.

---

## Moderate Pitfalls

Mistakes that cause regressions, contributor friction, or refactoring-induced smells.

---

### Pitfall 7: Crate Interface Breakage Silently Compiles if Consumers Are in the Same Workspace

**What goes wrong:** In a Cargo workspace, all crates are compiled together. Removing a `pub` function from `crates/core` that is used in `crates/tui` will fail to compile — which is correct. However, removing a `pub` function that is *not currently used* in the workspace will compile successfully, even if it is part of the intended public API for future external consumers or upstream contributors.

In the claurst context, contributors depend on the stable crate interfaces. A refactoring that removes a `pub` symbol they use will break their forks without any local compilation warning.

**Why it happens:** Rust's visibility rules check compilation, not API contracts. `cargo build` does not warn you that you removed a previously-exported function — it only fails if a current workspace consumer uses it.

**Prevention strategy:**
- Use `cargo-semver-checks` before any PR that touches `pub` API surface: `cargo install cargo-semver-checks && cargo semver-checks check-release`
- Document which crate APIs are considered stable (used by forks/upstreams) vs internal-only. Use `#[doc(hidden)]` for technically-pub but not-API-contract symbols
- When removing `pub` functions during refactoring, add a one-release deprecation cycle via `#[deprecated(since = "1.1.0", note = "use X instead")]` for any function that appears in the spec or contributor documentation
- Review `pub(crate)` vs `pub` — over-use of bare `pub` in AI code is common (the LLM defaults to making things public to avoid compilation errors during generation). Downgrading to `pub(crate)` where appropriate reduces the stable API surface

**Phase mapping:** All refactoring phases — run `cargo-semver-checks` as a gate on every phase that modifies public function signatures.

---

### Pitfall 8: Shotgun Surgery During Module Splits — Imports Cascade Across All 12 Crates

**What goes wrong:** Splitting a monolithic file (e.g., `crates/commands/src/lib.rs` at 8,576 lines) into per-module files requires updating every `use` statement that previously imported from `commands::*`. In a 12-crate workspace where commands are used in `tui`, `core`, `cli`, and `mcp`, the import cascade touches dozens of files.

The risk is not just mechanical churn — it is that during the import cascade, some `use` statements are updated incorrectly (wrong module path), and the error only surfaces at a call site far from the renamed item.

**Why it happens:** Rust import paths are absolute within a crate (`crate::commands::some_fn`) and relative within a module (`super::some_fn`). When moving items between modules, both path types need updating. AI-generated code often mixes these styles unpredictably.

**Warning signs:**
- `lib.rs` has more than 2,000 lines and is the only file in its crate's `src/`
- `grep -r "use crate::" crates/ | wc -l` returns a large number — indicates many direct path imports that will all need updating
- The file being split is imported with `*` (glob import) anywhere in the workspace

**Prevention strategy:**
- Before splitting a large file, run `cargo test` and record the test count as a baseline
- Split in small increments: move one logical group at a time (one command, one widget) rather than attempting a complete reorganization in one commit
- Use `pub use` re-exports in the original location to maintain backward compatibility during the split: `pub use self::new_module::SomeType;` in `lib.rs` until all callers are updated
- After each sub-split, run the full test suite before proceeding to the next

**Phase mapping:** Coupler and Change Preventer fixes (splitting monolithic files) — the most mechanically risky category of refactoring in this codebase.

---

### Pitfall 9: Fixing Feature Envy by Moving Code Changes Trait Implementations

**What goes wrong:** "Feature Envy" — a function in module A that uses mostly data from module B — is a legitimate code smell. The fix is to move it to module B. But in Rust, moving a method to a different module may require moving it to a different `impl` block, which may require implementing a trait on a type from another crate, which Rust prohibits (the orphan rule).

In claurst, this is likely to occur with `LlmProvider` trait implementations: if a method in `crates/query` exhibits Feature Envy by primarily operating on types from `crates/api`, moving it to `crates/api` may require adding a method to a trait that is defined in a third crate — violating the orphan rule.

**Why it happens:** The orphan rule (`impl Trait for Type` requires either `Trait` or `Type` to be local to the crate) prevents a large class of "just move this method" refactors from compiling. AI code ignores this constraint during generation.

**Warning signs:**
- The method you want to move uses a type from a different crate as its primary receiver
- The target location for the method would require `impl ForeignTrait for ForeignType`
- Moving the method requires adding a new trait method, which would be a breaking API change

**Prevention strategy:**
- When Feature Envy crosses crate boundaries, the fix is usually a new wrapper type (newtype pattern) or a new free function, not a method move
- Alternatively, introduce an extension trait: `trait LlmProviderExt: LlmProvider { fn envious_method(&self) { ... } }` in the crate where the method logically belongs
- Document the crate boundary in the code: `// Note: logic here belongs conceptually to crates/api but cannot be moved due to orphan rule — see ARCHITECTURE.md`

**Phase mapping:** Coupler fixes (Feature Envy, Inappropriate Intimacy) — crate boundary awareness required before any cross-crate method move.

---

### Pitfall 10: Removing "Speculative Generality" Over-Abstractions Can Leave Callers with No Alternative

**What goes wrong:** AI code frequently generates "speculative generality" — unused generic parameters, traits with one implementation, and abstract factory patterns for things that will only ever have one concrete form. The refactoring instinct is correct: remove them. But some of these abstractions are used in test doubles or mocking setups that are not immediately obvious from a codebase grep.

Removing an abstraction that is "only used once" in production code may break a test helper that the author intended to swap in a fake. After removal, the test suite cannot compile.

**Why it happens:** Test mocks and fakes are consumers of abstraction. If the characterization test suite is written *after* the speculative abstractions are removed, the problem is not discovered. If tests are written *before* (as intended in this milestone), the problem surfaces correctly during the test-writing phase — before any code is deleted.

**Warning signs:**
- A trait has one concrete implementation in production code but is used in `#[cfg(test)]` blocks or in a `tests/` directory with a different implementation
- An abstract factory or builder pattern has a single `build()` variant and no variants in the codebase history

**Prevention strategy:**
- The characterization test phase (first phase) must be completed before any dispensable cleanup. Tests will surface which abstractions are load-bearing for testing even if not for production
- Before removing any trait, run: `grep -r "TraitName" . --include="*.rs"` including in `tests/` and `benches/` directories
- If a trait is used only in tests, consider whether it is testing the right thing — test-only abstractions can often be replaced with direct struct construction

**Phase mapping:** Characterization test phase gates all dispensable cleanup. Do not delete speculative abstractions before the test suite is complete.

---

### Pitfall 11: Introducing New Primitive Obsession While Fixing the Old

**What goes wrong:** The fix for primitive obsession (strings used as identifiers, ints used as enums) is to introduce newtype wrappers or proper enums. But the refactoring itself can introduce new primitive obsession if done carelessly. For example, replacing `String` provider names with a `ProviderName(String)` newtype wrapper, and then also replacing session IDs with a `SessionId(String)` newtype — but using the same inner representation `String` for both with no conversion boundary. Now `ProviderName` and `SessionId` are different types that are still both just strings, and the codebase has twice as many conversions between them.

**Why it happens:** Newtype wrappers added by refactoring are often added per-type rather than per-domain concept. Multiple newtypes wrapping the same primitive with similar behavior proliferate conversion boilerplate without actually improving type safety at the boundaries that matter.

**Warning signs:**
- Two or more newtype structs are defined with identical inner types and identical `From`/`Into` implementations
- The only methods on a newtype are `new(inner: T) -> Self`, `inner(&self) -> &T`, and `Display`
- A function accepts multiple newtype parameters of the same inner type — callers can still pass them in the wrong order without a compile error

**Prevention strategy:**
- Before introducing a newtype, ask: does this type participate in domain logic that validates or constrains the inner value? If yes (e.g., `ModelId` must match a known model name), the newtype is worth it. If no (e.g., `SessionId` is just an opaque string), a type alias `type SessionId = String` may serve the documentation purpose without the boilerplate
- Group related newtype definitions in a single `types.rs` module to make proliferation visible
- After each refactoring phase, grep for `struct [A-Z][A-Za-z]+(String)` or similar patterns and count newtype wrappers as a metric

**Phase mapping:** Bloater elimination (primitive obsession fix) — requires discipline to not over-apply the pattern.

---

### Pitfall 12: "Big Bang" Multi-Crate Refactoring Stalls Due to Compile Cycle

**What goes wrong:** If refactoring changes are batched across multiple crates in one PR (e.g., renaming a type in `crates/core` and updating all 11 consumers simultaneously), the PR becomes a 2,000-line diff that is difficult to review, impossible to bisect when something breaks, and prone to merge conflicts if other work is in flight.

More critically: if the PR fails CI (a test breaks), rolling back requires reverting all 12 crates simultaneously — or carefully cherry-picking revert commits. In a codebase where contributors are actively working, a stalled multi-crate PR blocks the entire team.

**Why it happens:** Type renames and interface changes *appear* to require atomic multi-crate updates because all crates must compile. This pressure pushes toward large PRs.

**Prevention strategy:**
- Use the "bridge" pattern: add the new interface alongside the old, update consumers one crate per PR, then remove the old interface in a final cleanup PR. This keeps each PR small and the codebase always compilable
- For renames specifically: use `#[deprecated]` + `pub use` re-exports to allow the rename to be incremental: (1) add new name, (2) deprecate old name, (3) update consumers, (4) remove old name
- Set a PR size budget: no more than 3 crates changed in a single PR, no more than 500 lines diff for structural changes
- Use `cargo fix --allow-dirty` for mechanical transformations (import path updates) — it handles the boilerplate and produces reviewable, targeted diffs

**Phase mapping:** All phases — establish the incremental PR discipline in the first phase so it becomes the norm.

---

## Minor Pitfalls

Issues that are annoying but bounded in impact.

---

### Pitfall 13: Cargo Feature Flag Interactions Break After Refactoring

**What goes wrong:** The claurst workspace has 36 feature flags. When a module is split or a type is moved to a different crate, the feature flag that conditionally compiled the original code may not be applied to the new location. The refactored code compiles in `dev_full` (all features enabled) but fails in a feature-limited build.

**Prevention strategy:**
- After every module split or type relocation, run `cargo check --no-default-features` and `cargo check --features default` in addition to `cargo check --all-features`
- Add a CI step that runs `cargo check` for each non-default feature in isolation: `cargo check --features voice`, `cargo check --features bridge`, etc.

**Detection:** Warning sign: CI `dev_full` passes but a user reports a compilation error with a specific feature enabled.

**Phase mapping:** All structural refactoring phases — add these check commands to the phase-exit gate.

---

### Pitfall 14: Test Helper Duplication Grows During Refactoring

**What goes wrong:** Each crate in the workspace has its own test utilities. During refactoring, characterization tests are added to each crate. Without a shared test utilities crate, each test crate implements its own fixture builders, fake providers, and assertion helpers. By the end of refactoring, the test code itself becomes a maintenance problem.

**Prevention strategy:**
- Create a `crates/test-utils` (or `crates/testing`) crate early in the characterization test phase
- Common fakes (`FakeLlmProvider`, `FakeSessionStore`, `MockMcpServer`) belong in `test-utils`, not repeated in each crate's `tests/` directory
- Gate the `test-utils` crate behind `cfg(test)` at the workspace level — it should not appear in release binaries

**Phase mapping:** Characterization test phase — establish the shared test utilities crate in the first phase.

---

### Pitfall 15: Visibility Downgrades (pub → pub(crate)) Silently Break Fork Contributors

**What goes wrong:** AI-generated code defaults to `pub` visibility to avoid compilation errors during generation. During refactoring, making items `pub(crate)` is correct hygiene. But the claurst project has fork contributors (kuberwastaken/claurst upstream) who may be importing these items from their own code. Downgrades from `pub` to `pub(crate)` are breaking changes even if they are not breaking changes within the workspace.

**Prevention strategy:**
- Run `cargo-semver-checks` before any visibility downgrade
- Audit the upstream fork for any imports of the symbols being downgraded before committing
- If a symbol was previously `pub` but is not in the documented API, add it to an internal API list and announce the visibility change in the release notes for v1.1

**Phase mapping:** All phases — particularly Dispensable cleanup and Coupler fixes which are most likely to downgrade visibility.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Characterization test suite | Tests written at wrong granularity (Pitfall 6) | Target public crate interfaces, not private helpers |
| Characterization test suite | Speculative abstractions removed before tests reveal their test use (Pitfall 10) | Never delete before test phase is complete |
| Bloater: extract long methods | Method extraction breaks split borrows (Pitfall 1) | Map field accesses before extracting; prefer direct field access over getters |
| Bloater: remove clone spam | Lifetime cascade across multiple crates (Pitfall 2) | Remove one clone at a time; profile first to prioritize hot-path clones |
| Bloater: long parameter lists | Replacing Arc<Mutex<T>> parameters with owned state exposes async deadlock (Pitfall 3) | Audit guard lifetimes vs .await points before restructuring |
| Dispensable: remove dyn overuse | Monomorphization explosion on core traits (Pitfall 4) | Keep dyn on runtime-selected traits (LlmProvider); target compile-time-fixed uses |
| Dispensable: unwrap cleanup | Replacing unwrap() papers over invariant type design issues (Pitfall 5) | Categorize each unwrap before replacing; do not batch |
| Dispensable: dead/dup code | Speculative abstractions may be load-bearing for test doubles (Pitfall 10) | Complete test suite first |
| Coupler: module splits | Import cascade across 12 crates (Pitfall 8) | Use pub use re-exports during split; move one group per commit |
| Coupler: Feature Envy across crates | Orphan rule prevents method moves (Pitfall 9) | Use extension traits or newtype wrappers instead |
| Change Preventer: Shotgun Surgery | Multi-crate PR stalls CI and blocks team (Pitfall 12) | 3-crate / 500-line PR budget; use deprecation bridges |
| All structural phases | Crate API breakage invisible within workspace (Pitfall 7) | Run cargo-semver-checks as phase exit gate |
| All structural phases | Feature flag coverage gaps after code moves (Pitfall 13) | Add no-default-features and per-feature cargo check |
| Primitive obsession fix | Newtype proliferation creates new boilerplate obsession (Pitfall 11) | Add newtypes only where domain validation or ordering safety matters |

---

## Sources

- Codebase audit: `.planning/codebase/CONCERNS.md` (2026-05-04) — HIGH confidence (410 unwrap() count, Arc<Mutex<T>> patterns, 36 feature flags)
- [Refactoring Rust Code to Avoid Borrow Checker Conflicts — Sling Academy](https://www.slingacademy.com/article/refactoring-rust-code-to-avoid-borrow-checker-conflicts/) — MEDIUM confidence
- [How to Avoid Fighting the Rust Borrow Checker — qouteall](https://qouteall.fun/qouteall-blog/2025/How%20to%20Avoid%20Fighting%20Rust%20Borrow%20Checker) — HIGH confidence (covers split-borrow and method extraction)
- [Clone to Satisfy the Borrow Checker — Rust Design Patterns (unofficial)](https://rust-unofficial.github.io/patterns/anti_patterns/borrow_clone.html) — HIGH confidence
- [Item 12: Understand the Trade-offs Between Generics and Trait Objects — Effective Rust](https://www.lurklurk.org/effective-rust/generics.html) — HIGH confidence
- [Advanced Rust Anti-Patterns — Medium/Lado Kadzhaia](https://medium.com/@ladroid/advanced-rust-anti-patterns-36ea1bb84a02) — MEDIUM confidence
- [How to Deadlock a Tokio Application with a Single Mutex — Turso](https://turso.tech/blog/how-to-deadlock-tokio-application-in-rust-with-just-a-single-mutex) — HIGH confidence
- [Tokio Shared State Tutorial — tokio.rs](https://tokio.rs/tokio/tutorial/shared-state) — HIGH confidence
- [Long-term Rust Project Maintenance — corrode.dev](https://corrode.dev/blog/long-term-rust-maintenance/) — HIGH confidence
- [SemVer Compatibility — The Cargo Book](https://doc.rust-lang.org/cargo/reference/semver.html) — HIGH confidence
- [cargo-semver-checks — crates.io](https://crates.io/crates/cargo-semver-checks) — HIGH confidence
- [Item 22: Minimize Visibility — Effective Rust](https://effective-rust.com/visibility.html) — HIGH confidence
- [Be Simple — corrode.dev](https://corrode.dev/blog/simple/) — MEDIUM confidence (generics/abstraction discipline)
- [Step-by-Step Guide: Refactoring a Large Rust Codebase — codenotary.com](https://codenotary.com/blog/step-by-step-guide-refactoring-a-large-rust-codebase-with-aiderdev-and-custom-llms) — MEDIUM confidence
- [Incremental vs Big-Bang Refactoring — Steemit](https://steemit.com/business/@quantuminfo/incremental-vs-big-bang-in-software) — LOW confidence (general principles, not Rust-specific)

---

*Pitfalls audit: 2026-05-13 — v1.1 Codebase Refactoring Milestone*
