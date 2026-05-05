# Feature Landscape: claurst Parity with Claude Code

**Domain:** Rust reimplementation of Claude Code CLI
**Researched:** 2026-05-04
**Confidence:** HIGH — derived directly from spec/13_rust_codebase.md (Rust ground truth) and spec/02_commands.md + spec/03_tools.md (TypeScript parity targets)

---

## Summary of Current State

claurst implements 33 of 40+ TypeScript tools, 33 of 100+ slash commands, and has all foundational
infrastructure (query loop, TUI, MCP, bridge, ACP, cron). The gap is primarily:

1. **Security**: one critical unfixed vulnerability (MCP arbitrary execution, #123)
2. **Bug fixes**: 6 open issues blocking daily use (mouse, voice, Ollama, keyboard, API key paste, custom URL)
3. **Slash commands**: ~67 TypeScript commands not yet ported
4. **Tools**: 7 TypeScript tools not yet ported, with varying priority
5. **TUI gaps**: permission dialog, AskUserQuestion interactivity, rich message rendering

---

## Bug Fixes by Severity

### Critical (Security)

| Issue | Description | Why Critical | Fix Approach |
|-------|-------------|--------------|--------------|
| #123 MCP arbitrary execution | Project-level `.claude/mcp.json` can inject arbitrary shell commands without user confirmation | Attackers who control a repo can exfiltrate secrets or compromise the machine silently | Sandbox project-level MCP configs behind an explicit user-approval flow; never auto-trust project MCP configs; require `--allow-project-mcp` flag or per-server allowlist in `settings.json` |

This must be fixed before any new feature work. Every day it ships unfixed is a supply-chain-style attack surface.

### High (Correctness — Breaks Core Workflow)

| Issue | Description | Impact | Fix Approach |
|-------|-------------|--------|--------------|
| #86 Ollama remote not respected | `config.api_base` / `ANTHROPIC_BASE_URL` is read but `AnthropicClient` ignores it for Ollama provider; still hits `localhost:11434` | Users cannot point to remote Ollama; breaks containerised setups | Ensure `ClientConfig::api_base` is wired from `Config::resolve_api_base()` at client construction; add `provider_type` enum branch in `AnthropicClient` |
| #106 Custom OpenAI URL ignored | Same class of bug as #86 — custom `api_base` for OpenAI-compat providers (Azure, Together, etc.) not passed through | Blocks all Azure OpenAI and OpenAI-compat provider users | Same fix path as #86; `api_base` must propagate to `ClientConfig` |
| #117 Minimax Authorization header | Minimax API requires `Authorization: Bearer <key>` not `x-api-key`; current client always uses `x-api-key` | Breaks Minimax provider entirely | Add `auth_header_style: AuthHeaderStyle` field to `ClientConfig` enum (`XApiKey` vs `Bearer`); select by provider |

### Medium (UX — Breaks Interactive Use)

| Issue | Description | Impact | Fix Approach |
|-------|-------------|--------|--------------|
| #104 Mouse capture | `crossterm` raw mode captures all mouse events, breaking native terminal text selection and causing scroll lag | Daily frustration for anyone who selects+copies output | Disable `MouseCapture` crossterm event unless explicitly opted in; add `--enable-mouse` flag and `settings.json` toggle |
| #76 API key paste failure | Pasting into API key input field in TUI drops characters | New users can't authenticate via TUI | Use crossterm `Paste` event (bracketed paste) rather than char-by-char `KeyCode::Char` accumulation for password fields |
| #47 Non-English keyboard layouts | Modifier key combinations on non-QWERTY layouts produce wrong `KeyCode` values | Breaks keyboard shortcuts for significant user segment | Map logical action names to keys in `settings.json`; use `KeyCode::Char` semantic matching with configurable bindings rather than hardcoded physical keys |
| #88 Voice / ALSA not connecting | Voice input feature fails to connect to ALSA backend; toggle broken | Voice input unusable | Fix ALSA device enumeration and error propagation; ensure the voice feature-flag path compiles on Linux without ALSA; add graceful degradation message |

---

## Table Stakes Features (Missing = Claurst Unusable for Claude Code Users)

These are features Claude Code users expect to exist on day one. Their absence causes users to fall back to the TypeScript implementation.

### Missing Slash Commands — Must Have

The Rust codebase has 33 of 100+ commands. The following are the commands users invoke most frequently in real Claude Code sessions:

| Command | TypeScript Behavior | Priority Rationale |
|---------|---------------------|-------------------|
| `/add-dir <path>` | Adds a directory to the active working context; model can see and edit files in it | Power users with monorepos run this constantly |
| `/agents` | Lists active sub-agents and their status | Required once Managed Agents ships |
| `/branch` / `/fork` | Creates a git branch/worktree for the current task | Common agentic workflow starting point |
| `/color` | Toggles ANSI color output on/off | Needed for CI/pipe usage |
| `/context` | Shows what's currently in the model's context window (files, memories, size) | Core diagnostic; users hit context limits often |
| `/copy` | Copies last assistant response to clipboard | Used constantly |
| `/effort <level>` | Sets thinking budget (quick/normal/thorough) | Already in Rust as `EffortCommand` but needs thinking-budget wiring to API |
| `/feedback` / `/bug` | Opens browser to feedback form | Already partially implemented as `BugCommand` |
| `/hooks` | Shows/edits pre/post tool hooks | Already implemented; verify completeness |
| `/ide` | Configures editor integration (bridge/ACP) | Required for VS Code / Cursor extension users |
| `/keybindings` | Shows/edits keyboard shortcut map | Critical given #47 keyboard layout bug |
| `/output-style` | Switches between verbose/concise response style | Frequently used |
| `/permissions` / `/allowed-tools` | Shows current tool permission rules; lets user edit allowlist | Already implemented; verify it surfaces all rules |
| `/pr-comments` | Pulls GitHub PR review comments into context and asks model to address them | High-value agentic workflow |
| `/release-notes` | Shows what changed in latest claurst/Claude Code update | Trust-building for users tracking upstream |
| `/terminal-setup` | Installs shell completions, sets PATH | Onboarding essential |
| `/theme` | Sets color theme | Basic UX |
| `/upgrade` | Self-updates claurst binary | Distribution essential for non-package-manager installs |
| `/usage` | Shows token usage breakdown for current session | Budget-conscious users check this constantly |
| `/vim` | Toggles vim keybindings in input | Vim users cannot operate without this |
| `/voice` | Enables/disables voice input | Already broken (#88); command must exist even if feature is degraded |

### Missing Slash Commands — High Value

| Command | Notes |
|---------|-------|
| `/commit` | Already implemented as `CommitCommand`; verify git workflow completeness |
| `/commit-push-pr` | Chains commit + push + GitHub PR creation; saves 3 manual steps |
| `/ultraplan` | Extended planning mode with deeper research; prompt-expansion command |
| `/ultrareview` | Extended code review; prompt-expansion command |
| `/think-back` | Replays model's internal reasoning trace |
| `/review` | Already implemented; verify it handles PR diff context |
| `/sandbox` | Runs commands inside a sandboxed environment |
| `/security-review` | Specialized security audit prompt |
| `/tasks` / `/bashes` | Background task management; already partially in `TasksCommand` |

### Missing Slash Commands — Lower Priority (Stubs or Internal)

These exist in TypeScript as stubs, internal tooling, or features with niche audiences. Implement with a placeholder that prints "not yet supported" rather than silently failing.

`ant-trace`, `autofix-pr`, `backfill-sessions`, `break-cache`, `bughunter`, `ctx_viz`, `debug-tool-call`, `env`, `extra-usage`, `good-claude`, `heapdump`, `install-github-app`, `install-slack-app`, `insights`, `issue`, `mock-limits`, `oauth-refresh`, `onboarding`, `perf-issue`, `passes`, `rate-limit-options`, `reset-limits`, `share`, `stats` (verify), `stickers`, `summary` (verify), `tag`, `teleport`, `thinkback-play`, `init-verifiers`, `mobile`/`ios`/`android`, `remote-env`, `remote-setup`/`web-setup`, `statusline`, `desktop`/`app`, `chrome`, `install` (component), `reload-plugins`, `privacy-settings`

---

## Missing Tools by Priority

### High Priority — Blocks Real Workflows

| Tool | TypeScript Name | Gap | Why Needed |
|------|----------------|-----|------------|
| `McpAuthTool` | `"McpAuth"` | Absent | OAuth flows for authenticated MCP servers; without it, MCP servers requiring auth cannot be used |
| `LSPTool` | `"LSP"` | Absent | Language Server Protocol integration for semantic code actions (go-to-definition, rename, find-references); adds deep code understanding beyond grep |
| `TeamCreateTool` | `"TeamCreate"` | Absent | Prerequisite for Managed Agents (manager-executor architecture in plan.md); cannot assign work without creating agent teams |
| `TeamDeleteTool` | `"TeamDelete"` | Absent | Cleanup counterpart to TeamCreate |

### Medium Priority — Extends Capabilities

| Tool | TypeScript Name | Gap | Why Needed |
|------|----------------|-----|------------|
| `SyntheticOutputTool` | `"StructuredOutput"` | Absent | SDK/non-interactive structured output extraction; needed for programmatic integration of claurst results |
| `REPLTool` | `"REPL"` | Absent | Persistent REPL sessions (Python, Node, etc.); enables stateful computation without re-invoking bash; useful for data science workflows |
| `RemoteTriggerTool` | n/a | Absent | Bridge/remote triggering; only relevant if bridge remote sessions are heavily used |

### Low Priority — Niche / Special Mode Only

`BriefTool` is implemented. `SleepTool` is implemented. No other critical gaps.

---

## TUI / UI Gaps

### Critical TUI Gaps

| Gap | Description | Impact |
|----|-------------|--------|
| `AskUserQuestion` not interactive | `AskUserQuestion` tool returns metadata `{ type: "ask_user" }` but TUI `handle_query_event` never renders a dialog; the question is never shown | Model asks user something; user never sees it; model gets no answer; session hangs or errors |
| Permission dialog not wired to query loop | `render_permission_dialog()` widget exists but `AutoPermissionHandler` is used in all modes, bypassing interactive prompts; users never see "Allow / Deny" dialogs | All tools auto-allow in Default mode, which is a security regression vs TypeScript behavior |
| Mouse events causing scroll lag (#104) | Already listed in bugs; affects all interactive use | — |
| No rich message rendering | Messages rendered as plain `(Role, text)` pairs with color only; no syntax highlighting, no diff blocks, no code block borders | Code-heavy responses are hard to read |

### High-Value TUI Improvements

| Gap | Description | Priority |
|----|-------------|----------|
| Syntax highlighting in code blocks | Use `syntect` or `bat` integration to highlight fenced code blocks in assistant responses | High — renders output professional |
| Markdown rendering | Bold, italic, headers, lists in ratatui; `tui-markdown` crate exists | High |
| Inline diff view | When `Edit` tool runs, show a side-by-side or inline diff in the TUI rather than just "file written" | High — core UX of Claude Code |
| Tool output collapsing | Long tool outputs (e.g., 500-line bash output) should be collapsable with a "Show more" affordance | Medium |
| Input cursor and selection | Current input box shows `_` cursor; no left/right arrow navigation, no selection, no paste support (#76) | High — unusable for editing long prompts |
| Multi-line input | Shift+Enter for newlines; critical for pasting code into the input | High |
| Session list dialog | `/resume` command needs a TUI session picker, not just ID-based lookup | Medium |
| Token usage indicator | Show token count / context window fill % in status bar | Medium |
| Thinking display | Extended thinking blocks shown inline with collapsable toggle | Medium (partially implemented per live-thinking commit) |

---

## Differentiators (Claurst-Specific Advantages)

These are features claurst can offer that the TypeScript implementation cannot, or does with more friction:

| Feature | Value | Complexity |
|---------|-------|------------|
| Single static binary distribution | Zero npm, zero Node.js; `curl` install works | Already achieved — maintain it |
| Compile-time feature flags | Build without TUI, without bridge, without voice; smaller attack surface | Already implemented (36+ flags) — document them |
| Sub-10ms startup vs TypeScript | Rust binary vs Node.js cold start is ~50x faster for headless use | Already achieved — quantify and document |
| Memory safety by construction | No prototype pollution, no dependency confusion attacks at runtime | Architectural — document as security posture |
| MUSL static builds for containers | `cargo build --target x86_64-unknown-linux-musl` produces scratch-container-compatible binary | Medium effort — add CI target |
| Configurable provider auth headers | Once #117 is fixed, claurst can support any OpenAI-compat API with any auth scheme | Falls out of bug fix |
| `--output-format stream-json` for pipelines | Already implemented; TypeScript has it too but Rust is more composable as a Unix tool | Document + test |
| Managed Agents (planned) | Manager-executor architecture per plan.md; not in TypeScript upstream | High complexity — see plan.md |

---

## Anti-Features (Deliberately NOT Implement)

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| Sherpa-ONNX local ASR | High FFI complexity, niche audience, binary size bloat | Accept voice via OS speech-to-text bridge; already in Out of Scope |
| Kairos mode | Unclear spec, no community demand evidence, high design cost | Keep in Out of Scope until spec is clear |
| Analytics / telemetry | TypeScript sends usage events to Anthropic; Rust users expect no phoning home | Explicitly document that claurst has no telemetry; this is a differentiator |
| Node.js shim / interop layer | Defeats the purpose of a Rust rewrite | Never introduce; any TypeScript dep is an FFI smell |
| GUI (Electron/Tauri) | Out of scope; TUI is the target; GUI splits maintenance | Reject all PRs adding GUI; direct to separate project |
| Auto-updating without user consent | `upgrade` command should print instructions or ask; never silently replace the binary | Require explicit `--confirm` on upgrade |
| Bundled Node.js skills runtime | TypeScript skills rely on Node.js module resolution; do not port this | Port skills to Rust-native `.md` + prompt expansion only |
| WebSocket transport in bridge | TypeScript bridge supports WebSocket, SSE, and hybrid; Rust has long-poll only | Long-poll is simpler, correct, and sufficient; add WebSocket only if server side requires it |

---

## Feature Dependencies

```
Fix #123 (MCP security)
  ↓
MCP auth flow works safely
  ↓
McpAuthTool implementation

Fix #86 / #106 (api_base routing)
  ↓
Custom OpenAI URL (#106) works
  ↓
Minimax auth header (#117) works

AskUserQuestion TUI wiring
  ↓
Interactive permission dialog
  ↓
Default permission mode actually prompts users

TeamCreateTool + TeamDeleteTool
  ↓
Managed Agents (plan.md)
  ↓
/agents slash command
  ↓
/managed-agents slash command

Input cursor / multi-line input (#76 paste fix)
  ↓
/vim toggle (vim keybindings meaningful)
  ↓
keybinding configurability (fixes #47)

Syntax highlighting in TUI
  ↓
Inline diff view for Edit tool
```

---

## MVP Recommendation for This Milestone

Prioritize in this order:

1. **Fix #123 (MCP arbitrary execution)** — security cannot wait; ship nothing else until this is resolved
2. **Fix #86 + #106 (api_base routing)** — unblocks the majority of non-Anthropic users; one code path, two bug fixes
3. **Fix #104 (mouse capture)** — immediate daily-use quality improvement, one crossterm flag change
4. **Fix #76 (paste + cursor in input)** — input editing is a regression vs any terminal app users know
5. **Wire AskUserQuestion + permission dialog to TUI** — without this, interactive permission model is broken by design
6. **Add 5 highest-traffic missing commands**: `/add-dir`, `/context`, `/copy`, `/usage`, `/keybindings`
7. **Syntax highlighting + markdown in TUI** — biggest perceived quality jump for zero functional change
8. **Fix #47 (keyboard layouts) + configurable keybindings** — opens claurst to non-QWERTY users
9. **Fix #88 (voice/ALSA)** — fix or gate behind a cleaner error message
10. **TeamCreateTool + TeamDeleteTool** — prerequisite for Managed Agents work in plan.md

Defer to later phases: LSPTool, REPLTool, SyntheticOutputTool, 60+ low-traffic stub commands, Managed Agents full implementation.

---

## Sources

- spec/13_rust_codebase.md — Authoritative Rust implementation inventory (all 33 tools, 33 commands, TUI struct, CLI flags)
- spec/02_commands.md — Complete TypeScript command registry (100+ commands with types and behavior)
- spec/03_tools.md — Complete TypeScript tool list (40+ tools with schemas and algorithms)
- spec/INDEX.md — Cross-reference index and key numbers
- .planning/PROJECT.md — Active requirements, open issues, out-of-scope decisions
- GitHub issues #123, #104, #88, #86, #47, #76, #106, #117 (via PROJECT.md active requirements)
