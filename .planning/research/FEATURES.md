# Feature Landscape: Code Smell Catalog (Rust-Idiomatic)

**Domain:** Rust codebase refactoring — AI-generated Rust workspace
**Researched:** 2026-05-13
**Codebase:** claurst — 12-crate Rust workspace, ~354,938 lines

---

## How to Read This Document

Each smell category maps Fowler's Java original to its Rust equivalent. The "detection" column gives either a runnable grep pattern or a clippy lint name that fires on the symptom. The "refactoring technique" column names the operation (Fowler's vocabulary where applicable) and its Rust idiom.

Smells are ordered within each category: worst confirmed instances first.

---

## Category 1 — Bloaters

### 1.1 Long Method

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Long Method | Long `fn` body — deeply nested `match`/`if let` arms, 100+ line async functions mixing orchestration with business logic | `cargo clippy -- -W clippy::cognitive_complexity` (threshold 25); `cargo clippy -- -W clippy::too_many_lines` (threshold 100) | **Extract Function** — pull inner match arms and named logical steps into `fn step_name(args) -> Result<T>` helpers called from a thin orchestrator |

**Confirmed instances in this codebase:**

- `commands/src/lib.rs` — 20+ `execute` methods, some 300+ lines (e.g., line 1943: 301 lines, line 6659: 344 lines). The 344-line `execute` body is a single `match args` tree that should be split into `handle_<subcommand>` fns.
- `tui/src/app.rs` — `impl App` spans 4,853 lines containing 123 methods. The event-handling path mixes input parsing, state mutation, and render scheduling in the same function body.
- `commands/src/lib.rs:7765` — `format_status` is 264 lines despite only formatting a struct into a string; it embeds nested formatting helpers inline.
- `query/src/lib.rs:699` — cognitive complexity 156/25 per clippy.

**What it looks like:**

```rust
// SMELL: 300-line execute mixing dispatch, validation, formatting, and IO
async fn execute(&self, args: &str, ctx: &CommandContext) -> CommandResult {
    if args.is_empty() || args == "status" {
        // 40 lines of status formatting
    } else if args.starts_with("setup") {
        // 80 lines of setup logic
    } else if args.starts_with("configure") {
        // 60 lines of config logic
    } // ... 8 more branches
}

// REFACTORED: thin dispatcher + extracted helpers
async fn execute(&self, args: &str, ctx: &CommandContext) -> CommandResult {
    match args.split_whitespace().next().unwrap_or("status") {
        "status" | "" => self.show_status(ctx),
        "setup"       => self.run_setup(args, ctx).await,
        "configure"   => self.run_configure(args, ctx).await,
        _             => CommandResult::Error(format!("unknown subcommand: {args}")),
    }
}
```

---

### 1.2 Large Class

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Large Class | Monolithic `struct` + giant `impl` block mixing unrelated concerns (UI state, business logic, I/O coordination) | `wc -l src/file.rs` > 3000; count `impl TypeName` methods; `grep -c "pub fn\|fn " file.rs` | **Extract Module** / **Extract Struct** — move logically coherent field groups + their methods into a new `struct` in a new `mod`, expose via a delegation field or `impl Deref` |

**Confirmed instances:**

- `tui/src/app.rs` — `App` struct has 150 `pub` fields. `impl App` at line 1138 spans 4,853 lines with 123 methods covering: input handling, streaming updates, TUI rendering coordination, MCP auth, bridge events, permission dialogs, diff viewer, tool use overlay, session management. These are at least 6 separable concerns.
- `commands/src/lib.rs` — 8,657 lines total; all 30+ slash commands in one file; every command struct + its `impl SlashCommand` interleaved with private helpers shared ad-hoc.
- `core/src/lib.rs` — 4,291 lines with 25+ `pub mod` inline modules: `error`, `types`, `config`, `constants`, `context`, `permissions`, `cost`, etc. This is a grab-bag module masquerading as a coherent crate.

**What it looks like:**

```rust
// SMELL: App struct owns UI state, runtime handles, and business data together
pub struct App {
    pub config: Config,              // business
    pub messages: Vec<Message>,      // business
    pub input: String,               // UI
    pub scroll_offset: usize,        // UI
    pub is_streaming: bool,          // runtime state
    pub tool_use_blocks: Vec<...>,   // runtime state
    pub model_registry: ModelRegistry, // infrastructure
    // ... 143 more fields
}

// REFACTORED: split into coherent sub-structs
pub struct App {
    pub session: SessionState,       // messages, cost, session_id
    pub ui: UiState,                 // input, scroll, overlays
    pub runtime: RuntimeState,       // streaming, tool_use_blocks
    pub infra: InfraHandles,         // model_registry, mcp_manager
}
```

---

### 1.3 Primitive Obsession

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Primitive Obsession | Domain concepts stored as raw `String`, `u32`, `f64`, or `bool` instead of an enum or newtype; misuse detected only at runtime | `grep -n "model: String\|session_id: String\|provider: String\|status: String"` in struct definitions; `grep -n "\"anthropic\"\|\"openai\"\|\"ollama\"" src/` for string-dispatch | **Replace Primitive with Newtype** or **Replace Type Code with Enum** |

**Confirmed instances:**

- `CommandContext.session_id: String` — should be `SessionId(String)` newtype to prevent passing a `model_name` where a `session_id` is expected.
- `core/lib.rs` — `manager_model: String`, `executor_model: String` in `ManagedAgentConfig`. These are model IDs dispatched via string comparison; a `ModelId` newtype or `enum BuiltinModel` would catch typos at compile time.
- `core/lib.rs` — `status: String` on `ToolUseBlock`; a `ToolStatus` enum is partially defined but the string form still circulates.
- `commands/lib.rs` — `"anthropic"`, `"openai"`, `"ollama"`, `"bedrock"` string literals scattered through match arms for provider dispatch.

**What it looks like:**

```rust
// SMELL: session_id is just a String; easy to swap arguments
fn resume_session(session_id: String, model: String) { ... }
resume_session(model_name.clone(), session_id.clone()); // wrong order, compiles fine

// REFACTORED: newtypes make the swap a compile error
struct SessionId(String);
struct ModelId(String);
fn resume_session(id: SessionId, model: ModelId) { ... }
```

---

### 1.4 Long Parameter List

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Long Parameter List | Functions with 5+ parameters, especially when several parameters always travel together | `cargo clippy` fires `clippy::too_many_arguments` (default threshold 7); `grep -n "fn .*([^)]*,[^)]*,[^)]*,[^)]*,[^)]*,[^)]*,[^)]*)"` for 7+ params | **Introduce Parameter Object** — group co-traveling params into a `struct`; or **Preserve Whole Object** — pass the enclosing context struct instead of destructuring it |

**Confirmed instances:**

- `cli/src/main.rs:1329` — `run_interactive` takes 11 parameters: `config`, `settings`, `client`, `tools`, `tool_ctx`, `query_config`, `cost_tracker`, `resume_id`, `bridge_config`, `has_credentials`, `model_registry`. Clippy reports 11/7. These split naturally into `SessionConfig` (config + settings + cost_tracker) and `RuntimeHandles` (client + tools + model_registry).
- `core/src/import_config.rs:417,457,514,570` — four consecutive functions each taking 8-9 parameters. Clippy fires on all four.
- `tui/src/context_viz.rs:46` — 8 params; `tui/src/prompt_input.rs:523` — 8 params.
- `query/src/lib.rs:699` — 9 params.

**What it looks like:**

```rust
// SMELL: 11 params; callers must maintain exact ordering
async fn run_interactive(
    config: Config, settings: Settings, client: Arc<AnthropicClient>,
    tools: Arc<Vec<Box<dyn Tool>>>, tool_ctx: ToolContext,
    query_config: QueryConfig, cost_tracker: Arc<CostTracker>,
    resume_id: Option<String>, bridge_config: Option<BridgeConfig>,
    has_credentials: bool, model_registry: Arc<ModelRegistry>,
) -> Result<()>

// REFACTORED: parameter objects
struct SessionConfig { config: Config, settings: Settings, cost_tracker: Arc<CostTracker> }
struct RuntimeHandles { client: Arc<AnthropicClient>, tools: Arc<Vec<Box<dyn Tool>>>, model_registry: Arc<ModelRegistry> }
async fn run_interactive(session: SessionConfig, handles: RuntimeHandles, opts: RunOptions) -> Result<()>
```

---

### 1.5 Data Clumps

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Data Clumps | The same 3-4 fields appearing together in multiple structs or function signatures, never encapsulated as a unit | `grep -n "session_id.*model_name\|config.*messages.*cost"` across files; look for identical field triplets in separate struct definitions | **Extract Struct** — give the clump a name and a home |

**Confirmed instances:**

- `config`, `cost_tracker`, `messages` appear together in `CommandContext`, `App`, and the `run_interactive` parameter list. This is the session context clump — it should be a `SessionContext` struct.
- `manager_model: String`, `executor_model: String`, `executor_max_turns: u32`, `max_concurrent_executors: u32` in `ManagedAgentConfig` always travel as a group when configuring agent pairs.
- `session_id: String`, `session_title: Option<String>`, `remote_session_url: Option<String>` in `CommandContext` — these are session identity and should be `SessionIdentity`.

---

## Category 2 — Dispensables

### 2.1 Dead Code

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Dead Code | Unreachable match arms, `#[allow(dead_code)]` suppressions, unused struct fields, imported symbols never referenced | `grep -rn "#\[allow(dead_code)\]"` (32 hits in this repo); `cargo clippy` default `dead_code` lint; `cargo +nightly udeps` for unused dependencies | **Delete Dead Code** — remove outright; resist the urge to leave it "just in case" |

**Confirmed instances:**

- 32 `#[allow(dead_code)]` suppressions spread across: `tools/src/computer_use.rs` (3), `core/src/settings_sync.rs` (8), `core/src/remote_settings.rs` (1), `tools/src/web_fetch.rs`, `tools/src/todo_write.rs`, `bridge/src/lib.rs`, `tui/src/settings_screen.rs`.
- `core/src/settings_sync.rs` — 8 dead-code suppressions on a 447-line file suggests the whole sync subsystem may be mostly unused.
- 2 `#[allow(unused_imports)]` in `cli/src/oauth_flow.rs` and `commands/src/lib.rs`.
- `tools/src/agent_tool.rs` — 7 lines total; essentially a stub.

**Note:** Rust's compiler already catches unused private items. `#[allow(dead_code)]` on `pub` items means the code is either spec-driven scaffolding or genuinely dead and suppressions were added to silence the warning rather than fix it.

---

### 2.2 Duplicate Code

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Duplicate Code | Same logic copy-pasted into multiple `impl` blocks or match arms; often recognizable as near-identical `let x = if condition { A } else { B }` patterns | `grep -n "let error_marker"` (3 copies in `tui/src/message_copy.rs`); look for identical match arms in separate `execute` fns | **Extract Function** for intra-file duplication; **Move to Trait Default Method** for inter-impl duplication; **DRY via iterator adapters** for repeated transformation chains |

**Confirmed instances:**

- `tui/src/message_copy.rs` — `let error_marker = if is_error.unwrap_or(false) { ... }` appears verbatim at lines 47, 100, and 346 in the same file. Should be extracted to `fn error_marker(is_error: Option<bool>) -> &'static str`.
- All 30 slash command `execute` methods repeat the same `if args.is_empty() { return CommandResult::Error("Usage: ...") }` guard. A `fn require_args<'a>(args: &'a str, usage: &str) -> Result<&'a str, CommandResult>` helper would eliminate this.
- `format_status` and `format_cost` helpers defined as private inner functions inside `execute` blocks rather than as associated functions — prevent reuse.
- `query/src/lib.rs` has cognitive complexity 156/25 partly because the same "check streaming state, dispatch tool call, update token count" pattern is inlined in multiple code paths.

---

### 2.3 Lazy Class

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Lazy Class | A crate or module with so little responsibility it doesn't justify its existence as a separate unit | `find crates/ -name "*.rs" \| xargs wc -l \| sort -n \| head -20`; look for crates with < 300 lines total | **Inline Module** — collapse into its most natural consumer crate; or **Collapse Hierarchy** if it's a thin wrapper trait |

**Confirmed instances:**

- `acp/src/lib.rs` — 285 lines total; implements ACP JSON-RPC server. It is used only from `cli/src/main.rs`. Could be an `acp` module inside `cli` crate rather than a standalone crate, unless the interface is expected to be consumed externally.
- `tools/src/agent_tool.rs` — 7 lines, just a stub. Not yet a real tool implementation.
- `tools/src/formatter.rs`, `tools/src/synthetic_output.rs` — 53 and 61 lines respectively; each implements one tiny formatting function. Could fold into `tools/src/lib.rs`.

---

### 2.4 Data Class

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Data Class | A `struct` with all `pub` fields and no meaningful methods — behavior that should belong to the struct is implemented by its callers instead | `grep -n "^pub struct" src/` then check if the corresponding `impl` is absent or trivially thin; look for structs where `derive(Debug, Clone, Serialize, Deserialize)` is the entire impl surface | **Move Method** — find where callers manipulate the struct's fields and migrate that logic into `impl` methods on the struct |

**Confirmed instances:**

- `ToolUseBlock` in `tui/src/app.rs` — 10 `pub` fields including `status: ToolStatus`, but status transitions are managed entirely in `App::handle_tool_result`, an 80-line method on the 150-field `App` struct. `fn mark_complete(&mut self)` and `fn is_pending(&self) -> bool` belong on `ToolUseBlock`.
- Many inner structs in `core/src/lib.rs` types module (`ContentBlock`, `Message`, `UsageInfo`) are pure data with all behavior in the callers. Some logic like "does this message have tool use?" belongs on `Message`.
- `CommandContext` — all fields public, behavior elsewhere. `fn session_identity(&self) -> &SessionIdentity` would be a meaningful method.

---

### 2.5 Speculative Generality

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Speculative Generality | Over-engineered `Box<dyn Trait>`, excessive Cargo feature flags, unused generic type parameters, or traits with only one implementor | `grep -rn "Box<dyn " crates/ \| wc -l` (56 hits); `grep -rn "Arc<dyn " crates/` (36 hits); count feature flags in Cargo.toml | **Collapse Hierarchy** — if a trait has one implementor, inline it; **Remove Feature Flag** — if a `#[cfg(feature)]` is always enabled in practice, remove the gate; **Concretize Type** — replace `Arc<dyn Fn(...) + Send + Sync>` with a concrete callback struct when there is only one use |

**Confirmed instances:**

- 36 Cargo features in `Cargo.toml` controlling crate inclusion. Many feature-gated crates like `buddy` and `acp` are always compiled in practice. Feature flags that are never toggled off are dead configuration.
- `CommandContext.mcp_auth_runner: Option<Arc<dyn Fn(McpAuthSession) + Send + Sync>>` — an `Arc<dyn Fn>` exists to accommodate future callers. There is currently one call site. A concrete `McpAuthRunner` struct with a single `run` method is less speculative.
- `Box<dyn SlashCommand>` (56 hits) is legitimate for the command dispatch pattern. However the `aliases()` method on the trait returns `Vec<&str>` and only 4 of 30 commands override it — the other 26 use the default empty vec. This is legitimate, not speculative.
- `todo!()` / `unimplemented!()` — 2 instances found, both legitimate placeholders.

---

## Category 3 — Couplers

### 3.1 Feature Envy

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Feature Envy | A method or function that accesses fields of a foreign struct more than its own; an `fn` that belongs in a different module | `grep -c "ctx\." fn_body` vs `grep -c "self\." fn_body` — if ctx/foreign fields outnumber self fields; look for free functions that take `&SomeStruct` and do most of their work on that struct | **Move Function** — relocate the function into the `impl` of the struct it is most envious of |

**Confirmed instances:**

- `commands/src/lib.rs` — free functions like `text_from_content_blocks` (123 lines) operate entirely on `Vec<ContentBlock>`. This belongs on `Message` or as a method in `core::types`.
- `tui/src/app.rs` — `self.config.*` is accessed 25 times inside `App` methods; the config is not App's primary responsibility but App owns it as a field. Functions like `is_feature_enabled` read deeply into `config` and should be methods on `Config`.
- `query/src/lib.rs` — the query loop directly reads `claurst_core::effort::EffortLevel` via full path 6 times, building strings from it. This belongs in `EffortLevel::to_api_str()`.
- The `execute` functions in slash commands frequently call `ctx.config.*` more than any method on `self`. The command structs have no state; they are pure behavior on `CommandContext`. Moving the logic to `CommandContext` methods or a `CommandHandler` trait on the context would eliminate the envy.

---

### 3.2 Inappropriate Intimacy

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Inappropriate Intimacy | One module or crate directly reaching into the private internals of another via full module paths; callers bypassing the public API | `grep -rn "claurst_core::tasks::TaskStatus\|claurst_core::error::ClaudeError" crates/tools/` — direct path qualification instead of `use`; cross-crate `pub` fields accessed by non-owning crates | **Hide Delegate** — add a method on the owning type that encapsulates the access; **Move Field** — if the accessing crate needs the data, it should own it |

**Confirmed instances:**

- `tools/src/bash.rs` — uses `claurst_core::tasks::TaskStatus::Failed(...)` via 7 full path qualifications instead of `use claurst_core::tasks::TaskStatus`. Not just style — it reveals that `bash.rs` knows the shape of `TaskStatus` intimately enough to construct variants directly.
- `tools/src/lib.rs` — `claurst_core::error::ClaudeError::PermissionDenied(...)` constructed in 8 places. If the `tools` crate needs to raise permission errors, `core` should expose a constructor `ClaudeError::permission_denied(msg)` to decouple the variant name from callers.
- `tui/src/app.rs` — `claurst_api::AnthropicStreamEvent::ContentBlockDelta` pattern-matched directly at line 5217. The `tui` crate should not know the internal streaming event enum structure of `claurst_api`. A `tui`-facing event adapter in `query` would break this coupling.
- `cli/src/main.rs` uses `claurst_query::context_window_for_model` as a free function with direct model-name knowledge. This should be a method on `ModelId` or `ModelRegistry`.

---

### 3.3 Message Chains

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Message Chains | Chains of 4+ dot-method calls to navigate to a value; each step exposes the internal structure of the previous object | Look for `a.b().c().d().e()` in method bodies; `grep -n "\.[a-z_]*\.[a-z_]*\.[a-z_]*\.[a-z_]*\.[a-z_]*"` in source | **Hide Delegate** — add a shortcut method on the nearest object so callers do not need to know the full path; or **Law of Demeter** — pass the leaf value directly rather than the root object |

**Confirmed instances:**

- `tui/src/app.rs` — `self.config.settings.model_defaults.provider_id` style chains are common in the 150-field App struct. Each `.` step couples `App` to the internal layout of `Config`, `Settings`, and `ModelDefaults`.
- `commands/src/lib.rs` — `ctx.config.managed_agents.as_ref().map(|c| c.executor_model.clone())` — three levels of unwrapping a nested optional field. A `fn executor_model(&self) -> Option<&ModelId>` on `Config` would hide this.
- Provider-specific format functions: `ctx.config.settings.providers.get("anthropic").map(|p| &p.api_key)` — reaches four levels into the config tree. Should be `ctx.config.api_key_for_provider("anthropic")`.

---

### 3.4 Middle Man

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Middle Man | A module or struct whose entire job is forwarding calls to another struct, adding no value | Look for `impl Foo` where every method is `self.inner.method(args)` with no transformation; crates where 80%+ of `pub` surface is delegation | **Remove Middle Man** — let callers call the delegate directly, or use `impl Deref<Target=Inner>` if the wrapper exists for ownership reasons |

**Confirmed instances (moderate):**

- `acp/src/lib.rs` (285 lines) — the ACP server is a thin JSON-RPC wrapper over `claurst_query` operations. Almost every handler deserializes a request and calls a `query` function with the same arguments. If ACP is not expected to be a stable independent interface, its logic could live in `cli` or `query`.
- `buddy` crate companion accessor: several `pub fn get_species(&self) -> &Species` and similar methods that are single-line field getters with no transformation. Rust convention is to expose the field directly as `pub species: Species` or use `Deref` rather than wrapping every field in a getter.
- Note: `Box<dyn SlashCommand>` dispatch in `commands` is intentional polymorphism, not Middle Man. The distinction is whether the intermediary transforms or just forwards.

---

## Category 4 — Change Preventers

### 4.1 Divergent Change

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Divergent Change | A single module that must change for multiple unrelated reasons — adding a provider, adding a command, changing a UI widget — all require edits to the same file | Look for files > 3000 lines with multiple `pub mod` declarations; track which files have commits touching unrelated features | **Extract Module** — identify the distinct "reasons to change" and give each its own `mod` or crate; for `core/lib.rs`, each logical domain (types, config, error, permissions) should be its own file |

**Confirmed instances:**

- `core/src/lib.rs` (4,291 lines) — has 25+ inline `pub mod` declarations covering: error types, message types, config, permissions, cost tracking, auth, session storage, git utils, MCP templates, feature flags, IDE integration, update checking, skill discovery. These are independent domains. A change to the auth system should not require touching the same file as a change to message types.
- `tui/src/app.rs` (5,990 lines) — changes for: new keyboard shortcuts, new streaming event types, new overlay dialogs, new provider auth flows, new tool rendering — all land in the same file because `App` owns everything.
- `commands/src/lib.rs` (8,657 lines) — adding a new slash command, changing how existing commands format output, and modifying the command dispatch registry all require editing the same file. Each `SlashCommand` implementor should live in its own file under `commands/src/commands/`.

---

### 4.2 Shotgun Surgery

| Java name | Rust equivalent | Detection | Refactoring technique |
|---|---|---|---|
| Shotgun Surgery | A single logical change requires edits to many files — adding a new LLM provider means touching `api`, `query`, `commands`, `tui`, `cli`, and `core` | `grep -rn "\"anthropic\"\|\"openai\"\|\"ollama\"" crates/ --include="*.rs"` — count how many files contain provider string literals; adding a provider requires updating all of them | **Move Field** + **Extract Class** — centralize provider-specific behavior behind a `Provider` trait; define a single registry so new providers register once, not N times |

**Confirmed instances:**

- Provider string literals (`"anthropic"`, `"openai"`, `"ollama"`, `"bedrock"`) appear in: `api/`, `query/`, `commands/`, `tui/`, `core/`, and `cli/`. A new provider requires updating match arms in all six crates.
- `use claurst_core::` is imported in 11 crates (every crate uses core). Any change to `core`'s public types ripples everywhere. The inline `pub mod` structure makes it worse: renaming a type in `core::types` could require changes in all 11 dependents.
- `AppState`/`AppMode` enums (if added) would require match exhaustiveness updates across the `tui` crate's event handlers.

---

## Table Stakes vs Differentiator Refactors

These distinctions answer: "what must be done for the codebase to be maintainable?" vs "what elevates it to exemplary idiomatic Rust?"

### Table Stakes (must do — codebase is unmaintainable without these)

| Refactor | Smell | Why Blocking |
|---|---|---|
| Split `commands/src/lib.rs` into one file per command | Divergent Change, Large Class | Adding any command requires editing an 8,657-line file |
| Split `core/src/lib.rs` into real submodules | Divergent Change | 25 unrelated domains in one file; every PR touches it |
| Extract `App`'s 150-field struct into sub-structs | Large Class | Impossible to understand App's state machine; all methods compete for the same namespace |
| Extract parameter objects for 8+ param functions | Long Parameter List | `run_interactive(11 args)` is a callers nightmare; wrong-order bugs are silent |
| Remove/address 32 `#[allow(dead_code)]` suppressions | Dead Code | Suppression hides whether code is intentional scaffolding or garbage |
| Extract the 3 duplicate `error_marker` blocks | Duplicate Code | Same bug fix must be applied in 3 places |
| Replace `session_id: String` / `model: String` clumps with newtypes | Primitive Obsession | Argument swap bugs are invisible to the compiler |

### Differentiators (elevate to idiomatic Rust)

| Refactor | Smell | Why Valuable |
|---|---|---|
| `ToolStatus` enum replaces status strings everywhere | Primitive Obsession | Exhaustive match; compiler-enforced transitions |
| `Provider` trait with single registration point | Shotgun Surgery | New providers add one `impl`, not N match arms |
| `impl Message { fn has_tool_use(&self) }` | Data Class | Behavior migrates to where the data lives |
| `fn to_api_str() -> &'static str` on `EffortLevel` | Feature Envy | Eliminates 6 copies of the same `match effort` in `query` |
| `Arc<dyn Fn>` callbacks replaced with concrete handler structs | Speculative Generality | Removes one level of indirection; easier to trace in a debugger |
| Law-of-Demeter access methods on `Config` | Message Chains | `config.api_key_for("anthropic")` vs 4-step chain |
| `impl Deref<Target=Inner>` on thin wrappers | Middle Man | Eliminates boilerplate getter methods on `buddy` companion |

---

## Anti-Features: What Not to Build

| Anti-Feature | Why Avoid | What to Do Instead |
|---|---|---|
| God `Utils` module consolidating all helpers | Creates a new Large Class and Divergent Change target | Each helper belongs in the module whose data it operates on |
| Blanket `#[allow(clippy::too_many_arguments)]` | Hides the symptom without treating the disease | Introduce parameter structs |
| Extracting every `impl Trait` into its own crate | Lazy Class in reverse — too-fine crates add build overhead | Move small crates into their most natural consumer, not smaller units |
| Replacing `Box<dyn SlashCommand>` with an enum | Over-engineering; the trait is legitimate polymorphism | Keep the trait dispatch; just split files |
| Rewriting async runtime or ownership model | Not a code smell issue; high risk, zero smell benefit | Refactor structure, not mechanics |

---

## Detection Quick Reference (Grep and Clippy)

| Smell | Primary detection command |
|---|---|
| Long Method | `cargo clippy -- -W clippy::cognitive_complexity -W clippy::too_many_lines` |
| Large Class | `wc -l crates/*/src/*.rs \| sort -rn \| head -20` |
| Primitive Obsession | `grep -rn ": String," crates/*/src/*.rs \| grep -v "//\|test"` then review for domain concepts |
| Long Parameter List | `cargo clippy` (fires `too_many_arguments` at threshold 7) |
| Data Clumps | `grep -rn "session_id.*String\|model.*String\|provider.*String" crates/` |
| Dead Code | `grep -rn "#\[allow(dead_code)\]" crates/`; `cargo check 2>&1 \| grep "unused"` |
| Duplicate Code | `grep -n "let error_marker\|if args.is_empty()" crates/`; manual audit of 30 `execute` bodies |
| Lazy Class | `find crates/ -name "*.rs" \| xargs wc -l \| sort -n \| head -20` |
| Data Class | `grep -n "^pub struct" crates/ \| xargs -I{} grep -l "^impl {}"` — structs with no matching impl |
| Speculative Generality | `grep -rn "Arc<dyn Fn\|Box<dyn " crates/ \| wc -l`; check Cargo.toml feature count |
| Feature Envy | `cargo clippy -- -W clippy::cognitive_complexity` on functions accessing foreign types heavily |
| Inappropriate Intimacy | `grep -rn "claurst_core::.*::.*::" crates/` (full-path construction of foreign internals) |
| Message Chains | `grep -rn "\.[a-z_]*\.[a-z_]*\.[a-z_]*\.[a-z_]*\." crates/` |
| Middle Man | Manual: find `impl Foo` where all methods are single-line `self.inner.method()` |
| Divergent Change | `wc -l crates/*/src/lib.rs \| sort -rn` + `grep -c "^pub mod" crates/core/src/lib.rs` |
| Shotgun Surgery | `grep -rn "\"anthropic\"\|\"openai\"\|\"ollama\"" crates/ --include="*.rs" \| awk -F: '{print $1}' \| sort -u` |

---

## Sources

- Codebase direct inspection: `crates/commands/src/lib.rs`, `crates/tui/src/app.rs`, `crates/core/src/lib.rs`, `crates/cli/src/main.rs` (2026-05-13)
- `cargo clippy` output (rust-clippy 1.95.0): `too_many_arguments`, `cognitive_complexity`, `dead_code`, `needless_borrow` warnings per-crate
- `cargo clippy -- -W clippy::cognitive_complexity`: 17 functions above threshold 25; top 3 at complexity 149, 156, 163
- Fowler, M. — *Refactoring: Improving the Design of Existing Code* (2nd ed.) — smell taxonomy
- [Clippy Lint Index](https://rust-lang.github.io/rust-clippy/stable/index.html) — `clippy::too_many_arguments`, `clippy::cognitive_complexity`, `clippy::too_many_lines`
- [idiomatic-rust](https://github.com/mre/idiomatic-rust) — newtype pattern, builder pattern references (MEDIUM confidence — community collection)
