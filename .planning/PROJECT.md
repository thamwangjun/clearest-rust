# claurst

## What This Is

claurst is a full Rust rewrite of Anthropic's Claude Code CLI, reimplementing its multi-provider LLM query loop, ratatui TUI, MCP client, slash commands, tool suite, plugin system, and bridge/remote-control protocol. It targets developers who want the Claude Code experience in a compiled, dependency-light binary — and serves as the upstream Rust reference implementation for the kuberwastaken/claurst community.

## Core Value

A reliable, feature-complete Rust alternative to Claude Code that stays current with upstream changes and fixes bugs fast enough that contributors trust it for daily use.

## Requirements

### Validated

- ✓ Multi-crate Rust workspace (12 crates): cli, core, api, query, tui, tools, commands, mcp, acp, bridge, buddy, plugins — existing
- ✓ Multi-provider LLM support: Anthropic, OpenAI, GCP, Bedrock, Ollama, and others via `LlmProvider` trait — existing
- ✓ Streaming agentic query loop with tool dispatch and auto-compact — existing
- ✓ ratatui TUI with event loop, dialogs, overlays, slash command framework — existing
- ✓ MCP client (JSON-RPC 2.0, stdio + HTTP/SSE) — existing
- ✓ ACP server (editor integration over stdio JSON-RPC 2.0) — existing
- ✓ Bridge / claude.ai remote-control (long-poll, JWT, SSE events) — existing
- ✓ Plugin system (discovery, ZIP extraction, marketplace, capability guard) — existing
- ✓ SQLite-backed session/conversation persistence — existing
- ✓ Feature-flag-gated crate compilation (36+ Cargo features) — existing
- ✓ Live model thinking process display — existing
- ✓ Import of Claude config — existing
- ✓ Welcome screen Enter keypress advances to TUI session without silent exit — v1.0 (BUG-01)
- ✓ Collapsed thinking block shows animated dots, not leaked content text — v1.0
- ✓ First-run no-credentials users land on Welcome page, not ProviderSetup — v1.0
- ✓ ANTHROPIC_AUTH_TOKEN accepted as bearer auth credential with conflict detection — v1.0
- ✓ `/status` displays "Authenticated (Bearer token)" for ANTHROPIC_AUTH_TOKEN — v1.0

### Active

- Characterization test suite covering unit, integration, and CLI snapshot paths before any refactoring begins
- Bloater elimination: extract long functions, split oversized files, decompose long parameter lists, replace primitive obsession
- Dispensable cleanup: remove dead code, de-duplicate logic, delete speculative over-abstractions
- Coupler fixes: Feature Envy, Inappropriate Intimacy, Message Chains across all 12 crates
- Change Preventer fixes: single responsibility per module, resolve Shotgun Surgery scatter

### Out of Scope

- TypeScript / Node.js implementation — this is Rust-only
- GUI (non-terminal) interface — ratatui TUI is the target
- Sherpa-ONNX local ASR (issue #114) — deferred; high integration complexity, small audience
- Kairos mode (issue #103) — deferred; unclear spec, needs separate design
- Feature parity gap-close — parity is largely achieved; new Claude Code features will be tracked as future milestones
- Managed Agents (plan.md) — deferred to a future milestone
- Upstream sync workflow — handled ad-hoc
- Security hardening (#123, #79, #96) — deferred to a future milestone
- Bulk bug fixes (#104, #88, #86, #47, #76, #106, #117) — deferred; will surface as future milestones

## Context

- **Shipped:** v1.0 (2026-05-09) — 3 phases, 9 plans, 4 days, ~354,938 lines of Rust
- **Upstream:** `git remote upstream → https://github.com/kuberwastaken/claurst.git`. Merging upstream changes is part of ongoing maintenance. The parent repo (`claurst/`) holds the git history and `.planning/`; Rust source lives at the repo root (Cargo.toml at `/`).
- **Spec reference:** `spec/` directory contains ~990 KB of Claude Code feature specs across 15 files (INDEX.md for navigation). This is the ground truth for parity work.
- **Existing plan:** `plan.md` has a detailed Managed Agents implementation plan (manager-executor architecture, budget splitting, `/managed-agents` slash command).
- **Open issues:** 20+ open GitHub issues; highest priority are security (#123), mouse/TUI (#104), Ollama (#86), and voice (#88).
- **Codebase map:** `.planning/codebase/` has architecture, stack, conventions, concerns, integrations, and testing docs (refreshed 2026-05-04).
- **Build:** Pure Rust, Cargo workspace, Tokio async runtime. Feature-flag-gated compilation. No external build tools needed beyond `cargo`.
- **Tech debt (v1.0):** `RenderContext::default()` in test paths always shows one dot; `auth_status()` mirrors resolver conflict check rather than delegating — both low priority.

## Constraints

- **Tech Stack:** Rust only. No new language runtimes. Dependencies must be compatible with workspace resolver v2.
- **Compatibility:** Must maintain CLI and TUI UX continuity across releases — no breaking changes to settings.json schema without migration.
- **Milestones:** New Claude Code features are discovered by the owner and brought in as new milestones — not continuously tracked.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Minimal v1 roadmap (single bug fix phase) | Feature parity is largely achieved; future work arrives as new milestones | ✓ Good — scope stayed tight; 3 phases delivered cleanly |
| `spec/` as parity ground truth | Spec was reverse-engineered from the official TypeScript Claude Code; 990 KB coverage across all subsystems | ✓ Good — used for bearer auth D-0x requirement derivation |
| TDD with `serial_test` for bearer auth | Async env-var tests have race conditions without serialization | ✓ Good — 6 stable integration tests, no flakiness |
| Remove `use_bearer_auth` field vs expose it (D-04 scope change) | User-approved in UAT Round 2: simpler API, resolver decides automatically | ✓ Good — cleaner UX, no user-facing toggle needed |
| `auth_status()` mirrors resolver conflict check (not delegates) | Minimal fix scope; full delegation would require refactoring fast-path callers | ⚠️ Revisit — documented as tech debt if conflict conditions grow |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

## Current Milestone: v1.1 Codebase Refactor

**Goal:** Transform the AI-written codebase into maintainable, idiomatic Rust by eliminating all major code smells across all 12 crates — anchored by a characterization test suite written before any code is moved.

**Target features:**
- Characterization test suite (unit + integration + CLI snapshot) — behavior anchor before refactoring
- Bloater elimination: long methods, large classes, primitive obsession, long parameter lists
- Dispensable cleanup: dead code, duplicate code, speculative generality
- Coupler fixes: Feature Envy, Inappropriate Intimacy, Message Chains
- Change Preventer fixes: Divergent Change, Shotgun Surgery

---
*Last updated: 2026-05-13 — v1.1 milestone started*
