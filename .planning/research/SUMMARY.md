# Project Research Summary

**Project:** claurst — Rust rewrite of Claude Code CLI
**Domain:** Agentic developer tool (compiled TUI binary, multi-provider LLM, MCP client)
**Researched:** 2026-05-04
**Confidence:** HIGH

## Executive Summary

claurst is a mature 12-crate Rust workspace that has already shipped the hard infrastructure: a streaming agentic query loop, ratatui TUI, MCP client, plugin system, ACP/bridge protocol, and SQLite session storage. The core architecture is sound and needs no structural reinvention. The milestone work is correctness and parity: close 6 open bugs (including one critical security vulnerability), refactor two monolithic files that block safe parallel development, then layer feature parity and Managed Agents on a stable foundation.

The recommended build order is: security hardening first (issue #123 is an active arbitrary-execution vector — every day it ships unfixed is unacceptable), bug fixes next, structural refactors that reduce upstream merge-conflict risk, feature parity work (slash commands, tools, TUI quality), and finally Managed Agents. This order is non-negotiable: the security issue gates trust, the monolith split gates conflict-free parallel development, and the bug fixes gate daily usability. The stack is essentially frozen — only two new crates (serde_yml for agent YAML, landlock for optional Linux MCP sandboxing) should ever be added, and only when those specific subsystems are built.

The principal risks are: (1) the 8,576-line commands/src/lib.rs monolith creating upstream merge conflicts that accelerate divergence; (2) the MCP security gap enabling supply-chain attacks until fixed; (3) async translation bugs silently making concurrent TypeScript features sequential in Rust; and (4) ~410 production unwrap() calls and std::sync::Mutex poisoning creating cascading TUI crashes. All four risks have concrete, confirmed mitigations documented in the research.

## Key Findings

### Recommended Stack

The existing dependency set is well-chosen and should not change. The codebase already uses tokio 1.44, ratatui 0.29, crossterm 0.28, reqwest 0.13, rmcp 1.4.0, rusqlite 0.31 (bundled), parking_lot, and dashmap. The SSE streaming layer, TUI input handling, and MCP transport are all hand-rolled to the correct level of abstraction — adding third-party crates for any of these would create conflicting abstractions, not improvements.

**Add only conditionally:**
- `serde_yml` 0.0.12: Agent YAML frontmatter — only when implementing the full agents subsystem from spec/05. The existing hand-rolled parser in `skill_discovery.rs` is sufficient until then.
- `landlock` 0.4.4: Linux MCP child process filesystem restriction — Linux-only, optional, only if deeper OS-level sandboxing beyond the allowlist fix is desired.

**Do not add:** eventsource-stream, reqwest-eventsource, tui-textarea, tui-input, seccompiler, async-openai, whisper-rs, sherpa-onnx, serde_yaml (deprecated), tower-lsp, keyring.

**Defer ratatui 0.30 upgrade** until a phase that touches the TUI heavily — upgrade in one step across all 12 crates, validate fully.

### Expected Features

**Must fix first — security/correctness (gates everything else):**
- Issue #123: MCP arbitrary execution via project-level config — add `McpServerScope` + `McpTrustStore` in claurst-core, trust gate in claurst-cli before server spawn
- Issues #79/#96: Path containment bypass for pre-creation paths — canonicalize parent dir, append filename
- `auth.json` world-readable credentials — one-line `set_permissions(0o600)` fix in `auth_store.rs`
- OAuth CSRF: missing state validation in `oauth_flow.rs` — verify or add `state == original_state` check before token exchange
- SSRF via `WebFetchTool` — pre-resolve hostname, reject RFC 1918/loopback/link-local IPs
- Web response OOM: stream with chunk loop and byte-counter cap before truncation

**Must fix next — high-severity bugs (breaks daily use for large user segments):**
- Issue #86: Ollama remote URL not respected — `api_base` not wired from `Config::resolve_api_base()` to `ClientConfig` at provider construction
- Issue #106: Custom OpenAI base URL not surfaced in TUI — `OPENAI_BASE_URL` already wired in env; gap is a missing input field in `settings_screen.rs`
- Issue #117: Minimax `Authorization: Bearer` header — add `auth_header_style: AuthHeaderStyle` to `ClientConfig`
- Issue #104: `EnableMouseCapture` unconditionally set — make opt-in via `mouse_capture: bool` in settings; fallback keyboard scroll
- Issue #76: API key paste drops characters — use crossterm `Paste` (bracketed paste) event
- Issue #47: Non-QWERTY keyboard layouts break shortcuts — configurable key bindings via `settings.json`
- Issue #88: Voice/ALSA not connecting — fix device enumeration error path and toggle state management

**Table-stakes TUI gaps (interactive correctness is broken without these):**
- `AskUserQuestion` tool never renders a dialog — sessions hang silently
- `AutoPermissionHandler` used in all modes — interactive permission prompts never fire; security regression vs TypeScript
- Input cursor/multi-line input — no left/right navigation, no selection, no Shift+Enter newline

**Table-stakes slash commands (most frequently invoked by Claude Code users):**
`/add-dir`, `/context`, `/copy`, `/usage`, `/keybindings`, `/vim`, `/voice`, `/color`, `/theme`, `/upgrade`, `/terminal-setup`

**High-value slash commands:**
`/commit-push-pr`, `/ultraplan`, `/ultrareview`, `/pr-comments`, `/branch`+`/fork`, `/output-style`, `/effort` (needs thinking-budget wiring)

**Missing tools by priority:**
- High: `McpAuthTool` (OAuth for authenticated MCP servers), `TeamCreateTool` + `TeamDeleteTool` (Managed Agents prerequisite), `LSPTool`
- Medium: `SyntheticOutputTool`, `REPLTool`

**TUI quality improvements (biggest perceived jump for zero functional change):**
- Syntax highlighting in code blocks (syntect or bat)
- Markdown rendering (bold, italic, headers, lists in ratatui)
- Inline diff view for Edit tool results
- Tool output collapsing with Show more affordance

**Defer to later:**
- LSPTool, REPLTool — useful but complex; not daily-use blockers
- 60+ low-traffic stub commands — implement as not-yet-supported stubs, not silent failures
- Managed Agents full implementation — depends on TeamCreate/Delete and clean core boundary
- Hardcoded model registry to hosted JSON manifest with 24h cache

**Explicit anti-features (never implement):**
- Local ASR (Sherpa-ONNX, whisper-rs) — out of scope per PROJECT.md
- GUI (Electron/Tauri) — reject PRs
- Auto-updating without user consent
- Telemetry — document absence as a differentiator
- Node.js shim / TypeScript interop

### Architecture Approach

The 12-crate dependency graph is a strict DAG with claurst-core at the foundation and claurst-cli at the apex. Every placement decision must respect it: new slash commands go in claurst-commands (not claurst-tui); new tools go in claurst-tools; shared types that multiple crates need go in claurst-core. The current monoliths — commands/src/lib.rs (8,576 lines), tui/src/app.rs (5,918 lines), core/src/lib.rs (4,246 lines) — are the primary merge-conflict surface with upstream and must be split into per-command and per-module files before new features are added.

**Major components and their change boundaries:**

1. `claurst-core` — data model foundation; gains `McpServerScope`, `McpTrustStore`, `ManagedAgentConfig`, sub-module split (`config.rs`, `permissions.rs`, `session.rs`). All new `Config`/`Settings` fields must use `#[serde(default)]`.
2. `claurst-commands` — slash command registry; split into `commands/src/commands/<name>.rs` per command, then add new commands without conflicts. `all_commands()` remains the single registration point.
3. `claurst-tools` — tool implementations; one file per tool, registered in `ToolRegistry`. `ToolContext` is the only injection point — add `managed_agent_config: Option<ManagedAgentConfig>` for Managed Agents.
4. `claurst-tui` — UI rendering only; dispatches to `claurst-commands` via `CommandResult` variants; never defines commands. Overlay triggers use `CommandResult` => `tui/src/app.rs` match.
5. `claurst-cli` — binary entry point; trust gate for project-scoped MCP servers before `McpToolWrapper` construction.
6. `claurst-query` — new `managed_orchestrator.rs` file for Managed Agents; `run_query_loop` gains a startup branch if managed config is active.

**Key abstractions to never break:** `Tool` trait signature, `SlashCommand` trait, `QueryEvent` enum, `LlmProvider` trait (no new required methods), `ContentBlock` enum, `Config`/`Settings` serde shapes.

### Critical Pitfalls

1. **MCP project-config enables arbitrary command execution (#123)** — Never auto-trust project-level MCP servers. Add `McpServerScope::Project` to `McpServerConfig`, gate connection behind `McpTrustStore::is_trusted()`, show approval dialog. Never bypass for `--yes` / non-interactive mode. Highest-priority fix.

2. **Monolith merge conflicts will kill upstream sync cadence** — `commands/src/lib.rs` at 8,576 lines causes multi-hundred-conflict merges on every upstream sync. Split into per-command files (pure refactor, no behavior change) before taking the next upstream.

3. **TypeScript async concurrency silently becomes sequential Rust** — `Promise.all([a(), b()])` maps to `tokio::join!(a(), b())`, not sequential `.await` chains. During every tool/command translation, audit each `async` function for concurrency intent.

4. **`unwrap()` + `std::sync::Mutex` cascade panics** — ~410 production `unwrap()` calls; a panic poisons `std::sync::Mutex` locks, and downstream `unwrap()` on the poisoned lock takes down the TUI render loop. Migrate to `parking_lot::Mutex` (already a workspace dep, never poisons).

5. **Path canonicalization fails for pre-creation paths** — `path_is_within_workspace` uses `canonicalize` which errors for files that do not exist yet; the `unwrap_or_else(|_| path.to_path_buf())` fallback skips the containment check. Fix: canonicalize parent dir, append filename.

6. **Credentials file is world-readable** — `auth.json` inherits process umask; API keys visible to all co-tenant processes. One-line fix: `set_permissions(0o600)` after write.

7. **`select!` cancels in-flight work non-deterministically** — `tokio::select!` drops non-winning futures at their `.await` point, silently dropping streaming tokens or key events. Prefer separate Tokio tasks communicating via channels; audit every `select!` in `tui/src/app.rs` for cancel safety before touching the event loop.

## Implications for Roadmap

### Phase 1: Security Hardening

**Rationale:** Issue #123 is an active attack surface — any project a user opens can silently execute OS commands. Credentials are world-readable. Path containment is bypassable for new files. None of these are acceptable for a daily-use tool. Security cannot wait for feature work.

**Delivers:** A claurst that users can safely point at untrusted repositories without risking code execution or credential theft.

**Addresses:**
- Issue #123: `McpServerScope` + `McpTrustStore` in core, trust gate in cli, approval dialog in tui
- `auth.json` permissions: `set_permissions(0o600)` in `auth_store.rs`
- Path containment bypass: canonicalize parent + filename in `path_is_within_workspace`
- SSRF in `WebFetchTool`: pre-resolve hostname, reject RFC 1918 ranges via `ipnet`
- Web fetch OOM: stream with byte-counter cap before truncation
- OAuth CSRF: verify state parameter in `oauth_flow.rs`

**Avoids:** Pitfalls 1, 2, 3, 13, 14

**No new crates needed.** The fix is enforcement logic in existing infrastructure.

---

### Phase 2: Structural Refactors

**Rationale:** `commands/src/lib.rs` (8,576 lines) and `core/src/lib.rs` (4,246 lines) are the primary upstream merge-conflict surface. Splitting them before adding new commands or before the next upstream sync reduces future merge effort from hours to minutes. This is a pure refactor — no behavior change, test suite must pass identically before and after.

**Delivers:** A codebase where adding a new slash command is one new file in `commands/src/commands/`, with zero merge-conflict risk against upstream. Enables parallel development of Phases 3 and 4.

**Addresses:**
- Split `crates/commands/src/lib.rs` into per-command modules under `commands/src/commands/`
- Split `crates/core/src/lib.rs` into `config.rs`, `permissions.rs`, `session.rs`, `auth_store.rs`
- Establish `upstream-merge` branch pattern: never merge upstream directly to `main`
- Add per-feature CI matrix during this phase

**Avoids:** Pitfalls 9, 10

**Research flag:** Standard Rust module refactor — no research phase needed.

---

### Phase 3: Bug Fixes

**Rationale:** Six open bugs break daily use for major user segments: remote API users (#86, #106, #117), all interactive users (#104, #76), non-QWERTY users (#47), and voice users (#88). Issues #86, #106, #117 share the same root cause (`api_base` not propagated) and should be fixed together. `AskUserQuestion` and permission dialog wiring are correctness failures that make the interactive permission model broken by design.

**Delivers:** A claurst where API key configuration works, remote providers connect, the input field is usable, and the permission model actually prompts users.

**Addresses:**
- #86 + #106: Wire `api_base` from `Config::resolve_api_base()` to `ClientConfig` at provider construction
- #117: Add `auth_header_style: AuthHeaderStyle` to `ClientConfig`; select by provider
- #104: Make `EnableMouseCapture` opt-in via `mouse_capture: bool` in settings; fallback keyboard scroll
- #76: Use crossterm `Paste` event (bracketed paste) for input fields
- #47: Configurable key bindings map in `settings.json`; semantic `KeyCode::Char` matching
- #88: Fix ALSA device enumeration error path; graceful voice toggle
- `AskUserQuestion` TUI wiring: render dialog when `ask_user` event received
- Permission dialog wiring: replace `AutoPermissionHandler` with interactive handler in default mode

**Avoids:** Pitfalls 5, 7, 8

**Research flag:** All root causes confirmed in codebase — no research phase needed.

---

### Phase 4: TUI Quality and Input Polish

**Rationale:** The biggest perceived quality jump for zero functional change is rendering improvement — syntax-highlighted code, markdown formatting, and inline diffs make code-heavy responses dramatically more readable. Input quality (cursor navigation, multi-line, selection) is a prerequisite for the slash command expansion phase.

**Delivers:** A TUI that matches user expectations for a professional terminal tool: readable code blocks, navigable input, copyable output.

**Addresses:**
- Syntax highlighting in code blocks (evaluate `syntect` at phase start)
- Markdown rendering in ratatui
- Inline diff view for `Edit` tool results
- Tool output collapsing with Show more affordance
- Input cursor left/right navigation, selection, multi-line Shift+Enter
- Session list dialog for `/resume` command
- Token usage indicator in status bar
- ratatui 0.29 -> 0.30 upgrade (evaluate at phase start; upgrade in one step, validate all 12 crates)

**Avoids:** Pitfall 8 (select! cancellation — TUI event loop changes require cancel-safety audit)

**Research flag:** `syntect` + ratatui integration path needs brief evaluation before committing.

---

### Phase 5: Feature Parity — Slash Commands and Tools

**Rationale:** After the monolith split (Phase 2), adding slash commands is one file per command with no merge risk. This phase delivers the most frequently-invoked missing commands and the tool gaps that block real workflows.

**Delivers:** The most-used missing slash commands and the tool gaps blocking real workflows. claurst becomes viable as a daily driver for Claude Code users who relied on these commands.

**Addresses:**
- High-traffic missing commands: `/add-dir`, `/context`, `/copy`, `/usage`, `/keybindings`, `/vim`, `/voice`, `/color`, `/theme`, `/upgrade`, `/terminal-setup`
- High-value chaining commands: `/commit-push-pr`, `/ultraplan`, `/ultrareview`, `/pr-comments`, `/branch`+`/fork`
- `/effort` thinking-budget wiring to API
- `McpAuthTool` (OAuth for authenticated MCP servers)
- `TeamCreateTool` + `TeamDeleteTool` (Managed Agents prerequisite)
- Placeholder stubs for 60+ low-traffic/internal commands (print not yet supported)
- `SyntheticOutputTool`, `REPLTool` (medium priority, can defer)

**Avoids:** Pitfall 7 (TypeScript to Rust async concurrency — audit each ported command for Promise.all equivalents)

**Research flag:** `McpAuthTool` OAuth flow needs a brief research phase — MCP OAuth implementation patterns are sparse.

---

### Phase 6: Managed Agents

**Rationale:** Managed Agents is the primary claurst-specific differentiator vs the TypeScript upstream. It requires `TeamCreateTool` and `TeamDeleteTool` (Phase 5), the core sub-module split (Phase 2), and clean `ToolContext` boundaries. The plan.md architecture is correct and requires no new crates.

**Delivers:** `/managed-agents` slash command, manager-executor architecture, budget splitting policy, agent role display in TUI, per-session cost breakdown.

**Addresses (plan.md phases):**
- Phase 1 (Config): `ManagedAgentConfig`, `BudgetSplitPolicy`, `ManagedAgentPreset` in core
- Phase 2 (Command): `ManagedAgentsCommand` in commands
- Phase 3 (Orchestrator): `managed_orchestrator.rs` in query; `AgentTool` reads `ToolContext.managed_agent_config`
- Phase 4 (TUI): Extend `agents_view.rs` with `AgentRole` enum and cost breakdown
- Phase 5 (Sessions): `agent_role` and `managed_session_id` fields (Option<T>, skip_serializing_if — no migration)
- Phase 6 (Tests): Integration tests for manager-executor delegation

**Avoids:** Pitfall 10 (add `managed_agents` feature to `dev_full` from the start)

**Research flag:** `CommandContext` -> `AuthStore` access pattern for `/managed-agents` setup needs a design decision before this phase begins.

---

### Phase 7: Upstream Sync Cadence and CI Hardening

**Rationale:** After Phases 1-6, the codebase has a clean module structure, security baseline, and full feature set. Establishing a regular monthly upstream sync cadence prevents the accumulated debt that made this milestone necessary.

**Delivers:** Sustainable maintenance: monthly upstream syncs that take hours not days, CI that catches feature-flag regressions, no `std::env::set_var` UB in the test suite.

**Addresses:**
- `upstream-merge` branch pattern (never merge upstream directly to `main`)
- Per-feature CI jobs for each non-default Cargo feature
- Replace `std::env::set_var` in async tests with `#[serial]` or `Config` injection
- `cargo deny` check for duplicate crossterm major versions
- Migrate `std::sync::Mutex` to `parking_lot::Mutex` (mechanical sweep)
- Replace `panic!` in production match arms with `Err(...)` returns

**Avoids:** Pitfalls 6, 9, 10, 11, 15

**Research flag:** No research phase needed — all patterns are established.

---

### Phase Ordering Rationale

- **Security first (Phase 1):** #123 is an active attack vector; credentials are world-readable. No feature work ships until these are closed.
- **Refactor before features (Phase 2):** The monolith split gates conflict-free parallel feature development. Done after security so the codebase is stable for the refactor.
- **Bug fixes in Phase 3:** Daily-use frustrations but not security crises. Some bug fixes (mouse capture, permission dialog wiring) overlap with TUI work.
- **TUI quality before slash command expansion (Phase 4 before Phase 5):** Input navigation and syntax highlighting are prerequisites for usable display of command output.
- **Managed Agents last (Phase 6):** Has the most prerequisites (TeamCreate/Delete tools, core split, clean ToolContext). Building it last minimizes cascading conflict risk.

### Research Flags

Phases needing `/gsd-research-phase` during planning:
- **Phase 6 (Managed Agents):** `CommandContext` -> `AuthStore` access pattern is underspecified; cross-provider budget validation needs a design decision.
- **Phase 4 (TUI Quality):** `syntect` + ratatui integration path needs brief evaluation before committing.
- **Phase 5 (McpAuthTool):** MCP OAuth spec implementation patterns are sparse; targeted research before implementation.

Phases with well-established patterns (skip research):
- **Phase 1 (Security):** All root causes confirmed; fix approaches fully specified in ARCHITECTURE.md and PITFALLS.md.
- **Phase 2 (Refactors):** Standard Rust module split; no domain knowledge needed.
- **Phase 3 (Bug Fixes):** All root causes confirmed in codebase; no external API uncertainty.
- **Phase 7 (CI Hardening):** Standard CI patterns.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All findings from direct codebase inspection; existing stack is well-chosen; only 2 optional additions recommended |
| Features | HIGH | Derived from spec/13_rust_codebase.md (authoritative inventory), spec/02 and spec/03 (parity targets), and confirmed open issues |
| Architecture | HIGH | All findings from direct inspection of crate boundaries, dependency graph, and monolith sizes |
| Pitfalls | HIGH | Security pitfalls corroborated by CVE-2025-53109/53110; async pitfalls from authoritative Tokio docs; remaining from CONCERNS.md audit |

**Overall confidence: HIGH**

### Gaps to Address

- **`CommandContext` / `AuthStore` coupling for Managed Agents:** During `/managed-agents setup`, the command needs to validate auth credentials for both manager and executor providers simultaneously. `CommandContext` currently carries `config: Config` but not a live `AuthStore` handle. Resolution: thread `AuthStore` through `CommandContext`, or read from the config `api_key` field (already merged in). Needs a design decision before Phase 6.

- **`serde_yml` maturity:** serde_yml is at 0.0.x. If agent YAML frontmatter needs complex nested structures beyond what the hand-rolled parser handles, evaluate serde_yml at that point but be prepared for API instability. Validate in a prototype before committing.

- **ratatui 0.30 upgrade timing:** 0.30 adds `scrolling-regions` for flicker-free `insert_before` in the message pane. Deferred to Phase 4 — validate across all 12 crates before merging.

- **#86 / #106 live API validation:** STACK.md notes that `OPENAI_BASE_URL` is already wired in `api_base_env_var_for_provider()` and Minimax `use_bearer_auth: true` appears already implemented. Validate against live APIs before closing issues — root causes may differ in the current upstream state.

## Sources

### Primary (HIGH confidence)
- `/Users/thamw/development/local/clearest-rust/crates/` — direct codebase inspection
- `spec/13_rust_codebase.md` — authoritative Rust implementation inventory
- `spec/02_commands.md` + `spec/03_tools.md` — TypeScript parity targets
- `.planning/codebase/CONCERNS.md` — security and reliability audit (2026-05-04)
- EscapeRoute CVE-2025-53109/53110 (cymulate.com) — MCP sandbox escape confirmation
- Tokio select! cancellation safety docs (tokio.rs) — async pitfall validation
- Ratatui mouse capture docs (ratatui.rs) — mouse pitfall validation

### Secondary (MEDIUM confidence)
- `landlock` on crates.io (v0.4.4) — Linux sandboxing option
- `serde_yml` on crates.io (v0.0.12) — agent YAML option; young crate
- ratatui v0.29/0.30 highlights (ratatui.rs) — upgrade path assessment
- MCP STDIO 200,000 servers exposure (venturebeat.com) — threat context

### Tertiary (LOW confidence)
- Migrating TypeScript to Rust (corrode.dev) — async pitfall framing

---
*Research completed: 2026-05-04*
*Ready for roadmap: yes*
