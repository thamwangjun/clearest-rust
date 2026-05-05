# Architecture Research: claurst Feature Integration

**Project:** claurst (Rust Claude Code rewrite)
**Dimension:** Architecture — how new features slot into the existing 12-crate workspace
**Researched:** 2026-05-04
**Overall confidence:** HIGH (all findings from direct codebase inspection)

---

## 1. Crate Ownership Rules

The dependency graph is a strict DAG. Every placement decision must respect it:

```
claurst-core        (no workspace deps — stable foundation)
claurst-api         → core
claurst-mcp         → core
claurst-tools       → core, api, mcp
claurst-plugins     → core
claurst-query       → core, api, tools, plugins
claurst-bridge      → core, api, query
claurst-acp         → core, api
claurst-buddy       → core
claurst-tui         → core, api, tools, query, mcp
claurst-commands    → core, api, tools, query, mcp, tui, plugins, bridge   (widest fan-in)
claurst-cli         → everything (binary entry point)
```

The critical constraint: a crate can only import from crates it depends on. `commands` is already the broadest crate — it can import from anything except `cli`. `tui` cannot import from `commands`. New code that needs both TUI rendering and command dispatch must either live in `commands` or route through a shared type in `core`.

---

## 2. Where Slash Commands Go

**Verdict: all new slash commands belong in `crates/commands/src/lib.rs` (or a new sub-module extracted from it), never in `crates/tui/`.**

Rationale:
- `SlashCommand` trait and `CommandContext`/`CommandResult` types are defined and owned by `claurst-commands`.
- `claurst-tui` does not depend on `claurst-commands` — commands and TUI are siblings in the graph. TUI dispatches to commands at runtime via `claurst-commands::execute_command`, but cannot define new commands itself.
- `all_commands()` at `crates/commands/src/lib.rs:8019` is the single registration point. Every new command is added here.

**The current structural problem:** `crates/commands/src/lib.rs` is 8,576 lines. Adding more commands to this file increases merge conflict risk substantially. The file should be split into per-command modules under `crates/commands/src/commands/` before adding the next batch of feature-parity commands (100+ spec commands remain).

**Recommended split strategy:**
```
crates/commands/src/
  lib.rs              (trait definitions, CommandContext, CommandResult, all_commands())
  commands/
    compact.rs
    model.rs
    cost.rs
    managed_agents.rs  (new — from plan.md phase 2)
    ...one file per command...
```

This is purely a refactor within the `commands` crate boundary — no dependency graph changes needed.

**TUI-only slash command behavior (overlays, dialogs):** Commands that need to open a TUI overlay (like `/rewind` opening `OpenRewindOverlay`) use a `CommandResult` variant, not a direct TUI call. The TUI matches on `CommandResult` variants after `execute_command` returns. New overlay-triggering commands follow this same pattern: add a `CommandResult` variant, handle it in `tui/src/app.rs`.

---

## 3. Where New Tools Go

**Verdict: all new tool implementations go in `crates/tools/src/`, one file per tool, registered in the `ToolRegistry` in `crates/tools/src/lib.rs`.**

The `Tool` trait is:
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

**Placement rules:**
- Each new tool is a `struct FooTool` in `crates/tools/src/foo_tool.rs`.
- Registered in the tool list in `crates/tools/src/lib.rs`.
- `ToolContext` (defined in `crates/tools/src/lib.rs`) is the only injection point for cross-cutting dependencies (config, cost tracker, permission handler, session ID, working dir). If a tool needs a new shared resource, add it to `ToolContext` rather than using global state.

**For Managed Agents:** `plan.md` correctly identifies adding `managed_agent_config: Option<ManagedAgentConfig>` to `ToolContext`. This is the right pattern — `AgentTool` reads it from context rather than from a global.

**MCP tools:** MCP tools are NOT `Tool` implementations living in `crates/tools/`. They are wrapped at runtime by `McpToolWrapper` (in `crates/cli/src/main.rs:54-61`) and appear as `Tool`-conforming objects. No changes to this wrapping mechanism are needed for new MCP tools — they are discovered dynamically from running MCP servers.

---

## 4. MCP Security Sandboxing (Issue #123)

**The current gap:** `McpServerConfig` has no `trusted` or `scope` field. The project-level settings loader (`find_project_settings`, `core/src/lib.rs:1477`) walks up the directory tree and merges any `.claurst/settings.json` it finds, including the `mcp_servers` array, with **no user consent step**. A malicious repo can add an arbitrary stdio MCP server that executes commands when the user opens the project.

**How to add sandboxing without breaking the existing MCP client:**

The MCP client (`claurst-mcp`) is a pure transport layer — it connects to servers and dispatches JSON-RPC. Security enforcement must live at the config merge layer in `claurst-core` and at the connection setup layer in `claurst-cli`. This keeps the `mcp` crate unchanged.

**Required component changes:**

**`claurst-core` (`crates/core/src/lib.rs`):**

Add a `scope` field to `McpServerConfig`:
```rust
pub struct McpServerConfig {
    pub name: String,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub env: HashMap<String, String>,
    pub url: Option<String>,
    pub server_type: String,
    // NEW:
    #[serde(default)]
    pub scope: McpServerScope,   // User | Project
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum McpServerScope {
    #[default]
    User,    // from global ~/.claurst/settings.json — auto-trusted
    Project, // from project .claurst/settings.json — requires consent
}
```

In `find_project_settings` / `merge`: tag all servers found in a project config file with `scope: McpServerScope::Project` before merging them into the effective config. User-level servers remain `User` scope.

**`claurst-core` (new: `crates/core/src/mcp_trust.rs`):**

Add a `McpTrustStore` that persists which project-level MCP servers the user has approved, keyed by `(project_dir_hash, server_name, command_hash)`. Store in `~/.claurst/mcp_trust.json`. Provide:
```rust
pub fn is_trusted(server: &McpServerConfig, project_dir: &Path) -> bool
pub async fn prompt_and_record_trust(server: &McpServerConfig, project_dir: &Path) -> bool
```

**`claurst-cli` (`crates/cli/src/main.rs`):**

Before connecting project-scoped MCP servers (currently done at line ~808 where `McpToolWrapper` is built), check `McpTrustStore::is_trusted`. If not trusted, call `prompt_and_record_trust` — in TUI mode this is an `elicitation_dialog`; in headless mode it prints a y/n prompt and blocks. Untrusted servers are skipped, not connected.

**Why this is safe to add:** The `McpManager` and all transport code in `claurst-mcp` are unchanged. The trust gate is purely a filter applied in `claurst-cli` at server connection time. `claurst-commands` gains a `/mcp trust` subcommand variant to manage the trust store.

**Boundary summary:**

| Component | Change |
|-----------|--------|
| `claurst-core` | Add `McpServerScope` to `McpServerConfig`; new `McpTrustStore` in `mcp_trust.rs` |
| `claurst-cli` | Trust check before connecting project-scoped servers |
| `claurst-commands` | Optional `/mcp trust`/`/mcp revoke` subcommands |
| `claurst-mcp` | No changes |
| `claurst-tui` | Reuse existing `elicitation_dialog` for consent UI |

---

## 5. Upstream Sync Workflow

**The repo structure:** The planning docs and git history live alongside the Rust source in the `clearest-rust/` repo root (no `src-rust/` subdirectory — workspace Cargo.toml is at the root). The upstream remote is `https://github.com/kuberwastaken/claurst.git`. Upstream commits touch `crates/` paths directly.

**Conflict surface analysis:**

The files most likely to conflict with upstream changes are the same files that are largest and most active:
- `crates/commands/src/lib.rs` (8,576 lines) — upstream adds new commands, local adds managed-agents and security commands
- `crates/tui/src/app.rs` (5,918 lines) — upstream adds UI features, local adds managed-agents TUI
- `crates/core/src/lib.rs` (4,246 lines) — upstream adds config fields, local adds MCP trust types and managed-agent config
- `crates/cli/src/main.rs` (3,502 lines) — upstream changes startup/dispatch, local changes MCP security gate

**Recommended sync strategy:**

1. **Split the monoliths before taking upstream** — the module split of `commands/src/lib.rs` (point 2 above) and extracting `core/src/lib.rs` into sub-modules (`config.rs`, `permissions.rs`, `session_storage.rs`, etc.) reduces conflict surface from one 8K-line file to many 300-500 line files. Upstream changes to individual commands will only conflict with that command's file.

2. **Use a dedicated merge branch** — use `git subtree` or a dedicated merge branch rather than squash-merging. This preserves per-file history and makes conflict resolution per-commit rather than per-blob.

3. **Feature-branch staging pattern:**
   ```
   main              → stable, always builds
   upstream-merge    → merge upstream here first, resolve conflicts, run CI
   feature/X         → local feature work
   ```
   Never merge upstream directly to `main`. Always go through `upstream-merge` first.

4. **CI gate on feature-flag matrix** — the 36 Cargo features mean upstream changes can silently break non-default configurations. CI should test at minimum: `default`, `dev_full`, and any security/bridge features. Add this before taking regular upstream syncs.

5. **Lock `crates/core/src/lib.rs` during upstream windows** — because nearly every other crate depends on core, conflicts there have cascading effects. When a major upstream sync is planned, defer local core changes until after the merge is clean.

---

## 6. Component Boundaries That Need to Change for Managed Agents

The `plan.md` is architecturally sound and requires no new crates. The boundary changes are:

**`claurst-core` — data model boundary (Phase 1 in plan.md):**
- Add `ManagedAgentConfig`, `BudgetSplitPolicy`, `ManagedAgentPreset` to `crates/core/src/lib.rs` (or a new `crates/core/src/managed_agents.rs` sub-module, re-exported from `lib.rs`)
- The types land in `core` because they are needed by `query` (orchestrator) AND `commands` (slash command) AND `tui` (display). Core is the only crate all three depend on.

**`claurst-query` — orchestration boundary (Phase 3 in plan.md):**
- New file: `crates/query/src/managed_orchestrator.rs` — self-contained module
- `run_query_loop()` in `lib.rs` gains a branch at startup: if managed config is active, call `managed_orchestrator::apply_manager_config(&mut query_config, managed_cfg)` before entering the loop. This avoids scattering managed-agent logic throughout `run_query_loop`.
- `AgentTool` (in `crates/query/src/agent_tool.rs` per plan.md) reads `ToolContext.managed_agent_config` to default executor model. This is a small, well-bounded change.

**`claurst-tools` — ToolContext boundary (Phase 3 in plan.md):**
- Add `managed_agent_config: Option<ManagedAgentConfig>` to `ToolContext`
- This is the only structural change to the tools crate for managed agents

**`claurst-commands` — command boundary (Phase 2 in plan.md):**
- Add `ManagedAgentsCommand` to the commands crate
- `CommandResult` already has `ConfigChangeMessage` — no new variants needed for managed agents
- The command reads/writes `ManagedAgentConfig` via the existing `save_settings_mutation()` pattern

**`claurst-tui` — display boundary (Phase 4 in plan.md):**
- Extend `agents_view.rs` with `AgentRole` enum and cost breakdown display
- Add a `managed_agent_cost_breakdown` field to `App` state
- No new `CommandResult` variants needed — TUI reads the live `Config` after `ConfigChangeMessage` is processed

**No new crates needed.** The manager-executor pattern reuses `AgentTool` + `run_query_loop` as designed in `plan.md`. The `CostTracker` is already `Arc`-shared to sub-agents. `ProviderRegistry` already supports all cross-provider combos.

---

## 7. Suggested Build Order for Phases

Given the dependency graph and what conflicts are likely with upstream, the recommended build order across all active work items is:

### Stage 1: Structural (prerequisite for everything else)
1. **Split `crates/commands/src/lib.rs`** into per-command modules — reduces merge conflicts, unblocks parallel command work. No functional change; test suite must pass before and after.
2. **Split `crates/core/src/lib.rs`** into sub-modules (`config.rs`, `permissions.rs`, `session.rs`) — same rationale. Both splits are refactors only, no dependency changes.

### Stage 2: Security (takes priority over features per PROJECT.md)
3. **MCP trust sandboxing (issue #123)** — add `McpServerScope`, `McpTrustStore`, trust gate in `cli`. Touches `core` and `cli` only. Ship before any feature that adds more MCP surface.
4. **`auth.json` permissions fix** — `set_mode(0o600)` in `auth_store.rs`. One line, highest-impact security fix.
5. **SSRF protection in `web_fetch`** — block RFC 1918/loopback before outbound request.

### Stage 3: Bug fixes (issues #86, #88, #76, #47, #104) — parallel with stage 2
- Ollama URL fix: `crates/api/src/providers/` — isolated provider change
- Voice/ALSA: `crates/core/src/voice.rs`, `crates/tui/src/voice_capture.rs` — feature-gated
- API key paste: `crates/tui/src/` — TUI input handling
- Keyboard layout: `crates/tui/src/prompt_input.rs` — input layer

### Stage 4: Managed Agents (plan.md phases 1-6)
- Phase 1 (Config): `core` — after stage 1 core split, clean target
- Phase 2 (Command): `commands` — after stage 1 commands split, clean target
- Phase 3 (Orchestrator): `query` — new file, minimal conflict risk
- Phase 4 (TUI): `tui` — agents_view extension
- Phase 5 (Sessions): `core` — additive fields only
- Phase 6 (Tests): ongoing

### Stage 5: Feature parity (slash commands, tools)
- Runs in parallel with Managed Agents phases 3-6
- Each new slash command is one file in `commands/src/commands/` — no cross-file conflicts
- Each new tool is one file in `tools/src/` — no cross-file conflicts

### Stage 6: Regular upstream sync cadence
- After stages 1-2, upstream merges are lower risk (monoliths split, security stable)
- Monthly sync cadence using the `upstream-merge` branch pattern

---

## 8. Integration Risks

### High Risk

**`crates/commands/src/lib.rs` is the merge collision point.** Any upstream change that adds a command and any local change that adds a command will conflict in the same 8,576-line file. The split (Stage 1) is the mitigation — do it before taking upstream and before adding new commands.

**`crates/core/src/lib.rs` cascades.** A conflict here breaks the build for all 11 dependent crates. When upstream changes `Config` or `Settings` structs simultaneously with local managed-agent field additions, the merge is non-trivial because serde `#[serde(default)]` usage must be validated after each merge. Mitigation: split the file and serialize all local core changes relative to upstream sync windows.

**MCP trust gate timing.** Issue #123 is a live security bug. Implementing the trust gate changes `cli/src/main.rs` significantly. If upstream also touches `main.rs` (startup sequence, MCP initialization), the merge is complex. Mitigation: land the trust gate as its own PR before taking upstream, then merge upstream on top.

### Medium Risk

**`ToolContext` is a shared struct.** Adding `managed_agent_config` to `ToolContext` requires a recompile of all tools — but because `ToolContext` is defined in `claurst-tools` and all tool files are in the same crate, this is contained. The risk is that existing tools construct `ToolContext` in tests — those test construction sites must be updated. The ~35 tool implementations all need `managed_agent_config: None` in their test setups.

**Feature-flag compilation gaps.** `managed_agents` as a capability may need a Cargo feature gate (e.g., `feature = "managed_agents"`). If it does, all 36 existing features must still compile correctly with it. The CI gap (features not fully tested) means this can silently break. Mitigation: add the managed-agents feature to `dev_full` from the start; test that combination explicitly.

**Cross-provider managed-agent auth validation** (noted in `plan.md` section 4). The `ProviderRegistry` handles all providers, but auth validation during `/managed-agents setup` requires checking `AuthStore` for both manager and executor providers simultaneously. The auth check logic lives in `crates/core/src/auth_store.rs`. This is a read-only check, but it must happen at command execution time (inside `ManagedAgentsCommand::execute`), which means `CommandContext` needs access to `AuthStore`. Currently `CommandContext` has `config: Config` but not a live `AuthStore` handle. Either pass `AuthStore` through `CommandContext` or read credentials from the config's `api_key` field (which is already merged in).

### Low Risk

**Session backwards compatibility.** The `agent_role` and `managed_session_id` fields in `plan.md` Phase 5 both use `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]`. Existing JSONL session files have neither field — deserialization will correctly default them to `None`. No migration needed.

**`McpServerScope` backwards compatibility.** `McpServerConfig` gains a new field with `#[serde(default)]`. Existing `settings.json` files have no `scope` field; deserialization defaults to `User` scope, which is the correct trust level for user-written global config. No migration needed.

**TUI agents_view extension.** `AgentRole` is additive to existing `AgentInfo`. The render function is extended, not replaced. Risk is purely visual regression (layout shifts), not functional breakage.

---

## 9. Key Abstractions to Preserve

Do not change these interfaces without understanding all dependents:

- **`Tool` trait** — 35 implementations + MCP wrapper + all tool tests. Adding fields to `ToolContext` is safe; changing `execute()` signature is not.
- **`SlashCommand` trait** — all commands implement it. `CommandResult` enum can gain new variants (TUI must handle them); existing variants must not change.
- **`QueryEvent` enum** — the channel from query loop to TUI. Adding variants is safe; renaming or removing breaks TUI rendering.
- **`LlmProvider` trait** — 30+ implementations. Do not add required methods.
- **`ContentBlock` enum** — used in session JSONL serialization. Adding variants is safe; removing or renaming breaks existing sessions.
- **`Config` and `Settings` serde shapes** — all new fields must use `#[serde(default)]` to preserve backwards compatibility with existing `~/.claurst/settings.json` files.

---

*Sources: direct inspection of crates/commands/src/lib.rs, crates/core/src/lib.rs, crates/cli/src/main.rs, crates/mcp/src/lib.rs, crates/tools/src/lib.rs, .planning/codebase/ARCHITECTURE.md, .planning/codebase/CONCERNS.md, .planning/PROJECT.md, plan.md — 2026-05-04*
