# Project Structure

**Analysis Date:** 2026-05-04

## Directory Layout

```
src-rust/                          # Cargo workspace root
├── Cargo.toml                     # Workspace manifest, shared deps
├── crates/
│   ├── core/                      # Foundation library — shared types, config, auth
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs             # Module declarations + inline error/types/config/history/permissions modules
│   │   │   ├── session_storage.rs # JSONL transcript persistence
│   │   │   ├── sqlite_storage.rs  # SQLite-backed session storage
│   │   │   ├── auth_store.rs      # API key + OAuth token credential store
│   │   │   ├── device_code.rs     # RFC 8628 device-code OAuth flow
│   │   │   ├── attachments.rs     # Per-turn context attachment pipeline
│   │   │   ├── claudemd.rs        # AGENTS.md hierarchical memory loading
│   │   │   ├── settings_sync.rs   # Remote settings sync
│   │   │   ├── snapshot.rs        # Per-session file snapshot / undo system
│   │   │   ├── feature_flags.rs   # GrowthBook feature flag manager
│   │   │   ├── skill_discovery.rs # Filesystem + git URL skill loading
│   │   │   ├── keybindings.rs     # User keybinding resolution
│   │   │   ├── voice.rs           # Voice capture (optional, behind `voice` feature)
│   │   │   └── ...                # 40+ additional modules
│   │   └── tests/                 # Integration tests for core types
│   │
│   ├── api/                       # LLM provider abstraction + Anthropic API client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # Public re-exports, module declarations
│   │       ├── provider.rs        # `LlmProvider` trait definition
│   │       ├── provider_types.rs  # Provider-agnostic request/response types
│   │       ├── provider_error.rs  # `ProviderError` type
│   │       ├── auth.rs            # `AuthProvider` + `LoginFlow` traits
│   │       ├── registry.rs        # `ProviderRegistry` — routes requests to providers
│   │       ├── stream_parser.rs   # SSE + JSON-lines stream parsers
│   │       ├── transform.rs       # `MessageTransformer` trait
│   │       ├── model_registry.rs  # Dynamic model metadata (models.dev)
│   │       ├── error_handling.rs  # Provider-aware error classification
│   │       ├── cch.rs             # Claude.ai cache-and-credit helper
│   │       ├── codex_adapter.rs   # OpenAI Codex adapter
│   │       ├── providers/         # Concrete provider adapters
│   │       │   ├── mod.rs
│   │       │   ├── anthropic.rs   # Anthropic Messages API
│   │       │   ├── openai.rs      # OpenAI Chat Completions
│   │       │   ├── google.rs      # Google Gemini
│   │       │   ├── bedrock.rs     # AWS Bedrock
│   │       │   ├── azure.rs       # Azure OpenAI
│   │       │   ├── copilot.rs     # GitHub Copilot
│   │       │   ├── codex.rs       # OpenAI Codex
│   │       │   ├── cohere.rs      # Cohere
│   │       │   ├── minimax.rs     # MiniMax
│   │       │   ├── openai_compat.rs          # Generic OpenAI-compatible base
│   │       │   ├── openai_compat_providers.rs # 30+ provider factories (groq, ollama, etc.)
│   │       │   ├── message_normalization.rs   # Cross-provider message normalization
│   │       │   └── request_options.rs         # Provider-specific request options
│   │       └── transformers/      # Concrete message transformer implementations
│   │
│   ├── tools/                     # All LLM-callable tool implementations
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # `Tool` trait, `ToolResult`, `ToolContext`, `PermissionLevel`
│   │       ├── bash.rs            # BashTool (legacy subprocess)
│   │       ├── pty_bash.rs        # PtyBashTool (PTY-based, persistent shell state)
│   │       ├── file_read.rs       # FileReadTool
│   │       ├── file_edit.rs       # FileEditTool (search-and-replace edits)
│   │       ├── file_write.rs      # FileWriteTool
│   │       ├── apply_patch.rs     # ApplyPatchTool (unified diff)
│   │       ├── batch_edit.rs      # BatchEditTool
│   │       ├── glob_tool.rs       # GlobTool
│   │       ├── grep_tool.rs       # GrepTool
│   │       ├── web_fetch.rs       # WebFetchTool
│   │       ├── web_search.rs      # WebSearchTool
│   │       ├── agent_tool.rs      # AgentTool (spawns sub-agents)
│   │       ├── tasks.rs           # Task lifecycle tools (TaskCreate/Get/Update/Stop/Output)
│   │       ├── todo_write.rs      # TodoWriteTool
│   │       ├── send_message.rs    # SendMessageTool (inter-agent messaging)
│   │       ├── team_tool.rs       # TeamCreate/Delete (agent swarms)
│   │       ├── computer_use.rs    # ComputerUseTool (screen capture, input — feature-gated)
│   │       ├── skill_tool.rs      # SkillTool (user-defined slash commands)
│   │       ├── notebook_edit.rs   # NotebookEditTool (Jupyter)
│   │       ├── mcp_resources.rs   # ListMcpResourcesTool, ReadMcpResourceTool
│   │       ├── lsp_tool.rs        # LspTool (LSP diagnostics)
│   │       ├── remote_trigger.rs  # RemoteTriggerTool
│   │       ├── monitor_tool.rs    # MonitorTool
│   │       ├── worktree.rs        # EnterWorktree/ExitWorktreeTool
│   │       ├── cron.rs            # CronCreate/Delete/List
│   │       ├── powershell.rs      # PowerShellTool (Windows)
│   │       ├── formatter.rs       # try_format_file helper
│   │       └── ...                # ask_user, brief, config_tool, enter/exit_plan_mode, repl_tool, sleep, synthetic_output
│   │
│   ├── query/                     # Agentic conversation loop
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # `run_query_loop()`, `QueryConfig`, `QueryEvent`, `QueryOutcome`
│   │       ├── compact.rs         # Auto-compact, micro-compact, context-collapse logic
│   │       ├── agent_tool.rs      # `AgentTool` integration + swarm runner init
│   │       ├── coordinator.rs     # Coordinator / worker agent modes
│   │       ├── managed_orchestrator.rs # Managed agent orchestration
│   │       ├── command_queue.rs   # `CommandQueue` — TUI→loop command injection
│   │       ├── context_analyzer.rs     # Context window analysis
│   │       ├── session_memory.rs  # Memory extraction from sessions
│   │       ├── skill_prefetch.rs  # Background skill index loading
│   │       ├── cron_scheduler.rs  # Cron job scheduler
│   │       ├── auto_dream.rs      # Auto-dream (background reflection) feature
│   │       └── away_summary.rs    # Away-summary generation
│   │
│   ├── tui/                       # Terminal UI (ratatui + crossterm)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs             # Terminal setup/teardown, module declarations
│   │   │   ├── app.rs             # `App` struct, main event loop, state machine
│   │   │   ├── render.rs          # All ratatui layout and widget rendering
│   │   │   ├── prompt_input.rs    # Input field (vim mode, history, typeahead, paste)
│   │   │   ├── messages/          # Per-message-type renderers
│   │   │   ├── transcript_turn.rs # Turn grouping and metadata
│   │   │   ├── virtual_list.rs    # Virtualized scrollable message list
│   │   │   ├── overlays.rs        # Help, history-search, message selector, rewind
│   │   │   ├── dialogs.rs         # Permission and confirmation dialogs
│   │   │   ├── settings_screen.rs # Full-screen tabbed settings
│   │   │   ├── model_picker.rs    # Model/effort picker overlay
│   │   │   ├── session_browser.rs # Session history browser
│   │   │   ├── mcp_view.rs        # MCP server management UI
│   │   │   ├── agents_view.rs     # Agent definitions list + coordinator progress
│   │   │   ├── diff_viewer.rs     # Two-pane diff viewer dialog
│   │   │   ├── stats_dialog.rs    # Token usage and cost charts
│   │   │   ├── notifications.rs   # Notification / banner system
│   │   │   ├── bridge_state.rs    # Bridge connection status badge
│   │   │   ├── plugin_views.rs    # Plugin hint/recommendation UI
│   │   │   ├── theme_colors.rs    # Color palette for themes
│   │   │   ├── theme_screen.rs    # Theme picker overlay
│   │   │   ├── voice_capture.rs   # Voice PTT UI (behind `voice` feature)
│   │   │   ├── kitty_image.rs     # Kitty graphics protocol inline images
│   │   │   ├── image_paste.rs     # Clipboard image paste + Ctrl+V
│   │   │   ├── figures.rs         # Icon/figure constants (matches figures.ts)
│   │   │   └── ...                # 20+ additional dialog/screen modules
│   │   └── tests/                 # TUI unit tests
│   │
│   ├── commands/                  # Slash command system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # `SlashCommand` trait, `CommandContext`, `CommandResult`
│   │       └── named_commands.rs  # All concrete slash command implementations
│   │
│   ├── mcp/                       # Model Context Protocol client
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # `McpManager`, env-var expansion, public re-exports
│   │       ├── backend.rs         # Raw MCP transport (stdio subprocess)
│   │       ├── rmcp_backend.rs    # `rmcp` crate-backed transport (streamable HTTP)
│   │       ├── connection_manager.rs # `McpConnectionManager`, reconnection, status
│   │       ├── registry.rs        # Tool/resource/prompt discovery registry
│   │       └── oauth.rs           # MCP OAuth flows
│   │
│   ├── plugins/                   # Plugin runtime
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs             # `GLOBAL_HOOK_REGISTRY`, `GLOBAL_PLUGIN_REGISTRY`, capability enforcement
│   │       ├── manifest.rs        # `PluginManifest` struct, hook/MCP/LSP config types
│   │       ├── plugin.rs          # `LoadedPlugin`, `PluginCommandDef`, `PluginError`
│   │       ├── loader.rs          # `discover_plugins()`, directory search logic
│   │       ├── registry.rs        # `PluginRegistry` — lookup and dispatch
│   │       ├── hooks.rs           # `HookRegistry`, `RegisteredHook`, hook firing
│   │       └── marketplace.rs     # Plugin marketplace: search, download, verify (sha2)
│   │
│   ├── bridge/                    # claude.ai web remote-control bridge
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs             # JWT decode, device fingerprint, session lifecycle, long-poll loop
│   │
│   ├── acp/                       # Agent Client Protocol server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs             # JSON-RPC 2.0 over stdio: initialize, session/*, tool/list, model/list
│   │
│   ├── buddy/                     # Companion/Tamagotchi system
│   │   ├── Cargo.toml
│   │   └── src/
│   │       └── lib.rs             # Mulberry32 PRNG, deterministic companion traits, persistence
│   │
│   └── cli/                       # Binary crate (the `claurst` executable)
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs            # Entry point: arg parsing, mode routing, McpToolWrapper
│           ├── oauth_flow.rs      # Claude.ai / Console OAuth web flow
│           ├── codex_oauth_flow.rs# OpenAI Codex OAuth flow
│           └── system_prompt.txt  # Embedded default system prompt
│
├── .planning/
│   └── codebase/                  # GSD codebase map documents (this directory)
│
└── target/                        # Cargo build output (not committed)
```

## Crate Inventory

| Crate name | Package name | Type | Purpose |
|------------|-------------|------|---------|
| `crates/cli` | `claurst` | **bin** | CLI entry point; `claurst` binary |
| `crates/core` | `claurst-core` | lib | Foundation: types, config, auth, permissions, sessions, feature flags |
| `crates/api` | `claurst-api` | lib | Multi-provider LLM abstraction + Anthropic client; streaming SSE |
| `crates/tools` | `claurst-tools` | lib | All `Tool` implementations (~35 tools); `Tool` trait definition |
| `crates/query` | `claurst-query` | lib | Agentic conversation loop; auto-compact; multi-agent coordinator |
| `crates/tui` | `claurst-tui` | lib | ratatui terminal UI; event loop; all dialogs and overlays |
| `crates/commands` | `claurst-commands` | lib | Slash command framework (`/model`, `/compact`, `/help`, etc.) |
| `crates/mcp` | `claurst-mcp` | lib | MCP JSON-RPC 2.0 client; tool/resource/prompt discovery |
| `crates/plugins` | `claurst-plugins` | lib | Plugin loader; hook registry; marketplace; capability enforcement |
| `crates/bridge` | `claurst-bridge` | lib | claude.ai web remote-control bridge (long-poll) |
| `crates/acp` | `claurst-acp` | lib | ACP server — JSON-RPC 2.0 over stdio for editor integration |
| `crates/buddy` | `claurst-buddy` | lib | Companion/Tamagotchi system |

## Module Organization per Crate

### `claurst-core` (`crates/core/src/lib.rs`)

All sub-modules are declared in `lib.rs`. Key inline module blocks (not separate files):
- `error` — `ClaudeError` enum, `Result<T>` alias
- `types` — `ContentBlock`, `Message`, `Role`, `ToolDefinition`, `UsageInfo`
- `config` — `Config`, `Settings`, `McpServerConfig`, `PermissionMode`, `AgentDefinition`
- `history` — `ConversationSession`
- `cost` — `CostTracker`
- `permissions` — `PermissionHandler` trait, `PermissionManager`, all implementations
- `constants` — `APP_VERSION`, `ANTHROPIC_API_VERSION`, tool name constants

File-per-module for: `session_storage`, `sqlite_storage`, `auth_store`, `device_code`, `attachments`, `claudemd`, `feature_flags`, `snapshot`, `skill_discovery`, `keybindings`, `voice`, and ~25 more.

### `claurst-api` (`crates/api/src/`)

Organized in phases matching the provider abstraction rollout:
- Phase 1A: `provider_types.rs` — unified request/response types
- Phase 1B: `provider.rs`, `auth.rs`, `stream_parser.rs`, `transform.rs` — traits
- Phase 1C: `registry.rs` — `ProviderRegistry`
- Phase 1D: `providers/` — concrete adapters (Anthropic, OpenAI, Google, Bedrock, Azure, Copilot, Codex, Cohere, MiniMax, 30+ OpenAI-compat)
- Phase 3: `model_registry.rs` — dynamic model metadata
- Phase 4: `transformers/` — concrete transformers
- Phase 6: `error_handling.rs` — provider-aware error classification

The original Anthropic-only client lives in inline `client` and `streaming` modules inside `lib.rs` (still re-exported as `AnthropicClient`, `AnthropicStreamEvent`, `StreamHandler`).

### `claurst-tools` (`crates/tools/src/`)

One file per tool. Common tool groupings:
- **File I/O:** `file_read.rs`, `file_edit.rs`, `file_write.rs`, `apply_patch.rs`, `batch_edit.rs`
- **Search:** `glob_tool.rs`, `grep_tool.rs`
- **Shell:** `bash.rs`, `pty_bash.rs`, `powershell.rs`
- **Web:** `web_fetch.rs`, `web_search.rs`
- **Agent/task:** `agent_tool.rs`, `tasks.rs`, `send_message.rs`, `team_tool.rs`, `remote_trigger.rs`
- **UI/interaction:** `ask_user.rs`, `enter_plan_mode.rs`, `exit_plan_mode.rs`
- **System:** `computer_use.rs` (feature-gated), `cron.rs`, `sleep.rs`, `worktree.rs`
- **MCP:** `mcp_resources.rs`, `mcp_auth_tool.rs`
- **Dev:** `lsp_tool.rs`, `notebook_edit.rs`, `repl_tool.rs`, `todo_write.rs`

### `claurst-query` (`crates/query/src/`)

- `lib.rs` — `run_query_loop()` (the main agentic loop, ~2000 lines), `QueryConfig`, `QueryEvent`, `QueryOutcome`
- `compact.rs` — all auto-compact, micro-compact, context-collapse strategies
- `coordinator.rs` — coordinator vs. worker mode detection and prompts
- `managed_orchestrator.rs` — managed agent orchestration (preset agents)
- `command_queue.rs` — `CommandQueue` for TUI→loop injection
- `skill_prefetch.rs` — background skill index building

### `claurst-tui` (`crates/tui/src/`)

- `app.rs` — central `App` struct and main crossterm event loop
- `render.rs` — all ratatui widget rendering (the largest rendering module)
- `messages/` — subdirectory with per-block-type renderers
- `overlays.rs` — help, history-search, message selector, rewind flow
- `dialogs.rs` — permission request and confirmation dialogs
- `prompt_input.rs` — full vim-mode input field with history and typeahead
- Other files are focused dialogs/screens (settings, model picker, MCP view, agents view, etc.)

## Naming Conventions

**Files:**
- `snake_case.rs` for all Rust source files
- One tool per file in `crates/tools/src/`
- One dialog/screen per file in `crates/tui/src/`

**Directories:**
- `snake_case/` for all directories

**Crates:**
- Workspace crate names: `claurst-<component>` (hyphen-separated)
- Package path aliases in `Cargo.toml`: `claurst-core`, `claurst-api`, etc.

## Where to Add New Code

**New LLM provider:**
- Implement `LlmProvider` trait from `crates/api/src/provider.rs`
- Add adapter file to `crates/api/src/providers/<name>.rs`
- Register in `crates/api/src/providers/mod.rs`
- For OpenAI-compatible: add a factory function in `crates/api/src/providers/openai_compat_providers.rs`

**New tool:**
- Add `<tool_name>.rs` to `crates/tools/src/`
- Implement `Tool` trait (name, description, permission_level, input_schema, execute)
- Declare module and `pub use` in `crates/tools/src/lib.rs`
- Add to `all_tools()` vector in `crates/tools/src/lib.rs:483`

**New slash command:**
- Add implementation to `crates/commands/src/named_commands.rs`
- Return appropriate `CommandResult` variant
- Register in `crates/commands/src/lib.rs`

**New TUI dialog or overlay:**
- Add `<name>_dialog.rs` or `<name>_screen.rs` to `crates/tui/src/`
- Declare module in `crates/tui/src/lib.rs`
- Add state struct to `App` in `crates/tui/src/app.rs`
- Add rendering branch in `crates/tui/src/render.rs`

**New core type or utility:**
- Add module file to `crates/core/src/`
- Declare with `pub mod` in `crates/core/src/lib.rs`
- Re-export at crate root with `pub use` if widely needed

**New feature flag:**
- Add to `[features]` in `crates/core/Cargo.toml`
- Add pass-through feature in `crates/tui/Cargo.toml` pointing to `claurst-core/<feature>`
- Gate code with `#[cfg(feature = "...")]`

## Special Directories

**`.planning/codebase/`:**
- Purpose: GSD codebase map documents
- Generated: By `/gsd-map-codebase` command
- Committed: Yes

**`target/`:**
- Purpose: Cargo build artifacts
- Generated: Yes
- Committed: No (in `.gitignore`)

---

*Structure analysis: 2026-05-04*
