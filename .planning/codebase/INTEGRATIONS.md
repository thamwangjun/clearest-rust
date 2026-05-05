# Integrations

**Analysis Date:** 2026-05-04

## External APIs

### LLM Providers

All providers are implemented in `crates/api/src/providers/` and registered via `crates/api/src/registry.rs`.

**Anthropic (primary / default)**
- Provider file: `crates/api/src/providers/anthropic.rs`
- Auth env var: `ANTHROPIC_API_KEY`
- Transformer: `crates/api/src/transformers/anthropic.rs`

**OpenAI**
- Provider file: `crates/api/src/providers/openai.rs`
- Auth env var: `OPENAI_API_KEY`
- Transformer: `crates/api/src/transformers/openai_chat.rs`
- Custom base URL override supported

**Google Gemini / Vertex AI**
- Provider file: `crates/api/src/providers/google.rs`
- Auth env vars: `GOOGLE_API_KEY` or `GOOGLE_GENERATIVE_AI_API_KEY`
- Vertex AI: `ANTHROPIC_VERTEX_PROJECT_ID` or `CLOUD_ML_PROJECT_ID`

**AWS Bedrock**
- Provider file: `crates/api/src/providers/bedrock.rs`
- Auth env vars: `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, `AWS_SESSION_TOKEN` (optional), `AWS_BEARER_TOKEN_BEDROCK` (token auth alternative)
- Region env vars: `AWS_REGION` or `AWS_DEFAULT_REGION`
- Model env var: `AWS_BEDROCK_MODEL_ID`
- Uses HMAC-SHA request signing (`hmac` crate)

**Azure OpenAI**
- Provider file: `crates/api/src/providers/azure.rs`
- Auth env vars: `AZURE_API_KEY`, `AZURE_RESOURCE_NAME`, `AZURE_API_VERSION`

**Cohere**
- Provider file: `crates/api/src/providers/cohere.rs`
- Auth env var: `COHERE_API_KEY`

**GitHub Copilot**
- Provider file: `crates/api/src/providers/copilot.rs`
- Auth: API key passed via config

**OpenAI Codex**
- Provider file: `crates/api/src/providers/codex.rs`, adapter: `crates/api/src/codex_adapter.rs`
- Auth: stored credentials (not an env var key)

**MiniMax**
- Provider file: `crates/api/src/providers/minimax.rs`
- Auth: API key passed via config

### OpenAI-Compatible Providers (via `crates/api/src/providers/openai_compat_providers.rs`)

All use `reqwest` with OpenAI-format JSON API:

| Provider ID | Base URL | Auth env var |
|-------------|----------|-------------|
| `ollama` | `http://localhost:11434/v1` (configurable) | None (local) |
| `deepseek` | `https://api.deepseek.com/v1` | Provider API key |
| `groq` | `https://api.groq.com/openai/v1` | Provider API key |
| `xai` | xAI API | Provider API key |
| `togetherai` / `together-ai` | `https://api.together.xyz/v1` | Provider API key |
| `qwen` | Alibaba Cloud | Provider API key |
| `mistral` | `https://api.mistral.ai/v1` | Provider API key |
| `openrouter` | `https://openrouter.ai/api/v1` | Provider API key |

### Web Search

- **Brave Search API** — primary web search backend
  - File: `crates/tools/src/web_search.rs`
  - Auth env var: `BRAVE_SEARCH_API_KEY`
  - Fallback: DuckDuckGo (no API key required)
- **WebFetch** — direct HTTP page fetch via `reqwest`
  - File: `crates/tools/src/web_fetch.rs`

## Services / Databases

### Embedded SQLite

- Library: `rusqlite` `0.31` (bundled — SQLite compiled into the binary)
- Storage file: `crates/core/src/sqlite_storage.rs`
- Purpose: Conversation history, session persistence, prompt history
- Location: Platform config directory (`dirs` crate resolves path)
- Also accessed from ACP server: `crates/acp/src/lib.rs`

### CCR Remote Sessions (Claude.ai Bridge)

- Purpose: Mirrors a local Claurst session to claude.ai for remote access
- Implementation: `crates/bridge/src/lib.rs`
- Protocol: WebSocket (`tokio-tungstenite`) + HTTP (`reqwest`)
- Session registration: CCR server endpoint (Anthropic-hosted)
- Feature gates: `ccr_auto_connect`, `ccr_mirror`, `ccr_remote_setup` (in `claurst-core` features)
- Remote session types: `crates/core/src/remote_session.rs`

### Plugin Marketplace

- Purpose: Plugin discovery, download, and installation
- Implementation: `crates/plugins/src/` 
- Transport: HTTP (`reqwest`) for marketplace API and plugin archive downloads
- Archive format: ZIP (extracted via `zip` crate)
- Integrity: SHA-256 hash verification (`sha2` + `hex`)

## Auth

**API Key auth (most providers):** Plain bearer token in `Authorization` header; key sourced from environment variables (see provider list above).

**AWS Bedrock auth:** SigV4-style request signing using `hmac` + `sha2`; credentials from `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_SESSION_TOKEN` or `AWS_BEARER_TOKEN_BEDROCK` for token-based auth.

**Azure auth:** API key in header; `AZURE_API_KEY` + `AZURE_RESOURCE_NAME` + `AZURE_API_VERSION` required.

**Codex auth:** Stored credentials (not env var); loaded via `CodexProvider::from_stored()`.

**No centralized auth framework** — each provider handles its own credential resolution inside its `from_config()` / `new()` constructor.

## Notable Third-Party SDKs

**`rmcp` `1.4.0`** — Official Rust MCP (Model Context Protocol) SDK
- Used in: `crates/mcp/src/lib.rs`, `crates/mcp/src/rmcp_backend.rs`
- Features enabled: `client`, `auth`, `transport-child-process`, `transport-streamable-http-client-reqwest`, `reqwest-native-tls`
- Supports: child-process stdio transport (local MCP servers), streamable HTTP transport (remote MCP servers), legacy SSE transport

**`portable-pty` `0.9`** — PTY (pseudo-terminal) support
- Used in: `crates/tools/src/pty_bash.rs`
- Purpose: Wraps Bash execution in a real PTY so interactive programs behave correctly

**`enigo` `0.2`** — Keyboard and mouse automation
- Used in: `crates/tools/src/computer_use.rs` (gated behind `computer-use` feature)

**`xcap` `0.0.13`** — Cross-platform screen capture
- Used in: `crates/tools/src/computer_use.rs` (gated behind `computer-use` feature)

**`cpal` `0.15`** — Cross-platform audio I/O
- Used in: `crates/core/src/` (voice recorder, gated behind `voice` feature)
- Also pulled into `claurst-tui` for voice PTT mode

**`arboard` `3`** — Cross-platform clipboard access
- Used in: `crates/commands/src/lib.rs`

**`syntect` `5`** — Syntax highlighting engine
- Used in: `crates/tui/src/` for code block rendering in the TUI
- Config: `default-syntaxes`, `default-themes`, `regex-fancy` features

**`icy_sixel` `0.5`** — Sixel graphics protocol encoder
- Used in: `crates/tui/src/` for in-terminal image rendering

**`qrcode` `0.14`** — QR code generation
- Used in: `crates/commands/src/lib.rs` for session sharing

## Communication Protocols

**HTTP/HTTPS (REST + SSE):**
- All LLM provider API calls — streaming via Server-Sent Events (SSE) parsed in `crates/api/src/stream_parser.rs`
- Web search (Brave, DuckDuckGo), web fetch
- Plugin marketplace
- CCR bridge registration

**WebSocket (WSS):**
- Remote session sync — `crates/core/src/remote_session.rs`
- CCR bridge real-time channel — `crates/bridge/src/lib.rs`
- MCP servers over HTTP/WS — `crates/mcp/`
- Library: `tokio-tungstenite` with `native-tls`

**JSON-RPC 2.0 over stdio:**
- ACP (Agent Client Protocol) server — `crates/acp/src/lib.rs`
- Purpose: Editor integrations (Zed, VS Code) communicate with Claurst as a subprocess
- MCP child-process transport (via `rmcp`) — `crates/mcp/src/rmcp_backend.rs`

**PTY (pseudo-terminal):**
- Bash tool execution — `crates/tools/src/pty_bash.rs`
- MCP local server process launch via `rmcp` transport

## Feature Flags

**GrowthBook** — remote feature flag management
- File: `crates/core/src/feature_flags.rs`
- API endpoint: `https://api.growthbook.io/api/features`
- Auth env var: `GROWTHBOOK_API_KEY`
- Caches flags locally; falls back to defaults if unavailable

## Observability

**Telemetry:**
- File: `crates/core/src/analytics.rs`
- First-party analytics events (no PII); defaults to **off**
- Opt-in env var: `CLAURST_ENABLE_TELEMETRY=1`
- OSS build ships a no-op stub for all telemetry functions — all data is discarded unless a first-party endpoint is configured

**Structured logging:**
- `tracing` / `tracing-subscriber` throughout all crates
- JSON log format available via `tracing-subscriber` json feature

## Environment Variables Reference

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_API_KEY` | Anthropic API authentication |
| `OPENAI_API_KEY` | OpenAI API authentication |
| `GOOGLE_API_KEY` / `GOOGLE_GENERATIVE_AI_API_KEY` | Google Gemini auth |
| `ANTHROPIC_VERTEX_PROJECT_ID` / `CLOUD_ML_PROJECT_ID` | Google Vertex AI project |
| `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_SESSION_TOKEN` | AWS Bedrock credentials |
| `AWS_BEARER_TOKEN_BEDROCK` | AWS Bedrock token auth alternative |
| `AWS_REGION` / `AWS_DEFAULT_REGION` | AWS region for Bedrock |
| `AWS_BEDROCK_MODEL_ID` | Override Bedrock model ID |
| `AZURE_API_KEY` | Azure OpenAI API key |
| `AZURE_RESOURCE_NAME` | Azure resource name |
| `AZURE_API_VERSION` | Azure API version |
| `COHERE_API_KEY` | Cohere API authentication |
| `BRAVE_SEARCH_API_KEY` | Brave Search web search |
| `GROWTHBOOK_API_KEY` | GrowthBook feature flags |
| `CLAURST_ENABLE_TELEMETRY` | Opt-in telemetry (default: off) |
| `CLAURST_REMOTE` | Enable remote/CCR mode |
| `CLAURST_SIMPLE` / `--bare` | Minimal/bare UI mode |
| `CLAURST_SKIP_PROMPT_HISTORY` | Disable prompt history persistence |

---

*Integration audit: 2026-05-04*
