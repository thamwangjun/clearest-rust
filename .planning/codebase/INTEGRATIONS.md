# External Integrations

**Analysis Date:** 2026-05-05

## LLM Provider APIs

Claurst integrates with a large number of LLM providers through two abstraction layers in `crates/api`:

1. **Native providers** — dedicated client implementations
2. **OpenAI-compatible providers** — generic `OpenAiCompatProvider` wrapper

### Native Providers (`crates/api/src/providers/`)

**Anthropic Claude:**
- File: `crates/api/src/providers/anthropic.rs`
- Endpoint: `https://api.anthropic.com` (overrideable via `ANTHROPIC_BASE_URL`)
- Auth: `ANTHROPIC_API_KEY` env var or `~/.claurst/auth.json` OAuth token
- Transport: HTTPS with SSE streaming (`reqwest` stream)
- Protocol: Anthropic Messages API (native format)

**OpenAI:**
- File: `crates/api/src/providers/openai.rs`
- Endpoint: `https://api.openai.com` (overrideable via `OPENAI_BASE_URL`)
- Auth: `OPENAI_API_KEY`
- Transport: HTTPS with SSE streaming

**Google Gemini / Vertex AI:**
- File: `crates/api/src/providers/google.rs`
- Auth: `GOOGLE_API_KEY` or `GOOGLE_GENERATIVE_AI_API_KEY`; Vertex: `ANTHROPIC_VERTEX_PROJECT_ID` or `CLOUD_ML_PROJECT_ID`
- Transport: HTTPS with SSE streaming

**AWS Bedrock:**
- File: `crates/api/src/providers/bedrock.rs`
- Auth: AWS SigV4 signed requests (HMAC-SHA256 in `crates/api`)
- Config: `AWS_REGION` / `AWS_DEFAULT_REGION`, `AWS_BEDROCK_MODEL_ID`
- Transport: HTTPS

**Azure OpenAI:**
- File: `crates/api/src/providers/azure.rs`
- Auth: `AZURE_API_KEY`

**GitHub Copilot:**
- File: `crates/api/src/providers/copilot.rs`
- Auth: `GITHUB_TOKEN`

**Cohere:**
- File: `crates/api/src/providers/cohere.rs`
- Auth: `COHERE_API_KEY`

**MiniMax:**
- File: `crates/api/src/providers/minimax.rs`
- Auth: `MINIMAX_API_KEY`
- Endpoint: `https://api.minimax.io/anthropic` (overrideable via `MINIMAX_BASE_URL`)

**OpenAI Codex:**
- File: `crates/api/src/providers/codex.rs`
- Auth: handled via `crates/core/src/codex_oauth.rs` OAuth flow

### OpenAI-Compatible Providers (`crates/api/src/providers/openai_compat_providers.rs`)

All use the same HTTP+SSE transport as OpenAI, each with a different base URL and API key env var.

| Provider ID | Env Var | Default Base URL |
|------------|---------|-----------------|
| `ollama` | `OLLAMA_HOST` | `http://localhost:11434/v1` |
| `lmstudio` | `LM_STUDIO_HOST` | `http://localhost:1234/v1` |
| `llamacpp` | `LLAMA_CPP_HOST` | `http://localhost:8080/v1` |
| `deepseek` | `DEEPSEEK_API_KEY` | `https://api.deepseek.com/v1` |
| `groq` | `GROQ_API_KEY` | `https://api.groq.com/openai/v1` |
| `xai` | `XAI_API_KEY` | `https://api.x.ai/v1` |
| `deepinfra` | `DEEPINFRA_API_KEY` | `https://api.deepinfra.com/v1/openai` |
| `cerebras` | `CEREBRAS_API_KEY` | `https://api.cerebras.ai/v1` |
| `togetherai` | `TOGETHER_API_KEY` | `https://api.together.xyz/v1` |
| `perplexity` | `PERPLEXITY_API_KEY` | `https://api.perplexity.ai` |
| `venice` | `VENICE_API_KEY` | `https://api.venice.ai/api/v1` |
| `qwen` | `DASHSCOPE_API_KEY` | `https://dashscope-intl.aliyuncs.com/compatible-mode/v1` |
| `mistral` | `MISTRAL_API_KEY` | `https://api.mistral.ai/v1` |
| `openrouter` | `OPENROUTER_API_KEY` | `https://openrouter.ai/api/v1` |
| `sambanova` | `SAMBANOVA_API_KEY` | `https://api.sambanova.ai/v1` |
| `huggingface` | `HF_TOKEN` | `https://api-inference.huggingface.co/v1` |
| `nvidia` | `NVIDIA_API_KEY` | `https://integrate.api.nvidia.com/v1` |
| `siliconflow` | `SILICONFLOW_API_KEY` | `https://api.siliconflow.cn/v1` |
| `moonshot` | `MOONSHOT_API_KEY` | `https://api.moonshot.cn/v1` |
| `zhipu` | `ZHIPU_API_KEY` | `https://open.bigmodel.cn/api/paas/v4` |
| `zai` | `ZAI_API_KEY` | (custom) |
| `nebius` | `NEBIUS_API_KEY` | `https://api.studio.nebius.ai/v1` |
| `novita` | `NOVITA_API_KEY` | `https://api.novita.ai/v3/openai` |
| `ovhcloud` | `OVHCLOUD_API_KEY` | (custom) |
| `scaleway` | `SCALEWAY_API_KEY` | (custom) |
| `vultr` | `VULTR_API_KEY` | `https://api.vultrinference.com/v1` |
| `baseten` | `BASETEN_API_KEY` | (custom) |
| `friendli` | `FRIENDLI_TOKEN` | `https://inference.friendli.ai/v1` |
| `upstage` | `UPSTAGE_API_KEY` | `https://api.upstage.ai/v1` |
| `stepfun` | `STEPFUN_API_KEY` | `https://api.stepfun.com/v1` |
| `fireworks` | `FIREWORKS_API_KEY` | `https://api.fireworks.ai/inference/v1` |

AI gateway providers also listed in `crates/core/src/lib.rs`:
- `cloudflare` / `cloudflare-ai-gateway` — `CLOUDFLARE_API_TOKEN`
- `vercel` — `AI_GATEWAY_API_KEY`
- `helicone` — `HELICONE_API_KEY`
- `sap` / `sap-ai-core` — `AICORE_SERVICE_KEY`
- `gitlab` — `GITLAB_TOKEN`

## Data Storage

**Embedded SQLite:**
- Crate: `rusqlite` 0.31 (bundled — no system dependency)
- Implementation: `crates/core/src/sqlite_storage.rs`
- Location: `~/.claurst/` (exact filename determined at runtime)
- Schema: `sessions` table + `messages` table with indexes on `session_id` and `updated_at`
- Purpose: Session history, message transcripts
- Alternative: JSONL flat file storage (also in `crates/core/src/session_storage.rs`)

**Filesystem:**
- Credential store: `~/.claurst/auth.json` (`crates/core/src/auth_store.rs`)
- Feature flag cache: `~/.claurst/feature_flags.json` (`crates/core/src/feature_flags.rs`)
- Prompt history: `~/.claurst/` (managed by `crates/core/src/prompt_history.rs`)
- Plugin archives: downloaded via HTTPS, extracted to local dir (`crates/plugins`)
- Temporary files: `tempfile` crate for intermediate operations

## Authentication & Identity

**Multi-path auth — resolved in priority order:**
1. Environment variable (e.g. `ANTHROPIC_API_KEY`)
2. `~/.claurst/auth.json` stored credential (API key or OAuth token)
3. Interactive login flow

**OAuth 2.0 Flows:**

*Anthropic / Claude.ai OAuth:*
- Implementation: `crates/core/src/oauth_config.rs`, `crates/cli/src/oauth_flow.rs`
- Scopes: `user:inference`, `user:profile`, `user:sessions:claude_code`, `user:mcp_servers`, `user:file_upload`
- Endpoints: claude.ai authorization server and api.anthropic.com token endpoint
- Flow: Authorization Code with PKCE (device-code variant for headless)

*Device Code Flow:*
- Implementation: `crates/core/src/device_code.rs`
- Used when browser is unavailable; displays QR code via `qrcode` crate

*OpenAI Codex OAuth:*
- Implementation: `crates/core/src/codex_oauth.rs`, `crates/cli/src/codex_oauth_flow.rs`

*MCP OAuth:*
- Implementation: `crates/mcp/src/oauth.rs`
- Used when MCP servers require OAuth authorization

**Credential Storage:**
- `crates/core/src/auth_store.rs` — JSON store at `~/.claurst/auth.json`
- Stores both `ApiKey { key }` and `OAuthToken { access, refresh, expires }` variants
- Cryptographic utilities in `crates/core/src/crypto_utils.rs`

## Model Context Protocol (MCP)

**Role:** Outbound client — connects Claurst to external MCP tool/resource servers

**Implementation:** `crates/mcp/`
- `rmcp` 1.4.0 SDK — protocol transport and framing
- `crates/mcp/src/lib.rs` — connection manager, tool discovery, tool execution
- `crates/mcp/src/backend/` — stdio (subprocess) and HTTP/SSE transports
- `crates/mcp/src/rmcp_backend.rs` — `rmcp` adapter

**Transports supported:**
- Stdio (subprocess) — MCP server launched as child process
- HTTP/SSE (streamable HTTP) — remote MCP servers
- OAuth-authenticated MCP servers (`crates/mcp/src/oauth.rs`)

**Config:** MCP server definitions in `~/.claurst/settings.json` under `mcpServers` key (`McpServerConfig` type)

## Remote Bridge (claude.ai Web UI)

**Role:** Bidirectional long-poll bridge connecting local CLI to claude.ai web sessions

**Implementation:** `crates/bridge/src/lib.rs`
- Protocol mirrors TypeScript `bridgeMain.ts` / `bridgeApi.ts`
- Device fingerprinting via SHA-256 for trusted-device identification
- JWT decode utilities (client-side, no signature verification) for session-ingress tokens
- Long-polling with exponential backoff and `CancellationToken`
- External URL: `https://api.claude.ai` (hardcoded in `crates/core/src/remote_session.rs`)

**Feature gate:** `bridge_mode` compile-time feature flag in `crates/core`

## Remote Session Sync

**Role:** Sync local session transcripts to claude.ai cloud

**Implementation:** `crates/core/src/remote_session.rs`
- WebSocket client via `tokio-tungstenite`
- Base URL: `https://api.claude.ai`
- Auth: OAuth `access_token` Bearer header
- Session events: `SessionCreated`, `SessionUpdated`, `SessionDeleted` over WebSocket

## Web Search

**Role:** Web search tool for agent use

**Implementation:** `crates/tools/src/web_search.rs`

**Primary: Brave Search API**
- Env var: `BRAVE_SEARCH_API_KEY`
- Endpoint: `https://api.search.brave.com/res/v1/web/search`

**Fallback: DuckDuckGo**
- No API key required
- Endpoint: `https://duckduckgo.com/` (HTML scrape)
- Used when `BRAVE_SEARCH_API_KEY` is absent or empty

## Feature Flags (GrowthBook)

**Role:** Remote feature flag management

**Implementation:** `crates/core/src/feature_flags.rs`
- Service: GrowthBook (`https://api.growthbook.io/api/features`)
- Auth: `GROWTHBOOK_API_KEY` env var
- Cache: `~/.claurst/feature_flags.json` (1-hour TTL)
- Fallback: cached flags if fetch fails; empty if no cache

## Plugin Marketplace

**Role:** Download and install community plugins

**Implementation:** `crates/plugins/src/`
- HTTP downloads via `reqwest`
- Archive format: ZIP with `deflate` (extracted via `zip` crate)
- Integrity: SHA-256 hash verification
- Install path: local user directory (resolved via `dirs`)

## IDE / Editor Integration

**ACP (Agent Client Protocol):**
- Implementation: `crates/acp/src/lib.rs`
- Protocol: JSON-RPC 2.0 over stdio
- Used by: Zed, VS Code, and other editors to use Claurst as an AI back-end
- Transport: line-delimited JSON over stdin/stdout

**LSP support:**
- Implementation: `crates/core/src/lsp.rs`
- Provides Language Server Protocol integration for code context

## Monitoring & Observability

**Logging:**
- `tracing` 0.1 + `tracing-subscriber` 0.3 (`env-filter`, `json` features)
- Log level controlled by `RUST_LOG` env var (standard `EnvFilter`)
- JSON output format available for structured logging

**Session Metrics:**
- Implementation: `crates/core/src/analytics.rs`
- In-process counters (AtomicU64) for cost, tokens, API latency, tool usage
- No external telemetry destination found — metrics are session-local

## CI/CD & Deployment

**Hosting:** Not detected (library/CLI tool distributed as binary)

**CI Pipeline:** Not detected (no `.github/workflows/` or similar found)

## Webhooks & Callbacks

**Incoming:** None — no HTTP server in this codebase; ACP and MCP use stdio

**Outgoing:** Long-poll requests to claude.ai bridge API (not traditional webhooks)

---

*Integration audit: 2026-05-05*
