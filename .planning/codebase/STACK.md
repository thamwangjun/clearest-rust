# Technology Stack

**Analysis Date:** 2026-05-05

## Languages

**Primary:**
- Rust 2021 edition — entire codebase; all 12 workspace crates

**Secondary:**
- None (pure Rust project, no embedded JS/Python/shell)

## Runtime

**Environment:**
- Native binary (no VM/interpreter)
- Tested on: rustc 1.95.0 (2026-04-14)

**Package Manager:**
- Cargo 1.95.0
- Lockfile: `Cargo.lock` (present — binary project, should be committed)

## Workspace Layout

12 crates under `crates/`, unified by `Cargo.toml` at repo root using Cargo workspace resolver v2.

| Crate | Binary/Lib | Purpose |
|-------|-----------|---------|
| `crates/cli` | Binary (`claurst`) | CLI entry point, argument parsing, TUI/headless dispatch |
| `crates/core` | Lib | Types, config, session storage, auth, OAuth, feature flags |
| `crates/api` | Lib | LLM provider abstraction, streaming, model registry |
| `crates/tui` | Lib | Ratatui terminal UI, rendering, input handling |
| `crates/tools` | Lib | Built-in agent tools (bash, file ops, web search, computer-use) |
| `crates/query` | Lib | Conversation orchestration, query/response pipeline |
| `crates/commands` | Lib | Slash-command handlers (`:help`, `:config`, etc.) |
| `crates/mcp` | Lib | Model Context Protocol (MCP) client — stdio + HTTP/SSE |
| `crates/bridge` | Lib | Remote bridge to claude.ai web UI (long-poll over HTTPS) |
| `crates/buddy` | Lib | Lightweight conversation memory / summarization helper |
| `crates/plugins` | Lib | Plugin runtime — marketplace download, zip extraction, lifecycle |
| `crates/acp` | Lib | Agent Client Protocol server — JSON-RPC 2.0 over stdio for editor integration |

## Frameworks

**Async Runtime:**
- `tokio` 1.44 (full features) — all async I/O and task scheduling
- `tokio-stream` 0.1 — streaming combinators
- `tokio-util` 0.7 (`codec`, `rt`) — codec framing, cancellation tokens
- `futures` 0.3 — core Future/Stream traits
- `async-trait` 0.1 — async trait methods
- `async-stream` 0.3 — generator-style stream macros

**Terminal UI:**
- `ratatui` 0.29 — TUI widget framework (`crates/tui`)
- `crossterm` 0.28 (`event-stream`) — cross-platform terminal I/O

**CLI Parsing:**
- `clap` 4 (`derive`, `env`, `string` features) — argument parsing in `crates/cli`

**HTTP:**
- `reqwest` 0.13 (`json`, `stream`, `native-tls`, `multipart`, `form`, `query`) — all outbound HTTP; native-tls for TLS

**WebSocket:**
- `tokio-tungstenite` 0.24 (`native-tls`) — WebSocket client for remote bridge and session sync

**MCP Protocol:**
- `rmcp` 1.4.0 — Model Context Protocol SDK (`client`, `auth`, `transport-child-process`, `transport-streamable-http-client-reqwest`, `reqwest-native-tls` features) in `crates/mcp`

**Serialization:**
- `serde` 1 (`derive`) — all types
- `serde_json` 1 — JSON everywhere (API wire format, config, storage)
- `toml` 0.8 — TOML config file parsing
- `indexmap` 2 (`serde`) — ordered maps with serde support

**JSON Schema:**
- `schemars` 0.8 (`derive`) — schema generation for tool input schemas

## Key Dependencies

**Storage:**
- `rusqlite` 0.31 (`bundled`) — embedded SQLite for session/message storage in `crates/core/src/sqlite_storage.rs`; bundled variant (no system lib dependency)

**Cryptography / Hashing:**
- `sha2` 0.10 — SHA-256 for content hashing and auth
- `hmac` 0.12 — HMAC signatures in `crates/api` (AWS Bedrock signing)
- `xxhash-rust` 0.8 (`xxh64`) — fast non-cryptographic hash in `crates/api`
- `base64` 0.22 — Base64 encode/decode
- `hex` 0.4 — hex encoding
- `getrandom` 0.2 — secure random bytes

**Image / Terminal Graphics:**
- `image` 0.25 (`png`, `jpeg`) — image decoding for attachments
- `icy_sixel` 0.5 — Sixel protocol for in-terminal image rendering
- `xcap` 0.0.13 — screen capture (computer-use feature, `crates/tools`)
- `enigo` 0.2 — keyboard/mouse control (computer-use feature)

**Text Processing:**
- `similar` 2 — diff/patch computation (file edit tools)
- `syntect` 5 (`default-syntaxes`, `default-themes`, `regex-fancy`) — syntax highlighting in TUI
- `unicode-width` 0.2 / `unicode-segmentation` 1 — Unicode-aware text layout

**Process / PTY:**
- `portable-pty` 0.9 — PTY support for `PtyBashTool` in `crates/tools/src/pty_bash.rs`
- `nix` 0.29 (`process`, `signal`, `user`) — Unix process management
- `which` 7 — executable lookup

**Concurrency Primitives:**
- `parking_lot` 0.12 — fast Mutex/RwLock
- `dashmap` 6 — concurrent HashMap

**Utilities:**
- `uuid` 1 (`v4`) — UUIDs for sessions and messages
- `chrono` 0.4 (`serde`) — timestamps
- `regex` 1 — regex matching
- `glob` 0.3 — glob path patterns
- `walkdir` 2 — recursive directory traversal
- `tempfile` 3 — temporary files/dirs (dev-dependency and tools)
- `dirs` 5 — platform config/home directories
- `once_cell` 1 — lazy statics
- `bytes` 1 — byte buffer types
- `url` 2 — URL parsing/validation
- `mime` 0.3 — MIME type handling
- `urlencoding` 2 — URL percent-encoding
- `open` 5 — open URLs/files in system browser
- `qrcode` 0.14 — QR code generation (device-code OAuth display)
- `hostname` 0.4 — machine hostname lookup
- `arboard` 3 — clipboard access in `crates/commands`
- `zip` 2 (`deflate`) — zip extraction for plugin archives in `crates/plugins`
- `hound` 3.5 — WAV audio file I/O (voice feature, optional)
- `cpal` 0.15 — audio capture (optional `voice` feature in `crates/core` and `crates/tui`)

## Feature Flags

`crates/core` defines 36 compile-time feature flags covering UI, agent/memory, and infrastructure capabilities. The default build enables `ultraplan`. A `dev_full` meta-feature enables all 36 flags.

Notable flags:
- `voice` — enables `cpal` microphone capture
- `computer-use` — enables `enigo` + `xcap` + `image` in `crates/tools`
- `bridge_mode` — remote bridge to claude.ai web UI
- `ultraplan` (default on) — extended planning capabilities

## Configuration

**Files:**
- `~/.claurst/settings.json` — user settings loaded at startup
- `~/.claurst/auth.json` — persisted API keys and OAuth tokens (`crates/core/src/auth_store.rs`)
- `~/.claurst/feature_flags.json` — cached GrowthBook flags (`crates/core/src/feature_flags.rs`)
- `.claurst/settings.json` — project-level settings (per-repo override)
- `AGENTS.md` / `.claude/AGENTS.md` — project agent instructions (loaded by `ContextBuilder`)

**Environment Variables (key runtime vars):**
- `ANTHROPIC_API_KEY` — Anthropic Claude API key
- `ANTHROPIC_BASE_URL` — override Anthropic API base URL
- `OPENAI_API_KEY` — OpenAI API key
- `OPENAI_BASE_URL` — override OpenAI base URL
- `CLAURST_SIMPLE` — strip feature flags to minimal mode
- `CLAURST_SKIP_PROMPT_HISTORY` — disable prompt history persistence
- `CLAURST_REMOTE` — enable remote/cloud session mode
- `GROWTHBOOK_API_KEY` — GrowthBook feature flag API key
- `BRAVE_SEARCH_API_KEY` — Brave Search API (optional; falls back to DuckDuckGo)
- `AWS_REGION` / `AWS_DEFAULT_REGION` — AWS Bedrock region
- `ANTHROPIC_VERTEX_PROJECT_ID` / `CLOUD_ML_PROJECT_ID` — Google Vertex AI project
- `OLLAMA_HOST` — Ollama server URL (default: `http://localhost:11434`)
- `LM_STUDIO_HOST` / `LLAMA_CPP_HOST` — local model server URLs

**Build-time env vars** (set via `build.rs` in `crates/cli`):
- `BUILD_TIME` — RFC 3339 build timestamp
- `GIT_COMMIT` — short git commit hash
- `PACKAGE_URL`, `FEEDBACK_CHANNEL`, `ISSUES_EXPLAINER` — distribution metadata

## Platform Requirements

**Development:**
- Rust stable (tested: 1.95.0)
- Cargo workspace resolver v2
- Unix preferred (Linux/macOS) — `nix` crate used for PTY/signal; `PtyBashTool` is Unix-only (`crates/tools/src/pty_bash.rs`)

**Production:**
- Single statically-linked binary (`claurst`)
- SQLite bundled (no system SQLite dependency)
- TLS via native-tls (links system TLS: SecureTransport on macOS, OpenSSL on Linux)
- Terminal emulator required for TUI mode; headless mode works without one

---

*Stack analysis: 2026-05-05*
