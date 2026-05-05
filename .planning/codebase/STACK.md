# Technology Stack

**Analysis Date:** 2026-05-04

## Language & Runtime

**Primary:**
- Rust (Edition 2021) — entire codebase; all crates target stable Rust
- No explicit `rust-toolchain.toml` present; toolchain defaults to system Rust

**No secondary languages detected.** (Pure Rust workspace.)

## Build System

**Cargo** — Rust's built-in package manager and build system
- Workspace resolver: `2` (feature-aware dependency resolution)
- Root manifest: `Cargo.toml` (workspace definition)
- Lockfile: `Cargo.lock` — committed (binary application, pinned deps)
- Build script: `crates/cli/build.rs` — embeds `BUILD_TIME`, `GIT_COMMIT`, `PACKAGE_URL` at compile time via `cargo:rustc-env`

## Workspace Crates

| Crate | Binary / Library | Role |
|-------|-----------------|------|
| `crates/cli` (`claurst`) | **Binary** | Entry point; wires all crates together, owns `clap` CLI |
| `crates/core` (`claurst-core`) | Library | Shared types, session state, settings, SQLite storage, feature flags |
| `crates/api` (`claurst-api`) | Library | LLM provider abstraction layer and all provider implementations |
| `crates/tools` (`claurst-tools`) | Library | Tool implementations (Bash, PTY, file I/O, web search, computer-use, MCP auth, etc.) |
| `crates/query` (`claurst-query`) | Library | Query execution orchestrator; connects API + tools + plugins |
| `crates/tui` (`claurst-tui`) | Library | Terminal UI (Ratatui-based); all screen/dialog rendering |
| `crates/commands` (`claurst-commands`) | Library | Slash-command handlers (clipboard, QR, session sync, etc.) |
| `crates/mcp` (`claurst-mcp`) | Library | Model Context Protocol (MCP) client |
| `crates/acp` (`claurst-acp`) | Library | Agent Client Protocol server (JSON-RPC 2.0 over stdio for editor integrations) |
| `crates/bridge` (`claurst-bridge`) | Library | CCR (Claude.ai remote) bridge / remote session manager |
| `crates/buddy` (`claurst-buddy`) | Library | Lightweight companion utility (minimal deps) |
| `crates/plugins` (`claurst-plugins`) | Library | Plugin runtime — discovery, download, ZIP extraction, marketplace |

## Key Dependencies

**Async Runtime:**
- `tokio` `1.44` (features: full) — async executor; used across every crate
- `tokio-stream` `0.1` — async stream utilities
- `tokio-util` `0.7` — codec, runtime helpers
- `futures` `0.3` — `Future`/`Stream` combinators
- `async-trait` `0.1` — trait-level async support
- `async-stream` `0.3` — generator-style async streams

**HTTP Client:**
- `reqwest` `0.13` (features: json, stream, native-tls, multipart, form, query) — all outbound HTTP; used by API providers, web search, plugin downloads

**Serialization:**
- `serde` `1` + `serde_json` `1` — ubiquitous JSON ser/de
- `toml` `0.8` — TOML config parsing
- `schemars` `0.8` — JSON Schema generation for tool definitions

**CLI Argument Parsing:**
- `clap` `4` (features: derive, env, string) — used exclusively in `crates/cli`

**Terminal UI:**
- `ratatui` `0.29` — TUI widget library
- `crossterm` `0.28` (features: event-stream) — cross-platform terminal control
- `unicode-width` `0.2` + `unicode-segmentation` `1` — Unicode-aware text layout

**Error Handling:**
- `anyhow` `1` — application-level errors with context
- `thiserror` `2` — typed library errors

**Logging / Tracing:**
- `tracing` `0.1` — structured logging/spans
- `tracing-subscriber` `0.3` (features: env-filter, json) — subscriber config in CLI binary

**Storage:**
- `rusqlite` `0.31` (features: bundled) — embedded SQLite for conversation/session persistence; used in `claurst-core` (`src/sqlite_storage.rs`)

**Concurrency / Data Structures:**
- `parking_lot` `0.12` — fast `Mutex`/`RwLock`
- `dashmap` `6` — concurrent `HashMap`
- `indexmap` `2` — ordered `HashMap` with serde support
- `once_cell` `1` — lazy statics

**Cryptography / Hashing:**
- `sha2` `0.10` — SHA-256 for content hashing and HMAC
- `hmac` `0.12` — HMAC-SHA (API request signing in `claurst-api`)
- `xxhash-rust` `0.8` (xxh64) — fast non-cryptographic hash in `claurst-api`
- `hex` `0.4` — hex encoding
- `base64` `0.22` — base64 encoding
- `getrandom` `0.2` — OS random bytes

**Utilities:**
- `uuid` `1` (v4) — session/message IDs
- `chrono` `0.4` (serde) — timestamps; also used in build script
- `regex` `1` — pattern matching
- `glob` `0.3` — file glob patterns
- `walkdir` `2` — recursive directory traversal
- `tempfile` `3` — temporary files in tests and tools
- `dirs` `5` — platform-aware home/config directories
- `which` `7` — executable discovery on PATH
- `bytes` `1` — byte buffer utilities
- `url` `2` + `urlencoding` `2` + `mime` `0.3` — URL handling
- `open` `5` — open files/URLs in default OS application
- `qrcode` `0.14` — QR code generation (session share)
- `similar` `2` — text diffing (patch/edit tools)
- `syntect` `5` — syntax highlighting in TUI
- `hostname` `0.4` — machine hostname (bridge + commands)

**WebSocket:**
- `tokio-tungstenite` `0.24` (native-tls) — WebSocket client for remote session sync and CCR bridge

**Process / PTY:**
- `portable-pty` `0.9` — pseudo-terminal for `PtyBashTool` (Unix)
- `nix` `0.29` (features: process, signal, user) — Unix syscalls for process management

**Image / Audio:**
- `image` `0.25` (png, jpeg) — image handling for computer-use and screenshot tools
- `xcap` `0.0.13` — screen capture (gated behind `computer-use` feature)
- `enigo` `0.2` — keyboard/mouse automation (gated behind `computer-use` feature)
- `icy_sixel` `0.5` — Sixel image protocol for TUI image rendering
- `cpal` `0.15` — audio capture (gated behind `voice` feature)
- `hound` `3.5` — WAV audio encoding (gated behind `voice` feature in TUI)

**MCP Protocol:**
- `rmcp` `1.4.0` — official Rust MCP SDK (features: client, auth, transport-child-process, transport-streamable-http-client-reqwest, reqwest-native-tls)

**Plugin System:**
- `zip` `2` (deflate) — ZIP archive extraction for plugin downloads (in `claurst-plugins`)

**Clipboard:**
- `arboard` `3` — cross-platform clipboard access (in `claurst-commands`)

## Dev Dependencies

- `tempfile` `3` — used in test suites for `claurst-core`, `claurst-tools`, `claurst-query`, `claurst-buddy`, `claurst-plugins`, `claurst-tui`
- `tokio` (full features) — async test runtime

## Feature Flags

`claurst-core` defines ~36 Cargo feature flags for experimental capabilities, grouped into:
- **UI/Interaction**: `ultraplan`, `ultrathink`, `history_picker`, `token_budget`, `message_actions`, `quick_search`, `away_summary`, `hook_prompts`, `kairos_brief`, `kairos_channels`, `lodestone`
- **Agents/Memory**: `agent_triggers`, `agent_triggers_remote`, `extract_memories`, `verification_agent`, `builtin_explore_plan_agents`, `cached_microcompact`, `compaction_reminders`, `agent_memory_snapshot`, `teammem`
- **Tools/Infrastructure**: `bash_classifier`, `bridge_mode`, `mcp_rich_output`, `connector_text`, `unattended_retry`, `new_init`, `powershell_auto_mode`, `shot_stats`, `tree_sitter_bash`, `tree_sitter_bash_shadow`, `native_clipboard_image`, `ccr_auto_connect`, `ccr_mirror`, `ccr_remote_setup`, `prompt_cache_break_detection`
- **Hardware**: `voice` (microphone via `cpal`), `computer-use` (`xcap` + `enigo` + `image`)
- `dev_full` meta-feature enables all experimental flags for development builds

All feature flags propagate via pass-through from `claurst-tui` and `claurst-commands` up to the `claurst` binary crate.

## Toolchain

- **Language:** Rust, Edition 2021
- **Version:** No pinned toolchain file; requires a recent stable Rust (Tokio 1.44 requires Rust 1.75+)
- **Package Manager:** Cargo (workspace)
- **License:** GPL-3.0

---

*Stack analysis: 2026-05-04*
