# Codebase Structure

**Analysis Date:** 2026-05-05

## Directory Layout

```
clearest-rust/                  # Cargo workspace root
├── Cargo.toml                  # Workspace manifest — members, shared deps, workspace.package
├── Cargo.lock                  # Dependency lockfile (committed)
├── crates/                     # All library and binary crates
│   ├── acp/                    # Agent Client Protocol — JSON-RPC 2.0 stdio server
│   │   └── src/lib.rs
│   ├── api/                    # LLM provider abstraction layer
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── provider.rs         # LlmProvider trait
│   │       ├── provider_types.rs   # ProviderRequest/Response/StreamEvent types
│   │       ├── provider_error.rs   # ProviderError typed errors
│   │       ├── auth.rs             # Provider auth helpers
│   │       ├── registry.rs         # ProviderRegistry
│   │       ├── model_registry.rs   # ModelRegistry (multi-provider model listing)
│   │       ├── stream_parser.rs    # SSE streaming parser
│   │       ├── transform.rs        # Message transform trait
│   │       ├── error_handling.rs   # Retry and error classification
│   │       ├── cch.rs              # Prompt cache helpers
│   │       ├── codex_adapter.rs    # Codex/OpenAI Codex adapter
│   │       ├── providers/          # Concrete provider adapters
│   │       │   ├── mod.rs
│   │       │   ├── anthropic.rs
│   │       │   ├── openai.rs
│   │       │   ├── openai_compat.rs
│   │       │   ├── openai_compat_providers.rs  # Groq, Ollama, etc. via OpenAI compat
│   │       │   ├── google.rs
│   │       │   ├── azure.rs
│   │       │   ├── bedrock.rs
│   │       │   ├── cohere.rs
│   │       │   ├── copilot.rs
│   │       │   ├── codex.rs
│   │       │   ├── minimax.rs
│   │       │   ├── message_normalization.rs
│   │       │   └── request_options.rs
│   │       └── transformers/       # Message format transformers
│   │           ├── mod.rs
│   │           ├── anthropic.rs
│   │           └── openai_chat.rs
│   ├── bridge/                 # Remote bridge to claude.ai web UI
│   │   └── src/lib.rs
│   ├── buddy/                  # Companion/Tamagotchi system
│   │   └── src/lib.rs
│   ├── cli/                    # Binary crate — the `claurst` executable
│   │   ├── build.rs            # Embeds BUILD_TIME, GIT_COMMIT, etc.
│   │   └── src/
│   │       ├── main.rs         # Entry point, arg parsing, mode dispatch
│   │       ├── oauth_flow.rs   # Anthropic OAuth device flow
│   │       └── codex_oauth_flow.rs  # Codex OAuth flow
│   ├── commands/               # Slash command framework
│   │   └── src/
│   │       ├── lib.rs          # SlashCommand trait, CommandContext, CommandResult
│   │       └── named_commands.rs   # Named CLI subcommands (agents, ide, branch, …)
│   ├── core/                   # Foundation crate — shared types and utilities
│   │   ├── src/
│   │   │   ├── lib.rs              # Module declarations and pub re-exports
│   │   │   ├── attachments.rs      # Per-turn context attachment pipeline
│   │   │   ├── auth_store.rs       # API key and OAuth token storage
│   │   │   ├── auto_mode.rs        # Auto permission mode logic
│   │   │   ├── bash_classifier.rs  # Bash command safety classification
│   │   │   ├── claudemd.rs         # AGENTS.md hierarchical memory loading
│   │   │   ├── cloud_session.rs    # Cloud session API
│   │   │   ├── codex_oauth.rs      # Codex OAuth helpers
│   │   │   ├── context_collapse.rs # Context collapse / compaction helpers
│   │   │   ├── crypto_utils.rs     # Hashing and crypto utilities
│   │   │   ├── device_code.rs      # OAuth Device Code Flow (RFC 8628)
│   │   │   ├── effort.rs           # EffortLevel for extended thinking
│   │   │   ├── feature_flags.rs    # GrowthBook feature flag integration
│   │   │   ├── feature_gates.rs    # Compile-time feature gate helpers
│   │   │   ├── file_history.rs     # Per-session file modification history
│   │   │   ├── format_utils.rs     # Output formatting utilities
│   │   │   ├── git_utils.rs        # Git status and diff utilities
│   │   │   ├── ide.rs              # IDE environment detection
│   │   │   ├── import_config.rs    # Config import (CLAUDE.md, settings.json)
│   │   │   ├── keybindings.rs      # Keybinding resolver
│   │   │   ├── lsp.rs              # LSP integration helpers
│   │   │   ├── mcp_templates.rs    # MCP resource prompt template rendering
│   │   │   ├── memdir.rs           # Memory directory management
│   │   │   ├── message_utils.rs    # Message manipulation utilities
│   │   │   ├── migrations.rs       # SQLite schema migrations
│   │   │   ├── oauth_config.rs     # OAuth provider configuration
│   │   │   ├── output_styles.rs    # Output style variants
│   │   │   ├── prompt_history.rs   # Prompt input history persistence
│   │   │   ├── provider_id.rs      # ProviderId and ModelId newtypes
│   │   │   ├── ps_classifier.rs    # PowerShell command classifier
│   │   │   ├── remote_session.rs   # Remote session sync
│   │   │   ├── remote_settings.rs  # Remote settings sync
│   │   │   ├── session_storage.rs  # JSONL session transcript persistence
│   │   │   ├── session_tracing.rs  # Session event tracing
│   │   │   ├── settings_sync.rs    # Settings synchronization
│   │   │   ├── skill_discovery.rs  # Filesystem and git URL skill loading
│   │   │   ├── snapshot.rs         # File snapshot/undo system
│   │   │   ├── sqlite_storage.rs   # SQLite-backed session storage
│   │   │   ├── status_notices.rs   # Status notice management
│   │   │   ├── system_prompt.rs    # System prompt assembly
│   │   │   ├── team_memory_sync.rs # Team memory synchronization
│   │   │   ├── tips.rs             # Tip-of-the-day system
│   │   │   ├── token_budget.rs     # Token budget tracking
│   │   │   ├── truncate.rs         # Message truncation utilities
│   │   │   ├── update_check.rs     # Background update checker
│   │   │   └── voice.rs            # Voice recording (optional cpal feature)
│   │   └── tests/
│   │       ├── parity_smoke.rs
│   │       └── test_mcp_templates.rs
│   ├── mcp/                    # Model Context Protocol client
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── backend.rs          # MCP backend trait and implementations
│   │       ├── connection_manager.rs  # Connection lifecycle with backoff
│   │       ├── oauth.rs            # MCP OAuth 2.0 flow
│   │       ├── registry.rs         # MCP server registry
│   │       └── rmcp_backend.rs     # rmcp SDK integration
│   ├── plugins/                # Plugin runtime
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── hooks.rs            # Hook registry and event dispatch
│   │       ├── loader.rs           # Plugin discovery from filesystem
│   │       ├── manifest.rs         # Plugin manifest schema (TOML)
│   │       ├── marketplace.rs      # Plugin marketplace download
│   │       ├── plugin.rs           # LoadedPlugin, PluginCommandDef
│   │       └── registry.rs         # PluginRegistry
│   ├── query/                  # Agentic query loop and orchestration
│   │   └── src/
│   │       ├── lib.rs              # Main query loop, QueryConfig, QueryOutcome, QueryEvent
│   │       ├── agent_tool.rs       # AgentTool — spawns sub-agents
│   │       ├── auto_dream.rs       # Auto-dream background suggestions
│   │       ├── away_summary.rs     # Away summary generation
│   │       ├── command_queue.rs    # CommandQueue bridging TUI → query loop
│   │       ├── compact.rs          # Context compaction (auto-compact, micro-compact)
│   │       ├── context_analyzer.rs # Context window analysis
│   │       ├── coordinator.rs      # Coordinator/worker agent mode
│   │       ├── cron_scheduler.rs   # Cron-based background tasks
│   │       ├── managed_orchestrator.rs  # Managed agent (manager-executor) pattern
│   │       ├── session_memory.rs   # Session memory extraction
│   │       └── skill_prefetch.rs   # Skill index prefetch
│   ├── tools/                  # All tool implementations
│   │   └── src/
│   │       ├── lib.rs              # Tool trait, ToolContext, ToolResult, PermissionLevel
│   │       ├── agent_tool.rs       # Agent spawning tool
│   │       ├── apply_patch.rs      # Patch application tool
│   │       ├── ask_user.rs         # Interactive user question tool
│   │       ├── bash.rs             # BashTool — shell command execution
│   │       ├── batch_edit.rs       # Batch file edit tool
│   │       ├── brief.rs            # BriefTool
│   │       ├── bundled_skills.rs   # Built-in skill definitions
│   │       ├── computer_use.rs     # Screen capture + mouse/keyboard (optional)
│   │       ├── config_tool.rs      # Config read/write tool
│   │       ├── cron.rs             # Cron management tools
│   │       ├── enter_plan_mode.rs  # Enter plan mode tool
│   │       ├── exit_plan_mode.rs   # Exit plan mode tool
│   │       ├── file_edit.rs        # FileEditTool — targeted file editing
│   │       ├── file_read.rs        # FileReadTool — file content reading
│   │       ├── file_write.rs       # FileWriteTool — file creation/overwrite
│   │       ├── formatter.rs        # Post-edit code formatter
│   │       ├── glob_tool.rs        # GlobTool — file pattern matching
│   │       ├── grep_tool.rs        # GrepTool — content search
│   │       ├── lsp_tool.rs         # LSP diagnostics tool
│   │       ├── mcp_auth_tool.rs    # MCP OAuth trigger tool
│   │       ├── mcp_resources.rs    # MCP resource list/read tools
│   │       ├── monitor_tool.rs     # Process monitoring tool
│   │       ├── notebook_edit.rs    # Jupyter notebook edit tool
│   │       ├── powershell.rs       # PowerShell execution tool
│   │       ├── pty_bash.rs         # PTY-based bash (interactive processes)
│   │       ├── remote_trigger.rs   # Remote trigger tool
│   │       ├── repl_tool.rs        # REPL execution tool
│   │       ├── send_message.rs     # SendMessageTool (inter-agent communication)
│   │       ├── skill_tool.rs       # Skill invocation tool
│   │       ├── sleep.rs            # SleepTool
│   │       ├── synthetic_output.rs # SyntheticOutput tool
│   │       ├── tasks.rs            # Task management tools
│   │       ├── team_tool.rs        # Team swarm tools
│   │       ├── todo_write.rs       # TodoWriteTool
│   │       ├── tool_search.rs      # Tool search/discovery
│   │       ├── web_fetch.rs        # WebFetchTool — URL content fetch
│   │       ├── web_search.rs       # WebSearchTool
│   │       └── worktree.rs         # Git worktree management tool
│   └── tui/                    # Terminal UI
│       ├── src/
│       │   ├── lib.rs              # TUI init/teardown, module declarations
│       │   ├── app.rs              # App struct, main event loop, slash command list
│       │   ├── render.rs           # All ratatui rendering logic
│       │   ├── input.rs            # Slash command parsing helpers
│       │   ├── agents_view.rs      # Agents list/detail view
│       │   ├── bridge_state.rs     # Bridge connection status
│       │   ├── bypass_permissions_dialog.rs
│       │   ├── context_viz.rs      # /context overlay
│       │   ├── custom_provider_dialog.rs
│       │   ├── desktop_upsell_startup.rs
│       │   ├── device_auth_dialog.rs
│       │   ├── dialog_select.rs    # Generic selection dialog
│       │   ├── dialogs.rs          # Permission and MCP approval dialogs
│       │   ├── diff_viewer.rs      # Git diff viewer
│       │   ├── elicitation_dialog.rs
│       │   ├── export_dialog.rs    # /export format picker
│       │   ├── feedback_survey.rs
│       │   ├── figures.rs          # Icon/figure constants
│       │   ├── hooks_config_menu.rs
│       │   ├── image_paste.rs      # Clipboard image paste
│       │   ├── import_config_dialog.rs
│       │   ├── invalid_config_dialog.rs
│       │   ├── key_input_dialog.rs
│       │   ├── kitty_image.rs      # Kitty graphics protocol image rendering
│       │   ├── mcp_view.rs         # /mcp server browser
│       │   ├── memory_file_selector.rs
│       │   ├── memory_update_notification.rs
│       │   ├── message_copy.rs     # Clipboard copy of messages
│       │   ├── model_picker.rs     # Model and effort level picker
│       │   ├── notifications.rs    # Notification/banner queue
│       │   ├── onboarding_dialog.rs
│       │   ├── overage_upsell.rs
│       │   ├── overlays.rs         # Help, history-search, message-selector, rewind
│       │   ├── plugin_views.rs     # Plugin hint banners
│       │   ├── privacy_screen.rs
│       │   ├── prompt_input.rs     # PromptInputState, vim mode
│       │   ├── rustle.rs           # Rustle mascot rendering
│       │   ├── session_branching.rs
│       │   ├── session_browser.rs  # Session history browser
│       │   ├── settings_screen.rs  # Full-screen settings UI
│       │   ├── stats_dialog.rs     # Token/cost stats dialog
│       │   ├── tasks_overlay.rs    # Live tasks (agent workers) overlay
│       │   ├── theme_colors.rs     # Theme color palette
│       │   ├── theme_screen.rs     # Theme picker screen
│       │   ├── transcript_turn.rs  # Individual conversation turn rendering
│       │   ├── virtual_list.rs     # Virtual scrolling list widget
│       │   ├── voice_capture.rs    # Voice PTT capture UI
│       │   ├── voice_mode_notice.rs
│       │   └── messages/           # Markdown rendering sub-module
│       │       ├── mod.rs
│       │       ├── markdown.rs
│       │       └── markdown_enhanced.rs
│       └── tests/
│           ├── diff_viewer.rs
│           ├── markdown_enhancements.rs
│           └── render_snapshots.rs
├── .planning/                  # GSD planning workspace (not compiled)
│   ├── codebase/               # Codebase map documents
│   ├── phases/                 # Phase plans
│   ├── quick/                  # Quick task notes
│   └── research/               # Research notes
└── target/                     # Cargo build artifacts (gitignored)
```

## Directory Purposes

**`crates/core/src/`:**
- Purpose: Foundation — all shared primitives with zero workspace-crate dependencies
- Contains: Config/Settings, types, auth, permissions, session storage, context building
- Key files: `lib.rs` (all module declarations), `session_storage.rs`, `sqlite_storage.rs`, `auth_store.rs`

**`crates/api/src/providers/`:**
- Purpose: One file per LLM provider adapter
- Contains: Anthropic, OpenAI, Google, Azure, Bedrock, Cohere, Copilot, Codex, Minimax, OpenAI-compat wrappers
- Key files: `anthropic.rs` (primary), `openai_compat.rs` (base for Groq/Ollama/etc.)

**`crates/tools/src/`:**
- Purpose: One file per tool implementation; all implement the `Tool` trait
- Contains: 30+ tool files plus `lib.rs` with trait definition
- Key files: `lib.rs` (Tool trait), `bash.rs`, `file_edit.rs`, `file_read.rs`, `file_write.rs`

**`crates/tui/src/`:**
- Purpose: All terminal UI code; each file is a distinct screen, dialog, or overlay
- Contains: App state, rendering, all dialogs, overlays, and view components
- Key files: `app.rs` (App struct + event loop), `render.rs` (all drawing)

**`crates/query/src/`:**
- Purpose: Agentic query loop and orchestration logic
- Key files: `lib.rs` (main loop + QueryConfig/QueryOutcome), `compact.rs`, `coordinator.rs`

**`crates/mcp/src/`:**
- Purpose: MCP protocol client implementation
- Key files: `lib.rs`, `connection_manager.rs`, `backend.rs`, `rmcp_backend.rs`

**`crates/plugins/src/`:**
- Purpose: Plugin runtime — discovery, manifest, hooks, marketplace
- Key files: `manifest.rs` (TOML schema), `loader.rs` (discovery), `hooks.rs` (event dispatch)

## Naming Conventions

**Files:**
- `snake_case.rs` throughout — e.g., `file_edit.rs`, `session_storage.rs`, `markdown_enhanced.rs`
- One primary struct/trait per file, file named after the dominant concern
- Protocol or integration files named after their target: `anthropic.rs`, `openai.rs`, `bedrock.rs`

**Directories:**
- `snake_case` — e.g., `providers/`, `transformers/`, `messages/`
- Flat within crate `src/` — sub-directories only when grouping multiple related files (`providers/`, `transformers/`, `messages/`)

**Crates:**
- Binary: `claurst` (package name `claurst`, binary name `claurst`)
- Libraries: `claurst-<name>` — e.g., `claurst-core`, `claurst-api`, `claurst-tools`
- Workspace dependency alias: same as crate name, e.g., `claurst-core = { path = "crates/core" }`

**Structs/Traits:**
- `PascalCase` — e.g., `LlmProvider`, `FileEditTool`, `QueryConfig`, `McpManager`
- Tools named `<Verb><Noun>Tool` — e.g., `FileEditTool`, `WebFetchTool`, `GlobTool`
- Dialogs named `<Purpose>Dialog` or `<Purpose>DialogState` — e.g., `ExportDialogState`, `McpApprovalDialogState`

**Functions:**
- `snake_case` — e.g., `run_query_loop()`, `build_system_context()`, `check_permission()`
- Async functions follow the same convention; no `async_` prefix

## Key File Locations

**Entry Points:**
- `crates/cli/src/main.rs`: Binary entry, all startup logic, mode dispatch
- `crates/cli/build.rs`: Build-time metadata embedding

**Configuration:**
- `Cargo.toml`: Workspace manifest with all shared dependency versions
- `crates/core/src/lib.rs`: Core type re-exports (Config, Settings, types)

**Core Traits:**
- `crates/api/src/provider.rs`: `LlmProvider` trait
- `crates/tools/src/lib.rs`: `Tool` trait, `ToolContext`, `ToolResult`, `PermissionLevel`
- `crates/commands/src/lib.rs`: `SlashCommand` trait, `CommandContext`, `CommandResult`

**Query Loop:**
- `crates/query/src/lib.rs`: `run_query_loop()`, `QueryConfig`, `QueryOutcome`, `QueryEvent`

**TUI App State:**
- `crates/tui/src/app.rs`: `App` struct, event loop, slash command list

**Provider Adapters:**
- `crates/api/src/providers/`: One file per provider

**Testing:**
- `crates/core/tests/`: Integration tests for core utilities
- `crates/tui/tests/`: Snapshot and render tests for TUI components

## Where to Add New Code

**New LLM provider:**
- Implementation: `crates/api/src/providers/<provider_name>.rs` (implement `LlmProvider`)
- Register in: `crates/api/src/providers/mod.rs` and `crates/api/src/registry.rs`
- Auth: `crates/api/src/auth.rs` if new auth pattern needed
- Transformer: `crates/api/src/transformers/<provider_name>.rs` if message format differs

**New tool:**
- Implementation: `crates/tools/src/<tool_name>.rs` (implement `Tool` trait)
- Register in: `crates/tools/src/lib.rs` (`pub mod` + `pub use`)
- Add to tool list: `crates/cli/src/main.rs` where tools are assembled

**New slash command:**
- Implementation: `crates/commands/src/lib.rs` (add to the command match or new module)
- Named subcommand: `crates/commands/src/named_commands.rs`

**New TUI screen/dialog:**
- Implementation: `crates/tui/src/<feature_name>.rs`
- Register in: `crates/tui/src/lib.rs` (`pub mod`)
- Wire into app: `crates/tui/src/app.rs` (App state field + event handling)
- Render: `crates/tui/src/render.rs`

**New core utility:**
- Implementation: `crates/core/src/<utility_name>.rs`
- Register in: `crates/core/src/lib.rs` (`pub mod` + `pub use` if needed)

**New feature flag:**
- Add to: `crates/core/Cargo.toml` under `[features]`
- Add pass-through to: `crates/tui/Cargo.toml` and relevant crates
- Add to `dev_full` group in `crates/core/Cargo.toml`

**New MCP feature:**
- Implementation: `crates/mcp/src/<feature>.rs`
- Register in: `crates/mcp/src/lib.rs`

**New plugin capability:**
- Manifest schema: `crates/plugins/src/manifest.rs`
- Runtime enforcement: `crates/plugins/src/lib.rs` (`check_plugin_capability`)

## Special Directories

**`.planning/`:**
- Purpose: GSD project planning workspace
- Generated: No (manually maintained)
- Committed: Yes

**`target/`:**
- Purpose: Cargo compilation artifacts
- Generated: Yes
- Committed: No (gitignored)

**`crates/cli/src/` (system_prompt.txt):**
- Purpose: Base system prompt embedded at compile time via `include_str!()`
- Referenced in: `crates/cli/src/main.rs`
- Note: Not a `.rs` file but compiled into the binary

---

*Structure analysis: 2026-05-05*
