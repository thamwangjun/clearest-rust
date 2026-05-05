<!-- refreshed: 2026-05-04 -->
# Architecture

**Analysis Date:** 2026-05-04

## System Overview

```text
┌──────────────────────────────────────────────────────────────────────┐
│                        claurst (bin)                                  │
│                    crates/cli/src/main.rs                             │
│   Parses CLI args (clap) · loads Config · sets up tracing            │
│   Chooses: TUI REPL mode  ─OR─  headless print mode  ─OR─  ACP mode │
└──────────┬────────────────────────────┬─────────────────┬────────────┘
           │                            │                  │
           ▼                            ▼                  ▼
┌──────────────────┐  ┌────────────────────────┐  ┌──────────────────┐
│  claurst-tui     │  │  claurst-query          │  │  claurst-acp     │
│  crates/tui/     │  │  crates/query/          │  │  crates/acp/     │
│  ratatui UI      │  │  Agentic query loop     │  │  JSON-RPC 2.0    │
│  Event loop      │◄─┤  Streaming, tool-use    │  │  over stdio      │
│  Slash commands  │  │  Auto-compact           │  │  (editor integr) │
└──────────────────┘  └──────────┬──────────────┘  └──────────────────┘
                                  │
                    ┌─────────────┼─────────────┐
                    │             │              │
                    ▼             ▼              ▼
          ┌──────────────┐ ┌──────────┐ ┌────────────────┐
          │ claurst-api  │ │claurst-  │ │ claurst-mcp    │
          │ crates/api/  │ │tools     │ │ crates/mcp/    │
          │ LLM providers│ │crates/   │ │ MCP client     │
          │ Anthropic,   │ │tools/    │ │ stdio+HTTP/SSE │
          │ OpenAI, GCP, │ │File,Bash,│ │ JSON-RPC 2.0   │
          │ Bedrock, etc.│ │Web, Agent│ └────────────────┘
          └──────────────┘ └──────────┘
                    │
                    ▼
          ┌──────────────────────────────────────────────────┐
          │                  claurst-core                     │
          │                  crates/core/                     │
          │  Types · Config · Auth · Permissions · Sessions   │
          │  History · Snapshots · Feature flags · Skills     │
          └──────────────────────────────────────────────────┘
                    │
       ┌────────────┴──────────────┐
       │                           │
       ▼                           ▼
┌─────────────────┐    ┌──────────────────────────────┐
│ claurst-buddy   │    │  claurst-plugins              │
│ crates/buddy/   │    │  crates/plugins/              │
│ Companion system│    │  Plugin discovery, hooks,     │
│ (Tamagotchi)    │    │  marketplace, capability guard│
└─────────────────┘    └──────────────────────────────┘
       │
       ▼
┌──────────────────────────┐
│  claurst-bridge          │
│  crates/bridge/          │
│  claude.ai remote control│
│  Long-poll, JWT, events  │
└──────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | Key Files |
|-----------|----------------|-----------|
| `claurst-cli` | Binary entry point, CLI arg parsing, mode routing | `crates/cli/src/main.rs` |
| `claurst-core` | Shared types, config, auth, permissions, session storage | `crates/core/src/lib.rs` |
| `claurst-api` | LLM provider abstraction, streaming SSE, multi-provider adapters | `crates/api/src/lib.rs` |
| `claurst-query` | Agentic conversation loop, tool dispatch, auto-compact, coordinator | `crates/query/src/lib.rs` |
| `claurst-tools` | All tool implementations (Bash, file I/O, web, agent, etc.) | `crates/tools/src/lib.rs` |
| `claurst-tui` | ratatui terminal UI, event loop, dialogs, overlays | `crates/tui/src/app.rs` |
| `claurst-mcp` | MCP JSON-RPC client, tool/resource discovery, connection manager | `crates/mcp/src/lib.rs` |
| `claurst-commands` | Slash command framework (`/compact`, `/model`, `/cost`, etc.) | `crates/commands/src/lib.rs` |
| `claurst-plugins` | Plugin loader, hook registry, marketplace, capability enforcement | `crates/plugins/src/lib.rs` |
| `claurst-bridge` | claude.ai web remote-control bridge (long-poll, JWT, events) | `crates/bridge/src/lib.rs` |
| `claurst-acp` | Agent Client Protocol server (JSON-RPC 2.0 over stdio for editors) | `crates/acp/src/lib.rs` |
| `claurst-buddy` | Companion/Tamagotchi system, deterministic PRNG-based traits | `crates/buddy/src/lib.rs` |

## Architectural Pattern

**Overall:** Layered hexagonal architecture with a feature-flag-heavy core.

**Key Characteristics:**
- `claurst-core` is a dependency-free foundation depended on by every other crate.
- `claurst-api` implements a provider abstraction behind `LlmProvider` trait, enabling multi-LLM support without changing upper layers.
- `claurst-query` owns the agentic loop: send → stream → detect tool-use → dispatch → feed back. It emits `QueryEvent`s over `mpsc::UnboundedSender` so the TUI can render progress without polling.
- Tools are dispatched via the `Tool` trait (`crates/tools/src/lib.rs:456`). MCP tools are wrapped in `McpToolWrapper` at the CLI layer to appear as native tools.
- Feature gating is pervasive: `claurst-core` has 36+ Cargo features (e.g. `voice`, `ultraplan`, `bridge_mode`). TUI passes them through via feature re-exports.
- `parking_lot::Mutex`/`RwLock` are used throughout instead of `std::sync` for better performance. `DashMap` is used for lock-free concurrent maps.
- All async work runs on a single tokio runtime (via `#[tokio::main]` in `crates/cli/src/main.rs`).

## Crate Dependency Graph

```
claurst (bin)
  ├── claurst-core        (no workspace-crate deps)
  ├── claurst-api         → claurst-core
  ├── claurst-mcp         → claurst-core
  ├── claurst-tools       → claurst-core, claurst-api, claurst-mcp
  ├── claurst-plugins     → claurst-core
  ├── claurst-query       → claurst-core, claurst-api, claurst-tools, claurst-plugins
  ├── claurst-tui         → claurst-core, claurst-api, claurst-tools, claurst-query, claurst-mcp
  ├── claurst-commands    → claurst-core, claurst-api, claurst-tools, claurst-query,
  │                         claurst-mcp, claurst-tui, claurst-plugins, claurst-bridge
  ├── claurst-bridge      → claurst-core, claurst-api, claurst-query
  ├── claurst-acp         → claurst-core, claurst-api
  ├── claurst-buddy       → claurst-core
  └── claurst-plugins     → claurst-core
```

`claurst-core` is the only crate with **no** workspace-crate dependencies.
`claurst-commands` has the broadest fan-in (depends on 8 other crates).

## Data Flow

### Primary Interactive Request Path

1. User types input → `crates/tui/src/prompt_input.rs` captures keystroke via crossterm
2. `crates/tui/src/app.rs` dispatches to `run_query_loop()` in `crates/query/src/lib.rs:699`
3. Query loop builds `CreateMessageRequest` → `crates/api/src/lib.rs` selects provider via `ProviderRegistry`
4. Provider (e.g. `AnthropicProvider` at `crates/api/src/providers/anthropic.rs`) streams SSE events
5. Query loop accumulates stream, emits `QueryEvent::Stream` → TUI renders incrementally
6. On `tool_use` content block: `QueryEvent::ToolStart` → dispatches to `Tool::execute()` in `crates/tools/`
7. `QueryEvent::ToolEnd` with result → query loop feeds result back as next user message
8. On `end_turn` stop reason: `QueryEvent::TurnComplete` → TUI finalizes render

### Headless (--print) Path

1. `crates/cli/src/main.rs` calls `run_query_loop()` directly with `event_tx = None`
2. Output is accumulated and printed to stdout when `QueryOutcome::EndTurn` arrives

### ACP (Editor Integration) Path

1. `crates/acp/src/lib.rs` reads newline-delimited JSON-RPC 2.0 from stdin
2. Dispatches to session creation, tool listing, model listing
3. Returns responses as newline-delimited JSON to stdout

### MCP Tool Invocation Path

1. `McpToolWrapper` (in `crates/cli/src/main.rs:54`) wraps MCP tools as `Tool` implementations
2. `Tool::execute()` calls `McpManager::call_tool()` in `crates/mcp/`
3. MCP manager routes over stdio subprocess or HTTP/SSE transport

### Plugin Hook Path

1. Plugin hooks registered via `GLOBAL_HOOK_REGISTRY` at startup (`crates/plugins/src/lib.rs:76`)
2. Post-model-turn hooks fired in `fire_post_sampling_hooks()` (`crates/query/src/lib.rs:490`)
3. Hook stderr/stdout injected as user messages for the model to react to

### Auto-compact Path

1. Query loop checks token usage via `should_auto_compact()` (`crates/query/src/compact.rs`)
2. When context ≥ threshold, `compact_conversation()` is called — sends a compaction prompt
3. Compacted summary replaces prior history; loop continues

**State Management:**
- `Config` and `Settings` are immutable snapshots per-session; config changes trigger a full reload.
- `CostTracker` is `Arc`-wrapped and shared between query loop, TUI, and tools.
- Shell state (cwd, env vars) is tracked per session in `SHELL_STATE_REGISTRY` (`DashMap<session_id, Arc<Mutex<ShellState>>>`).
- Snapshot/undo state is in `SNAPSHOT_REGISTRY` (same pattern).
- MCP connection state is managed in `McpConnectionManager` (`crates/mcp/src/connection_manager.rs`).

## Key Abstractions

**`LlmProvider` trait** (`crates/api/src/provider.rs:48`):
- Implemented by: `AnthropicProvider`, `OpenAiProvider`, `GoogleProvider`, `BedrockProvider`, `AzureProvider`, `CopilotProvider`, `CodexProvider`, `CohereProvider`, `MinimaxProvider`, and 30+ `openai_compat` providers
- Methods: `create_message()`, `create_message_stream()`, `list_models()`, `check_health()`
- Registry: `ProviderRegistry` in `crates/api/src/registry.rs`

**`Tool` trait** (`crates/tools/src/lib.rs:456`):
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn permission_level(&self) -> PermissionLevel;
    fn input_schema(&self) -> Value;
    async fn execute(&self, input: Value, ctx: &ToolContext) -> ToolResult;
}
```
- ~35 built-in tool implementations in `crates/tools/src/`
- MCP tools wrapped via `McpToolWrapper` at runtime

**`ContentBlock` enum** (`crates/core/src/lib.rs:193`):
- Tagged union covering: `Text`, `Image`, `Document`, `ToolUse`, `ToolResult`, `Thinking`, `RedactedThinking`, plus UI-only variants (`UserLocalCommandOutput`, `UserCommand`, `UserMemoryInput`, `SystemAPIError`, `CollapsedReadSearch`, `TaskAssignment`)

**`Message` struct** (`crates/core/src/lib.rs:308`):
- `role: Role`, `content: MessageContent` (text or blocks), `uuid`, `cost`

**`QueryEvent` enum** (`crates/query/src/lib.rs:448`):
- Communication channel from query loop to TUI: `Stream`, `ToolStart`, `ToolEnd`, `TurnComplete`, `Status`, `Error`, `TokenWarning`

**`PermissionHandler` trait** (`crates/core/src/lib.rs:84`):
- Implementations: `AutoPermissionHandler`, `InteractivePermissionHandler`, `ManagedAutoPermissionHandler`, `ManagedInteractivePermissionHandler`

**`SlashCommand` trait** (`crates/commands/src/lib.rs`):
- Implemented by all `/` commands; produces `CommandResult` enum variants (e.g. `Message`, `ConfigChange`, `ClearConversation`, `Exit`, `OpenRewindOverlay`)

**`PluginManifest`** (`crates/plugins/src/manifest.rs`):
- Defines plugin hooks, MCP servers, LSP servers, slash commands
- Capability enforcement via `check_plugin_capability()` (`crates/plugins/src/lib.rs:44`)

## Entry Points

**Interactive TUI:**
- Location: `crates/cli/src/main.rs`
- Triggers: `claurst` (no `--print` flag)
- Responsibilities: Init tracing, load config, discover plugins/skills, connect MCP, launch `App::run()` in `crates/tui/src/app.rs`

**Headless print:**
- Location: `crates/cli/src/main.rs`
- Triggers: `claurst --print` / `claurst -p`
- Responsibilities: Run single query, output to stdout, exit

**ACP server:**
- Location: `crates/acp/src/lib.rs`
- Triggers: `claurst --acp` (or the `acp_server()` function)
- Responsibilities: JSON-RPC 2.0 over stdio for editor integration (Zed, VS Code)

## Architectural Constraints

- **Threading:** Single tokio runtime (multi-thread by default via `tokio = { features = ["full"] }`). No cross-thread blocking allowed on the async executor.
- **Global state:** `SHELL_STATE_REGISTRY`, `SNAPSHOT_REGISTRY` (both in `crates/tools/src/lib.rs`), `GLOBAL_HOOK_REGISTRY`, and `GLOBAL_PLUGIN_REGISTRY` (both in `crates/plugins/src/lib.rs`) are process-level singletons via `once_cell::sync::Lazy<DashMap<...>>` or `OnceLock`.
- **Circular imports:** None — crate dependency graph is a strict DAG with `claurst-core` at the root.
- **Feature propagation:** TUI feature flags are re-exported from `claurst-core` features. The binary must enable the same feature set on both crates to avoid mismatched conditional compilation.
- **MCP auth:** OAuth flows for MCP servers are dispatched back to the TUI via a callback arc (`mcp_auth_runner` in `CommandContext`), not handled inline.

## Anti-Patterns

### Blocking inside async context

**What happens:** Some tool implementations call `std::process::Command` (synchronous) inside an `async fn`. This appears in `fire_post_sampling_hooks()` (`crates/query/src/lib.rs:490`) and hook execution.
**Why it's wrong:** Blocks the tokio thread, reducing throughput under concurrent tool calls.
**Do this instead:** Use `tokio::process::Command` or `tokio::task::spawn_blocking` for subprocess calls. See `crates/tools/src/bash.rs` for the correct PTY-based approach.

### Config passed by value (clone-heavy)

**What happens:** `Config` is cloned and passed into `CommandContext`, `QueryConfig::from_config()`, and tool setups on every query.
**Why it's wrong:** `Config` is large (includes `HashMap` of MCP servers, hooks, agents). Repeated cloning adds allocation pressure.
**Do this instead:** Wrap `Config` in `Arc<Config>` and clone the `Arc`, not the data. The pattern already appears with `CostTracker` and `PermissionHandler`.

## Error Handling

**Strategy:** `anyhow::Result` at call-site boundaries; `ClaudeError` (`thiserror`) for library-facing error types.

**Patterns:**
- `ClaudeError` variants in `crates/core/src/lib.rs:100`: `Api`, `ApiStatus`, `Auth`, `PermissionDenied`, `Tool`, `Io`, `Json`, `Http`, `RateLimit`, `ContextWindowExceeded`, `MaxTokensReached`, `Cancelled`, `Config`, `Mcp`, `Other`
- `ClaudeError::is_retryable()` and `ClaudeError::is_context_limit()` used by the query loop to decide retry/compact behavior
- Tool errors are returned as `ToolResult { is_error: true, content: ... }` — they do NOT panic or propagate as `Err`
- Provider errors use `ProviderError` (`crates/api/src/provider_error.rs`) then converted at query-loop boundary

## Cross-Cutting Concerns

**Logging:** `tracing` crate throughout. `tracing-subscriber` with `env-filter` and optional JSON format configured in `crates/cli/src/main.rs`.
**Validation:** Input JSON validated against JSON Schema at tool dispatch; provider requests validated per-provider.
**Authentication:** `AuthStore` in `crates/core/src/auth_store.rs` stores API keys and OAuth tokens. OAuth device-code flow in `crates/core/src/device_code.rs`; OAuth web flow in `crates/cli/src/oauth_flow.rs` and `crates/cli/src/codex_oauth_flow.rs`.

---

*Architecture analysis: 2026-05-04*
