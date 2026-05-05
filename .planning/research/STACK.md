# Technology Stack Research: claurst Milestone

**Project:** claurst — Rust rewrite of Claude Code
**Researched:** 2026-05-04
**Scope:** Crates and patterns to adopt/avoid for missing features; existing codebase analyzed

---

## Existing Stack (Do Not Change)

The workspace already has a well-chosen dependency set. All crates below are
inherited; this document only discusses additions and patterns on top of them.

| Category | Crate | Version |
|----------|-------|---------|
| Async runtime | tokio (full) | 1.44 |
| HTTP client | reqwest (stream, native-tls) | 0.13 |
| SSE raw streaming | hand-rolled `sse_parser` in `crates/api/src/lib.rs` | — |
| TUI | ratatui | 0.29 |
| Terminal backend | crossterm (event-stream) | 0.28 |
| MCP SDK | rmcp | 1.4.0 |
| Concurrency | parking_lot, dashmap | 0.12 / 6 |
| Storage | rusqlite (bundled) | 0.31 |

---

## Domain 1: Async SSE / Streaming LLM Responses

### Current state

`crates/api/src/lib.rs` contains a hand-rolled `sse_parser::SseLineParser` that
operates by calling `resp.bytes_stream()` on a `reqwest::Response`, splitting
incoming `Bytes` chunks on `'\n'`, and accumulating event/data fields. This
works for Anthropic. The `stream_parser.rs` file declares `SseStreamParser` and
`JsonLinesStreamParser` trait objects with stub bodies (deferred to Phase 2A).
Google and OpenAI providers implement their own inline SSE parsing using the same
`bytes_stream()` + manual split pattern.

### Recommendation: Do NOT add eventsource-stream or reqwest-eventsource

**Rationale:**

The hand-rolled parser is sufficient and already used in production for the
Anthropic provider. Adding `eventsource-stream` (v0.2.3) or `reqwest-eventsource`
(v0.6.0) would introduce a new abstraction on top of `reqwest::Response` that
conflicts with the existing `StreamParser` trait. The `SseStreamParser` stub in
`stream_parser.rs` is the right seam — implement the `StreamParser::parse()`
method for each provider using the same pattern already used in `lib.rs`.

**Pattern to follow (HIGH confidence — directly observed in codebase):**

```rust
// Inside StreamParser::parse() for a new provider:
let mut byte_stream = response.bytes_stream();
let mut parser = SseLineParser::new();   // already exists in lib.rs mod
let mut leftover = String::new();

while let Some(chunk) = byte_stream.next().await {
    // split on '\n', feed lines to parser, emit StreamEvents
}
```

The `tokio-util` codec `LinesCodec` could replace the manual split but adds no
correctness benefit for SSE (SSE frames are newline-delimited, not
length-prefixed) and the current approach already handles split-chunk boundaries
via the `leftover` string. Do not change it.

**For non-SSE providers (JSON lines, chunked JSON):** use the `JsonLinesStreamParser`
stub with the same `bytes_stream()` pattern but accumulate until valid JSON rather
than until blank line.

### What actually needs doing

- Implement `SseStreamParser::parse()` for Anthropic/Google (Phase 2A marker).
- The Minimax provider uses `AnthropicClient` with `use_bearer_auth: true` which
  already sends `Authorization: Bearer <key>`. Issue #117 (minimax Authorization
  header) appears already implemented; validate against the live API.
- Ollama provider already reads `OLLAMA_HOST` env var from
  `crates/api/src/providers/openai_compat_providers.rs`. Issue #86 (remote Ollama)
  is a bug in the Settings UI not forwarding `OLLAMA_HOST` to the provider
  factory, not a missing crate dependency.
- Custom OpenAI base URL (#106): `OPENAI_BASE_URL` env var is already wired in
  `api_base_env_var_for_provider()`. The bug is in the TUI settings screen
  (`settings_screen.rs`) not exposing an input field for it.

---

## Domain 2: Terminal UI Patterns

### Current state

`crates/tui/` already has:
- `virtual_list.rs` — a `VirtualList<T: VirtualItem>` with `scroll_offset` and
  `viewport_height`, rendering only items whose row ranges intersect the viewport.
  This is the correct pattern for the message pane.
- `app.rs` — full `handle_mouse_event()` implementing scroll, text selection,
  double/triple-click detection.
- All major dialogs exist as separate files.

ratatui 0.30.0 is available (current is 0.29). The 0.30 release added
`scrolling-regions` for flicker-free `insert_before`, better `Table` scrolling
APIs. **Do not upgrade yet** — ratatui is pinned via workspace dep and a minor
upgrade would need validation across all 12 crates.

### Mouse capture issue (#104)

`crates/tui/src/lib.rs` unconditionally calls `EnableMouseCapture` on startup.
This intercepts all mouse events at the terminal level, preventing the native OS
text selection from working. No new crate is needed — the fix is conditional
mouse capture: only enable `EnableMouseCapture` when the user has mouse mode
turned on (via a settings flag), and disable it when they turn it off. This is a
logic change in `lib.rs`, not a dependency change.

### Recommendation: Do NOT add tui-input or tui-textarea

`crates/tui/src/prompt_input.rs` and `crates/tui/src/input.rs` are hand-rolled.
`tui-textarea` (a popular third-party crate) would conflict with the existing
multi-line prompt input that already handles Vim-mode keybindings and history.
Adding it is more work than fixing the existing code.

### For missing UI components (agents view, settings parity)

The `agents_view.rs` file exists in `crates/tui/src/`. The pattern for new views
is established:
- State machine with a `Mode` enum (list / detail / edit / confirm-delete)
- `StatefulWidget` with `ListState` for navigable lists
- Wrap in an overlay using the existing `overlays.rs` machinery

No new TUI crates are needed. The `VirtualList<T>` type in `virtual_list.rs` is
the correct primitive for any view with unbounded items.

---

## Domain 3: MCP Sandbox / Permission Enforcement (Security Issue #123)

### Current state

`claurst-core` has a complete `permissions` module (inline in `lib.rs`) with:
- `PermissionLevel` (Read / Write / Execute / Network)
- `PermissionRule` with tool name + glob path pattern matching
- `PermissionManager` with session and persistent scopes
- `InteractivePermissionHandler` that sends a `PermissionRequest` over a
  `tokio::sync::oneshot` channel and awaits TUI approval

The security vulnerability (#123) is **not** a missing crate issue. The Claude
Code TypeScript spec allows MCP server configurations to be placed in
`.claude/mcp.json` inside any project directory. If claurst loads and executes
these without validating that the MCP server binary path is on an allowlist,
arbitrary code runs. The fix is policy enforcement in `crates/mcp/src/backend.rs`
/ `connection_manager.rs` before spawning child processes.

### What to add for OS-level sandboxing (optional, Linux-only)

If deeper sandboxing of MCP child processes is desired beyond the existing
permission rules, use:

| Crate | Version | What it does |
|-------|---------|--------------|
| `landlock` | 0.4.4 | Linux Landlock LSM — restricts filesystem access for a process without root. Requires Linux 5.13+. |

**Confidence: MEDIUM** — landlock is well maintained (landlock-lsm/rust-landlock,
the official Rust bindings). The API is stable. Applying Landlock to a spawned
MCP child before `exec` requires `nix::unistd::fork` + `exec` which the codebase
already uses via `portable-pty` patterns. On macOS this path does not exist
(Sandbox.kext / `sandbox_init()` requires entitlements). Gate behind
`#[cfg(target_os = "linux")]`.

**Do NOT add `seccompiler` (v0.5.0)** for the MCP security fix. Seccomp BPF
policy writing is complex, brittle across kernel versions, and the permission
issue is an allowlist problem (which binary paths can MCP configs reference), not
a syscall filtering problem. The existing `PermissionManager` + a path allowlist
in `McpServerConfig` is the right fix and requires zero new crates.

**The minimal correct fix for #123 (no new crates):**
1. Add an `allowed_mcp_server_paths: Vec<String>` field (glob patterns) to
   `Settings` in `claurst-core`.
2. In `crates/mcp/src/connection_manager.rs`, before spawning a child process,
   check the binary path against the allowlist.
3. Require explicit user confirmation (via the existing `PermissionManager`) when
   a project-level MCP config introduces a new server not in the global allowlist.

---

## Domain 4: Cross-Provider Model Routing

### Current state

The `ProviderRegistry` in `crates/api/src/registry.rs` holds named
`Arc<dyn LlmProvider>` instances. The routing logic in
`crates/query/` selects a provider by name from the registry. All providers
listed in `openai_compat_providers.rs` (20+ entries) follow the
`OpenAiCompatProvider` pattern with `ProviderQuirks` for per-provider
idiosyncrasies.

### Recommendation: No new crates needed for routing

The existing `LlmProvider` trait + `ProviderRegistry` pattern handles routing
correctly. The pattern for adding a new provider is established and documented:

1. Create `providers/myprovider.rs` implementing `LlmProvider`.
2. Register in `crates/api/src/providers/mod.rs` and `registry.rs`.
3. Expose env-var configuration in `api_base_env_var_for_provider()` and
   `api_key_env_vars_for_provider()` in `claurst-core/src/lib.rs`.

The `model_registry.rs` in `crates/api/` handles model aliases. No new crate
is needed for routing.

### For Managed Agents (plan.md)

The `AgentTool` stub in `crates/tools/src/agent_tool.rs` is the correct
insertion point. The existing `run_query_loop` in `crates/query/` is the
primitive. The plan.md approach (manager is a query loop with a delegation system
prompt) is correct and needs no new crates.

---

## Domain 5: Voice / ALSA (#88)

### Current state

`crates/tui/src/voice_capture.rs` and `crates/core/src/voice.rs` implement PTT
audio capture via `cpal` (feature-gated). The bug is that `cpal`'s default ALSA
host on Linux requires `libasound2-dev` at compile time and `libasound2` at
runtime. When the user's system lacks ALSA configured correctly, `cpal` returns
no input devices. The toggle "off" bug is a state management issue in `app.rs`.

**No new audio crate is needed.** `cpal` 0.15 is the correct choice — it
abstracts over ALSA (Linux), CoreAudio (macOS), WASAPI (Windows). The fix is:
1. Better error messages when `default_input_device()` returns `None` (partially
   done — error message mentions ALSA/PulseAudio).
2. Graceful voice toggle that checks `check_voice_availability()` before enabling
   the UI state.

**Transcription:** The codebase calls the OpenAI Whisper API (POST
`/v1/audio/transcriptions`) via reqwest. No Whisper crate needed; the API is
plain HTTP multipart. This is the correct approach.

**Do NOT add `whisper-rs` or Sherpa-ONNX.** Local ASR is explicitly out of scope
per PROJECT.md.

---

## Domain 6: YAML Frontmatter for Agent Files

### Current state

`crates/core/src/skill_discovery.rs` hand-parses YAML frontmatter by splitting
on `---` and parsing `key: value` lines manually. This works for the simple case
(name, description). The agent system (`.claude/agents/*.md` per spec) uses
richer YAML front matter including `tools`, `model`, `memory`, `effort` fields.

### Recommendation: Add `serde_yml` for agent file parsing

`serde_yaml` (v0.9.34) is deprecated. Its maintained successor is `serde_yml`
(v0.0.12, maintained by the dtolnay ecosystem, same API surface).

**Confidence: MEDIUM** — serde_yml is young (0.0.x) but tracks serde_yaml API
closely. The alternative is keeping hand-rolled front-matter parsing, which is
correct for simple key-value pairs but will become fragile as agent YAML includes
nested structures (tool lists, permission modes).

**Only add this if implementing the full Agents subsystem from spec/05.** For
simple `name:` / `description:` parsing, the existing hand-roller in
`skill_discovery.rs` is sufficient.

---

## Gaps Summary: What to Add vs What to Leave

### Add (with justification)

| Crate | Version | Crate for | Confidence | Condition |
|-------|---------|-----------|------------|-----------|
| `landlock` | 0.4.4 | MCP child process filesystem restriction | MEDIUM | Linux only, only if deeper MCP sandbox desired beyond allowlist fix |
| `serde_yml` | 0.0.12 | Agent YAML frontmatter (spec/05 agents subsystem) | MEDIUM | Only when implementing full agents CRUD from spec |

### Do NOT Add

| Crate | Reason |
|-------|--------|
| `eventsource-stream` / `reqwest-eventsource` | Hand-rolled SSE parser in lib.rs is sufficient; adding this creates conflicting abstraction layers |
| `tui-textarea` | Conflicts with existing hand-rolled prompt_input.rs; more work to integrate than to fix |
| `tui-input` | Same reason as tui-textarea |
| `seccompiler` | Wrong tool for the MCP security problem; the fix is a path allowlist, not syscall filtering |
| `async-openai` | Would duplicate all of crates/api; the LlmProvider trait is the abstraction layer |
| `whisper-rs` / `sherpa-onnx` | Explicitly out of scope per PROJECT.md |
| `serde_yaml` | Deprecated as of v0.9.34+; use serde_yml if YAML is needed |
| `tower-lsp` / `lsp-types` | claurst-core already has a hand-rolled LSP client over JSON-RPC; adding tower-lsp would require a complete rewrite of lsp.rs for no behavioral gain |
| `keyring` | Auth is stored in `~/.claurst/auth.json` (plaintext JSON). A keyring would improve security but requires platform-specific library linking (libsecret on Linux) and is not required for parity |

---

## Pattern Recommendations

### Pattern 1: Per-Provider SSE Implementation (HIGH confidence)

Each provider's `create_message_stream()` should follow the existing `AnthropicClient::process_sse_stream()` pattern:

```rust
let (tx, rx) = tokio::sync::mpsc::channel(256);
tokio::spawn(async move {
    let mut byte_stream = response.bytes_stream();
    let mut leftover = String::new();
    while let Some(chunk) = byte_stream.next().await {
        // parse SSE frames, send StreamEvent via tx
    }
});
Ok(rx)
```

The `SseStreamParser` struct in `stream_parser.rs` should be filled in using this
pattern — it already has the right trait signature.

### Pattern 2: MCP Security Fix Without New Crates (HIGH confidence)

The correct fix for #123 is enforcement in the existing permission stack, not OS
sandboxing:

```
McpServerConfig loaded from project .claude/mcp.json
  → check server.command against allowlist in Settings.allowed_mcp_commands
  → if not in allowlist: send PermissionRequest to TUI
  → TUI shows bypass_permissions_dialog.rs (already exists)
  → User approves → add to session allowlist (PermissionScope::Session)
```

No new crates. The entire permission infrastructure already exists.

### Pattern 3: Provider Registry Extension for Custom Base URL (HIGH confidence)

Issue #106 (custom OpenAI API URL) is a TUI settings gap, not a missing crate.
The env var `OPENAI_BASE_URL` is already read by `OpenAiCompatProvider`. The fix
is adding an input field in `settings_screen.rs` that writes to a
`provider_base_urls: HashMap<String, String>` field in `Settings`, which then
gets forwarded when constructing providers in the registry.

### Pattern 4: Ratatui Upgrade Path (MEDIUM confidence)

ratatui 0.30.0 is available. The 0.29→0.30 changelog shows mostly additive
changes (new widget methods, scrolling-regions stabilization). **Defer the
upgrade** until a phase that touches the TUI heavily — then upgrade in one step
and run the existing TUI test suite. The only behavioral change that matters is
`scrolling-regions` for `insert_before`, which would fix potential flicker in the
message pane, but it requires opt-in via a crate feature flag.

---

## Confidence Assessment

| Area | Confidence | Reasoning |
|------|------------|-----------|
| SSE streaming patterns | HIGH | Directly read the implementation; hand-rolled parser is proven |
| MCP permission fix approach | HIGH | Permission infrastructure fully exists; issue is application logic |
| Ollama/minimax/custom-URL bugs | HIGH | Root causes confirmed in source; no crate changes needed |
| landlock for MCP sandbox | MEDIUM | Crate is well-maintained official bindings; Linux-only constraint limits scope |
| serde_yml for agent YAML | MEDIUM | serde_yml is young (0.0.x); API stable but not battle-tested at scale |
| ratatui 0.30 upgrade | MEDIUM | API is mostly additive; risk is in the 12 crates that use ratatui |
| Voice/ALSA fix | HIGH | Bug is in configuration/state management, not audio stack |

---

## Sources

- Codebase: `/Users/thamw/development/local/claurst/src-rust/crates/` (directly read)
- [eventsource-stream on crates.io](https://crates.io/crates/eventsource-stream) — v0.2.3
- [reqwest-eventsource on crates.io](https://crates.io/crates/reqwest-eventsource) — v0.6.0
- [landlock on crates.io](https://crates.io/crates/landlock/0.4.1) — v0.4.4
- [rust-landlock GitHub](https://github.com/landlock-lsm/rust-landlock) — official bindings
- [serde_yml on crates.io](https://crates.io/crates/serde_yml) — v0.0.12
- [ratatui v0.29 highlights](https://ratatui.rs/highlights/v029/)
- [ratatui on crates.io](https://crates.io/crates/ratatui) — v0.30.0 available
- [seccompiler on crates.io](https://crates.io/crates/seccompiler) — v0.5.0
- [How to Run Rust Binaries Without Root Using Sandboxing](https://oneuptime.com/blog/post/2026-01-07-rust-sandboxing-seccomp-landlock/view)
- ratatui Context7 docs (`/ratatui/ratatui`) — StatefulWidget / VirtualList patterns
- rmcp Context7 docs (`/websites/rs_rmcp_rmcp`) — MCP SDK patterns

---

*Research date: 2026-05-04*
