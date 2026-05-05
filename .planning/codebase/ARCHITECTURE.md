<!-- refreshed: 2026-05-05 -->
# Architecture

**Analysis Date:** 2026-05-05

## System Overview

```text
┌─────────────────────────────────────────────────────────────────────┐
│                  crates/cli  (binary: claurst)                       │
│           Entry point, arg parsing (clap), mode selection            │
└──────┬──────────────────────┬────────────────────────────┬───────────┘
       │ headless/print        │ interactive TUI             │ ACP stdio
       ▼                       ▼                            ▼
┌────────────┐     ┌──────────────────────┐     ┌─────────────────────┐
│  query     │◄────│   tui  (ratatui)     │     │   acp               │
│  loop      │     │  App state, render,  │     │  JSON-RPC 2.0 over  │
│ crates/    │     │  event loop          │     │  stdio for editors   │
│ query      │     │  crates/tui          │     │  crates/acp         │
└──────┬─────┘     └──────────────────────┘     └─────────────────────┘
       │
       ├─────────────────────────────────────┐
       ▼                                     ▼
┌────────────────┐                  ┌─────────────────────┐
│  api           │                  │  tools              │
│  LLM provider  │                  │  Tool impls: bash,  │
│  abstraction   │                  │  file_read/write,   │
│  + streaming   │                  │  web, computer_use  │
│  crates/api    │                  │  crates/tools       │
└───────┬────────┘                  └─────────────────────┘
        │ provider adapters
        │ (anthropic, openai, google,
        │  azure, bedrock, cohere, copilot,
        │  codex, minimax, openai-compat)
        ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       crates/core                                    │
│  Types, Config, Settings, Auth, Session storage (JSONL + SQLite),   │
│  Permissions, Attachments, Feature flags, Context building           │
└───────────────────────────────┬─────────────────────────────────────┘
                                │ shared by all crates
            ┌───────────────────┼──────────────────┐
            ▼                   ▼                  ▼
    ┌──────────────┐  ┌──────────────────┐  ┌─────────────────┐
    │  mcp         │  │  plugins         │  │  bridge         │
    │  MCP client  │  │  Plugin runtime  │  │  Remote bridge  │
    │  (rmcp SDK)  │  │  hooks, manifest │  │  to claude.ai   │
    │  crates/mcp  │  │  crates/plugins  │  │  crates/bridge  │
    └──────────────┘  └──────────────────┘  └─────────────────┘

    ┌──────────────┐  ┌──────────────────┐
    │  buddy       │  │  commands        │
    │  Companion   │  │  Slash commands  │
    │  system      │  │  (/help, /model, │
    │  crates/     │  │   /compact, …)   │
    │  buddy       │  │  crates/commands │
    └──────────────┘  └──────────────────┘
```

## Component Responsibilities

| Component | Responsibility | Path |
|-----------|----------------|------|
| `claurst` (cli) | Binary entry point, CLI arg parsing, mode dispatch | `crates/cli/src/main.rs` |
| `claurst-core` | Shared types, config, permissions, session storage, auth | `crates/core/src/` |
| `claurst-api` | LLM provider abstraction, HTTP streaming, model registry | `crates/api/src/` |
| `claurst-query` | Agentic query loop, tool dispatch, compaction, agent orchestration | `crates/query/src/` |
| `claurst-tools` | All tool implementations (bash, file, web, MCP wrappers, etc.) | `crates/tools/src/` |
| `claurst-tui` | Ratatui TUI: app state, event loop, rendering, dialogs | `crates/tui/src/` |
| `claurst-commands` | Slash command framework and all `/cmd` implementations | `crates/commands/src/` |
| `claurst-mcp` | MCP protocol client: tool discovery, execution, OAuth, connection mgr | `crates/mcp/src/` |
| `claurst-bridge` | Remote bridge connecting local CLI to claude.ai web UI | `crates/bridge/src/` |
| `claurst-plugins` | Plugin runtime: discovery, manifest parsing, hook registry, marketplace | `crates/plugins/src/` |
| `claurst-acp` | Agent Client Protocol server: JSON-RPC 2.0 over stdio for editors | `crates/acp/src/lib.rs` |
| `claurst-buddy` | Companion/Tamagotchi system with deterministic species derivation | `crates/buddy/src/lib.rs` |

## Pattern Overview

**Overall:** Layered Cargo workspace — shared core, specialized protocol crates, thin binary.

**Key Characteristics:**
- `claurst-core` is the only crate with no workspace-crate dependencies; all others depend on it.
- `claurst-query` is the orchestration heart: it drives the agentic tool-use loop and coordinates API, tools, MCP, and compaction.
- `claurst-tui` owns the interactive user-facing event loop and dispatches to `claurst-query` for each conversation turn.
- The binary (`crates/cli`) wires everything together, but contains minimal logic itself.
- Async-first: Tokio runtime (`#[tokio::main]`) runs throughout; blocking operations are offloaded.

## Layers

**Core / Foundation (`claurst-core`):**
- Purpose: Shared primitives — zero workspace-crate dependencies
- Location: `crates/core/src/`
- Contains: Types, Config/Settings, AuthStore, PermissionManager, session storage (JSONL + SQLite), context building, feature flags, snapshot/undo, skill discovery
- Depends on: External crates only (tokio, serde, rusqlite, reqwest, etc.)
- Used by: Every other crate in the workspace

**API / Provider Layer (`claurst-api`):**
- Purpose: Unified LLM provider abstraction with streaming
- Location: `crates/api/src/`
- Contains: `LlmProvider` trait (`provider.rs`), concrete adapters in `providers/` (anthropic, openai, google, azure, bedrock, cohere, copilot, codex, minimax, openai-compat), model registry, stream parser, message transformers
- Depends on: `claurst-core`
- Used by: `claurst-query`, `claurst-tui`, `claurst-bridge`, `claurst-acp`, `claurst-cli`

**Tool Layer (`claurst-tools`):**
- Purpose: All agent-callable tool implementations
- Location: `crates/tools/src/`
- Contains: `Tool` trait + 30+ implementations: `BashTool`, `PtyBashTool`, `FileReadTool`, `FileWriteTool`, `FileEditTool`, `GlobTool`, `GrepTool`, `WebFetchTool`, `WebSearchTool`, `NotebookEditTool`, `ComputerUseTool`, `SkillTool`, `AgentTool`, etc.
- Depends on: `claurst-core`, `claurst-api`, `claurst-mcp`
- Used by: `claurst-query`

**Protocol Crates:**
- `claurst-mcp` — MCP client (JSON-RPC, tool discovery/execution, HTTP+stdio transports, OAuth). Depends only on `claurst-core`. Location: `crates/mcp/src/`
- `claurst-bridge` — Remote bridge to claude.ai web (long-polling, device fingerprinting, JWT decode). Location: `crates/bridge/src/`
- `claurst-acp` — JSON-RPC 2.0 stdio server for editor integrations (Zed, VS Code). Location: `crates/acp/src/lib.rs`

**Query / Orchestration Layer (`claurst-query`):**
- Purpose: The agentic loop — sends messages, dispatches tools, manages context, handles auto-compact
- Location: `crates/query/src/`
- Contains: Main query loop (`lib.rs`), coordinator/worker agent mode (`coordinator.rs`), auto-compaction (`compact.rs`), session memory extraction, cron scheduler, managed orchestrator, skill prefetch
- Depends on: `claurst-core`, `claurst-api`, `claurst-tools`, `claurst-plugins`
- Used by: `claurst-tui` (interactive), `claurst-cli` (headless)

**UI Layer (`claurst-tui`):**
- Purpose: Full interactive TUI — app state, event loop, all screens and dialogs
- Location: `crates/tui/src/`
- Contains: `App` struct (`app.rs`), render logic (`render.rs`), 40+ view/dialog modules, markdown rendering (`messages/`), voice capture, diff viewer, session browser
- Depends on: `claurst-core`, `claurst-api`, `claurst-tools`, `claurst-query`, `claurst-mcp`
- Used by: `claurst-cli` (in interactive mode)

**Command Layer (`claurst-commands`):**
- Purpose: Slash command framework and all `/cmd` implementations
- Location: `crates/commands/src/`
- Contains: `SlashCommand` trait, `CommandContext`, `CommandResult`, named command dispatch
- Depends on: All other crates except `claurst-buddy` and `claurst-acp`
- Used by: `claurst-cli`, `claurst-tui`

**Auxiliary Crates:**
- `claurst-plugins` — Plugin runtime with capability enforcement, marketplace, hooks (`crates/plugins/src/`)
- `claurst-buddy` — Companion system with deterministic species derivation from user-id via Mulberry32 PRNG (`crates/buddy/src/lib.rs`)

## Data Flow

### Interactive Mode (TUI)

1. `#[tokio::main]` in `crates/cli/src/main.rs` — parses args with `clap`, loads `Settings::load_hierarchical()`
2. Auth resolution (`config.resolve_anthropic_auth_async()`) and provider selection
3. MCP servers initialized via `claurst-mcp::McpManager`
4. Tool list assembled — native tools + MCP wrappers (`McpToolWrapper` in `main.rs`)
5. `claurst_tui::run_interactive()` enters the `ratatui` event loop (`crates/tui/src/app.rs`)
6. User input → `crossterm` events → `App::handle_event()` → `CommandQueue` or direct `query_loop_tx`
7. `claurst_query::run_query_loop()` drives model turns: sends `CreateMessageRequest` → streams `AnthropicStreamEvent`
8. Tool-use blocks detected → `ToolDispatcher::dispatch()` → concrete `Tool::execute()` → result appended
9. `QueryEvent` channel streams events back to `App` → `render::render()` redraws terminal

### Headless / Print Mode (`-p` flag)

1. Same startup as interactive up through step 4
2. `claurst_query::run_query_loop_headless()` — no TUI, outputs directly to stdout
3. Supports `--output-format text|json|stream-json`
4. Exits after `QueryOutcome::EndTurn` or error

### ACP Server Mode (`claude acp`)

1. `main.rs` fast-path intercepts `acp` subcommand before `Cli::parse()`
2. `claurst_acp::run_acp_server()` reads newline-delimited JSON-RPC from stdin
3. Responds to `initialize`, `session/create`, `session/message`, `tool/list`, `model/list`
4. Designed for editor integrations (Zed, VS Code) without launching a TUI

### Agent / Coordinator Mode

1. `CLAURST_COORDINATOR_MODE` env var activates coordinator mode (`crates/query/src/coordinator.rs`)
2. Coordinator uses `AgentTool` to spawn parallel worker sub-agents
3. Workers receive fully self-contained prompts; coordinator uses `SendMessageTool` for ongoing communication
4. `TasksOverlay` in TUI displays live worker status

**State Management:**
- All shared mutable state uses `Arc<parking_lot::Mutex<T>>` or `Arc<dashmap::DashMap<K,V>>`
- `CommandQueue` (`Arc`-backed) bridges TUI input thread and query loop
- Session history stored in SQLite (`SqliteSessionStore`) or JSONL fallback
- No global mutable state at module level in the hot path (config passed by value/clone)

## Key Abstractions

**`LlmProvider` trait:**
- Purpose: Uniform interface for all LLM backends
- Location: `crates/api/src/provider.rs`
- Pattern: `async_trait` with `create_message()`, `create_message_stream()`, `list_models()`, `check_health()`

**`Tool` trait:**
- Purpose: Uniform interface for all agent-callable tools
- Location: `crates/tools/src/lib.rs`
- Pattern: `async_trait` with `name()`, `description()`, `permission_level()`, `input_schema()`, `execute()`

**`SlashCommand` trait:**
- Purpose: Uniform interface for all `/cmd` implementations
- Location: `crates/commands/src/lib.rs`
- Pattern: Synchronous `execute()` returning `CommandResult` enum

**`McpToolWrapper` struct:**
- Purpose: Makes remote MCP server tools appear as native `Tool` implementors
- Location: `crates/cli/src/main.rs` (inline)

**`QueryConfig` struct:**
- Purpose: Complete configuration snapshot for one query-loop invocation
- Location: `crates/query/src/lib.rs`
- Pattern: Builder-like, constructed from `Config` then overridden with CLI args

**`PermissionManager`:**
- Purpose: Intercepts all tool execution, enforces `PermissionMode` rules, prompts user
- Location: `crates/core/src/` (exported from `claurst_core`)
- Modes: `Default`, `AcceptEdits`, `BypassPermissions`, `Plan`

## Entry Points

**Interactive binary:**
- Location: `crates/cli/src/main.rs` (`fn main()`)
- Triggers: `claurst` CLI invocation without `-p` / no prompt arg
- Responsibilities: Arg parse, settings load, auth, MCP init, tool assembly, TUI launch

**Headless binary:**
- Location: `crates/cli/src/main.rs` (same `main()`, different branch)
- Triggers: `claurst -p "prompt"` or stdin pipe
- Responsibilities: Same init, but calls headless query loop, prints result to stdout

**ACP server:**
- Location: `crates/acp/src/lib.rs` (`run_acp_server()`)
- Triggers: `claurst acp` subcommand
- Responsibilities: JSON-RPC 2.0 stdio server for editor integrations

**Build script:**
- Location: `crates/cli/build.rs`
- Triggers: Cargo build
- Responsibilities: Embeds `BUILD_TIME`, `GIT_COMMIT`, `PACKAGE_URL`, `FEEDBACK_CHANNEL`, `ISSUES_EXPLAINER` as `env!()` constants

## Architectural Constraints

- **Threading:** Tokio multi-thread runtime; all tool executions are `async`. `PtyBashTool` uses `portable-pty` on a blocking thread. UI render happens on the main async task.
- **Global state:** No mutable global singletons in the query hot path. `once_cell::sync::Lazy` used for read-only globals (constants, regex patterns). `parking_lot::Mutex`/`RwLock` for shared write state.
- **Circular imports:** No circular crate dependencies. Dependency order is strictly: `core` → `api/mcp/plugins` → `tools` → `query` → `tui/commands` → `cli`.
- **Feature flags:** `claurst-core` defines 36+ Cargo feature flags for experimental capabilities. `claurst-tui` passes them through with `claurst-core/<feature>` delegation. `dev_full` activates all features.
- **Computer-use:** `claurst-tools` has an optional `computer-use` feature gating `enigo`, `xcap`, and `image` dependencies.
- **Voice:** Optional `voice` feature (cpal) in `claurst-core` and `claurst-tui`; enabled by default in the CLI binary.

## Anti-Patterns

### Bypassing the permission system

**What happens:** Calling `Tool::execute()` directly without going through `PermissionManager::check_permission()`
**Why it's wrong:** Skips user confirmation for destructive operations; breaks `--permission-mode plan`
**Do this instead:** All tool execution must flow through `ToolContext::check_permission()` as shown in `McpToolWrapper::execute()` in `crates/cli/src/main.rs`

### Adding logic to the binary crate

**What happens:** Placing business logic in `crates/cli/src/main.rs` rather than in a library crate
**Why it's wrong:** The binary cannot be unit-tested; the `McpToolWrapper` struct already lives there as a necessary exception
**Do this instead:** Add new functionality to the appropriate library crate (`claurst-core`, `claurst-tools`, etc.) and call it from `main.rs`

### Blocking in async context

**What happens:** Calling `std::fs::read`, `std::process::Command`, or other blocking syscalls directly in an `async fn`
**Why it's wrong:** Starves the Tokio runtime, causing TUI stutters and query-loop hangs
**Do this instead:** Use `tokio::fs`, `tokio::process::Command`, or `tokio::task::spawn_blocking` for any blocking work

## Error Handling

**Strategy:** `anyhow::Result` for application-level errors (propagated with `?`); `thiserror`-derived typed errors for library boundaries (`ClaudeError`, `ProviderError`, `PluginError`).

**Patterns:**
- `QueryOutcome::Error(ClaudeError)` — query loop surfaces typed errors to the caller
- `ToolResult::error(String)` — tool failures are returned as structured error content, not panics
- Provider-level retries with exponential backoff are handled inside `claurst-api` (rate limits, overloaded responses)

## Cross-Cutting Concerns

**Logging:** `tracing` crate with `tracing-subscriber` + `EnvFilter`. Default level `warn`; `--verbose` sets `debug`. JSON log output available. Initialized once in `main.rs`.

**Validation:** Input validation happens at tool boundary via JSON Schema (`schemars`) and `input_schema()` on each `Tool`.

**Authentication:** `AuthStore` in `claurst-core` persists API keys and OAuth tokens. Per-provider auth resolved via `config.resolve_anthropic_auth_async()` and `ProviderRegistry`. MCP OAuth handled separately in `claurst-mcp::oauth`.

---

*Architecture analysis: 2026-05-05*
