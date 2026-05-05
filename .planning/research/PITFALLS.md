# Domain Pitfalls

**Domain:** Rust rewrite of Claude Code CLI (claurst) — feature parity, upstream sync, TUI, security, async I/O
**Researched:** 2026-05-04
**Confidence:** HIGH (codebase-verified for Rust/security pitfalls; MEDIUM for upstream sync patterns)

---

## Critical Pitfalls

Mistakes that cause rewrites, security incidents, or data loss.

---

### Pitfall 1: MCP Project-Level Config Enables Arbitrary Command Execution

**What goes wrong:** A malicious or compromised project-level `.claurst/mcp.json` (or equivalent) can inject an MCP server entry that executes arbitrary OS commands under the user's account. Because MCP's STDIO transport executes any command it receives with no sanitisation boundary, the project config is a direct code execution vector (issue #123). This is the claurst analogue of CVE-2025-53109/53110 and the 200,000-server STDIO vulnerability class disclosed in 2025.

**Why it happens:** MCP STDIO transport intentionally launches sub-processes. The distinction between "trusted global config" and "untrusted project config" is not enforced at the transport layer, so project-level entries run with the same privilege as user-level ones.

**Consequences:** Any repository a user opens can silently execute code — supply-chain attack surface.

**Prevention:**
- Distinguish global MCP config (trusted, runs silently) from project-level MCP config (untrusted, requires explicit allow-listing per server).
- On first encounter of a project-level MCP server, display a confirmation dialog listing the command to be launched and require user approval before spawning.
- Store approved project MCP entries in `~/.claurst/mcp_approved.json` keyed by `(project_path, server_name, command_hash)` so approval is not re-asked on every start.
- Never auto-approve on `--yes` / non-interactive flags for MCP server launches from project config.

**Detection:** Issue #123 is the open tracker. Warning sign: any code path that reads `<project>/.claurst/*.json` and passes `command` fields directly to `Command::new` without an intermediate approval gate.

**Phase:** Security hardening — highest priority, should be in the first phase.

---

### Pitfall 2: Path Containment Bypass via `starts_with` Without Canonical Trailing Separator

**What goes wrong:** `ToolContext::path_is_within_workspace` uses `resolved.starts_with(root)` after canonicalization. In Rust, `Path::starts_with` on `PathBuf` is component-aware (it checks full path components, not byte prefixes), which is correct. However, if `canonicalize` fails (file does not exist yet), the fallback is `path.to_path_buf()` — an unresolved, non-canonical path — which could be manipulated by symlinks or `..` components.

**Why it happens:** `canonicalize` returns `Err` for paths that do not yet exist (e.g., before a file is created). The `unwrap_or_else(|_| path.to_path_buf())` fallback silently degrades to unchecked path comparison. A tool creating a new file outside the workspace would pass the check on the non-canonical pre-creation path.

**Consequences:** File write/create operations can escape the workspace sandbox — permission bypass analogous to CVE-2025-53110 (directory containment bypass in Anthropic's own MCP server).

**Prevention:**
- For path permission checks on files that may not exist yet, canonicalize the *parent directory* (which must exist) and append the filename component after.
- Reject paths containing `..` components before canonicalization.
- Add a unit test: `path_is_within_workspace("/workspace/../etc/passwd")` must return false even when `/workspace` exists.

**Detection:** Issues #79 and #96 (permission bypass). Look for `unwrap_or_else(|_| path.to_path_buf())` at permission check callsites.

**Phase:** Security hardening — address alongside #123.

---

### Pitfall 3: Credentials File Written World-Readable

**What goes wrong:** `auth_store.rs` writes `~/.claurst/auth.json` with `std::fs::write(path, json)` which inherits the process umask. A typical umask of 022 makes the file readable by all users on the system. API keys and OAuth tokens (including refresh tokens) are exposed to any co-tenant process.

**Why it happens:** Rust's `std::fs::write` does not accept a mode argument. A follow-up `set_permissions` call is required but was never added.

**Consequences:** Credential theft on multi-user or shared systems; any process running as the same user (malware, IDE plugins) reads API keys without any additional privilege.

**Prevention:**
```rust
use std::os::unix::fs::PermissionsExt;
std::fs::write(&path, &json)?;
std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
```
On Windows, use `fs::write` then ACL restriction via the `windows-permissions` crate.

**Detection:** `crates/core/src/auth_store.rs:52-60`. Confirmed by CONCERNS.md audit. Fix is one-line.

**Phase:** Security hardening — lowest effort, highest impact.

---

### Pitfall 4: Plugin Hooks Use `sh -c` with Unsanitized Hook Commands

**What goes wrong:** `crates/plugins/src/hooks.rs:166,300` spawns `sh -c <hook_command>`. If any part of `hook_command` is assembled from runtime user input or mutable plugin metadata (rather than the static, at-install-time plugin manifest), this is a shell injection vector.

**Why it happens:** Shell invocation is the easiest way to run arbitrary commands with pipes and redirects. The injection surface is non-obvious because the manifest is "trusted" — but plugin manifests can be updated, and future features (user-editable hooks) could introduce injection paths.

**Consequences:** Privilege escalation from plugin hook to arbitrary shell command execution.

**Prevention:**
- Document and assert in code that `hook_command` must come exclusively from the static, install-time plugin manifest — never from runtime user input.
- Where possible, parse the hook command into `argv` and use `Command::new(argv[0]).args(&argv[1..])` to spawn without a shell.
- Add a lint/audit comment at each `sh -c` callsite identifying the trust boundary.

**Detection:** `crates/plugins/src/hooks.rs`. Warning sign: any refactor that passes session state, user input, or tool results into hook command construction.

**Phase:** Security hardening. Low urgency until plugin hooks become user-editable; document the trust boundary now.

---

### Pitfall 5: Mouse Capture Unconditionally Breaks Native Text Selection

**What goes wrong:** `setup_terminal()` unconditionally calls `EnableMouseCapture`. When mouse capture is active, the terminal emulator routes all mouse events to the application rather than the OS. This prevents native text selection with click-and-drag in every terminal emulator that respects X11 mouse reporting (xterm, iTerm2, Alacritty, Kitty, etc.). Users cannot copy output without holding Shift (terminal-specific workaround). This is issue #104.

**Why it happens:** Mouse capture is all-or-nothing in crossterm's current API. There is no built-in crossterm mechanism for "capture clicks but not drags" or "let shift-click pass through."

**Consequences:** Users lose native copy-paste. Long conversation outputs cannot be selected and copied with the mouse, degrading daily usability.

**Prevention:**
- Make mouse capture opt-in via a `settings.json` key (`mouse_capture: bool`, default `false` or `true` with a migration prompt).
- Warn users in the UI that enabling mouse capture disables native text selection; provide the terminal-specific workaround (Shift+click/drag).
- For scroll handling specifically: implement scroll via keyboard events and `MouseScrollDown`/`MouseScrollUp` only; these can be captured without capturing drag events on some terminals.
- On Windows, test with crossterm's `EnableMouseCapture` separately — behavior differs from POSIX terminals.

**Detection:** `crates/tui/src/lib.rs:192` — `EnableMouseCapture` in `setup_terminal()`. Warning sign: any PR that adds mouse drag handling without making capture configurable.

**Phase:** TUI polish phase. Prerequisite: settings schema must support the `mouse_capture` key.

---

### Pitfall 6: `std::env::set_var` in Async Multi-Threaded Tests

**What goes wrong:** 15+ callsites in test code mutate process-global environment variables (`ANTHROPIC_API_KEY`, `HOME`, voice kill-switch vars) using `std::env::set_var`/`remove_var` inside `#[tokio::test]` harnesses. Tokio runs tests on a multi-threaded executor. Concurrent env var mutation is undefined behaviour in Rust 1.81+ (and documented UB in POSIX — `getenv`/`setenv` are not thread-safe).

**Why it happens:** Env vars are the simplest way to inject test configuration without passing it through constructors. The pattern was common before Tokio's multi-threaded test executor made the UB practical.

**Consequences:** Intermittent test failures (flaky CI), silently wrong test results when two tests race on the same env var, potential process-level memory corruption in POSIX builds.

**Prevention:**
- Short-term: Annotate affected tests with `#[serial_test::serial]` to force sequential execution within the env-mutating group.
- Medium-term: Replace env var injection with explicit `Config` or `Arc<Mutex<Config>>` passing through constructors. Remove all `std::env::set_var` calls from non-test code paths.
- Never use `std::env::set_var` inside `#[tokio::test]` without `#[serial]`.

**Detection:** `crates/core/src/lib.rs:3821-3858`, `crates/core/src/import_config.rs:887-901`, `crates/core/src/voice.rs:618-652`, `crates/mcp/src/lib.rs:1416-1439`. Confirmed by CONCERNS.md.

**Phase:** Technical debt cleanup. Should be fixed before CI is expanded; currently masks real failures.

---

## Moderate Pitfalls

Mistakes that cause regressions, degraded UX, or accumulating technical debt.

---

### Pitfall 7: TypeScript → Rust Translation Produces Deceptive Async Equivalences

**What goes wrong:** TypeScript `Promise.all([a(), b()])` is an eager, concurrent execution. The Rust equivalent looks like `tokio::join!(a(), b())` — but only if the futures are spawned correctly. A naive port of `async function foo() { await bar(); await baz(); }` becomes sequential `foo().await; bar().await; baz().await;` — concurrent in TS, sequential in Rust.

Similarly, TypeScript callbacks fire asynchronously (microtask queue). Rust `async fn` futures are lazy — they do not execute until polled. Fire-and-forget patterns from TS (`somePromise()` without `await`) must become `tokio::spawn(some_future())` in Rust, or the work never runs.

**Why it happens:** The syntax looks similar (`await` in both), but the execution model is inverted. JS futures are eager; Rust futures are lazy.

**Consequences:** Features that worked concurrently in Claude Code TypeScript silently become sequential in claurst, degrading streaming performance and causing apparent hangs.

**Prevention:**
- During feature parity work, audit every translated async function for concurrency intent.
- Use `tokio::join!` for "run these concurrently and wait for all", `tokio::spawn` for fire-and-forget background tasks.
- Add latency regression tests for streaming tool dispatch that would catch serialised-where-parallel regressions.

**Detection:** Warning sign: streaming responses feel slower in claurst than in Claude Code for equivalent operations. Look for sequential `.await` chains in tool dispatch and query loop code.

**Phase:** Feature parity phases. Verify during each tool/command translation.

---

### Pitfall 8: `tokio::select!` Cancels In-Flight Work Non-Deterministically

**What goes wrong:** `tokio::select!` drops all non-winning futures at their current `.await` point. Any state accumulated in those futures (partial writes, acquired locks, partially-sent messages) is discarded. In a TUI event loop, a pattern like:

```rust
loop {
    select! {
        event = terminal_events.recv() => { handle_event(event); }
        msg = query_result.recv() => { update_ui(msg); }
    }
}
```

...will silently drop `update_ui(msg)` processing if a terminal event arrives simultaneously, or drop a buffered terminal event if a query result races it.

**Why it happens:** `select!` is the idiomatic Rust tool for racing futures, but its cancellation semantics require every branch future to be "cancellation-safe." Many async channel `recv()` calls are cancellation-safe, but custom futures, partially-written buffers, or futures holding locks are not.

**Consequences:** Lost UI updates, dropped streamed tokens, silent message loss from the query loop, or deadlocks from cancelled lock holders.

**Prevention:**
- Prefer separate Tokio tasks communicating via channels over large `select!` blocks.
- When `select!` is used, ensure all branch futures are cancellation-safe (check the Tokio docs per future type).
- Re-use the same future instance across loop iterations (via `pin_mut!` or `Box::pin`) for futures with internal state (e.g., `tokio::time::sleep`).

**Detection:** Warning sign: intermittent dropped streaming tokens or missed key events under load. Review every `select!` in `crates/tui/src/app.rs` for non-cancellation-safe branches.

**Phase:** Any phase touching the TUI event loop or query loop. Introduce a design review checklist for new `select!` usage.

---

### Pitfall 9: Upstream Sync Merge Conflicts Concentrate in Monolithic Files

**What goes wrong:** Three of claurst's largest files — `crates/commands/src/lib.rs` (8,576 lines), `crates/tui/src/app.rs` (5,918 lines), `crates/core/src/lib.rs` (4,246 lines) — are primary targets for both upstream feature additions and claurst-local feature work. When the upstream kuberwastaken/claurst adds a command or TUI widget to these same files, every merge creates conflicts across thousands of lines.

**Why it happens:** Monolithic files accumulate conflicts proportional to their size. With no module boundary, any upstream change to command handling collides with any claurst-local command change.

**Consequences:** Upstream sync takes hours instead of minutes; merge errors introduce subtle bugs; contributors avoid syncing, accelerating divergence.

**Prevention:**
- Split `commands/src/lib.rs` into per-command files before the next major upstream sync.
- Split `tui/src/app.rs` into widget-specific modules.
- Use `git diff upstream/main...HEAD -- crates/commands/src/lib.rs` before every sync to quantify the conflict surface.
- Maintain a `CHANGELOG-local.md` of claurst-specific changes so they can be quickly re-applied after a disruptive upstream sync.

**Detection:** Warning sign: a `git merge upstream/main` produces more than 50 conflict markers in a single file. At 8,576 lines, `lib.rs` hits this threshold on any non-trivial upstream commit.

**Phase:** Refactoring phase (module split) should precede the next major upstream sync.

---

### Pitfall 10: Feature Flag Combinatorial Testing Gap

**What goes wrong:** `crates/core/Cargo.toml` defines 36 feature flags. CI likely tests `dev_full` (all features on) and the default set. Features that are individually enabled but not tested in any CI combination can silently break. Historically, the `voice` feature broke (issue #88) because ALSA integration was not exercised in the CI matrix.

**Why it happens:** Combinatorial feature testing is exponential. 36 features = 2^36 combinations. Teams default to testing extremes (all on / all off) and miss mid-combinations.

**Consequences:** Shipped binaries with enabled features (voice, computer-use, bridge) break at runtime for users even though CI is green.

**Prevention:**
- Identify which features are actually enabled in production release builds. Test those explicitly.
- For each non-default feature, add a CI job: `cargo test --features <feature>` in addition to `dev_full`.
- Replace empty feature flags (no code behind them) with runtime configuration to reduce the flag count.

**Detection:** Warning sign: a GitHub issue reports a broken feature that CI never tested in isolation (pattern matching issues #88, #114).

**Phase:** CI/infrastructure phase. Low-hanging: add per-feature CI jobs.

---

### Pitfall 11: `unwrap()` Panics in Production Cascade Through Tokio Tasks

**What goes wrong:** ~410 `.unwrap()` calls outside test modules. In a Tokio multi-threaded runtime, a panic in a spawned task does not kill the process — the task is silently dropped. However, panics on the main thread or in `tokio::spawn` tasks holding `std::sync::Mutex` locks poison those locks. Subsequent lock acquisitions will fail with `PoisonError`, and callers that call `.unwrap()` on the lock result will then also panic — creating a cascade.

**Why it happens:** Rust's `std::sync::Mutex::lock()` returns `LockResult`, which developers habitually `.unwrap()`. One task panic can poison a mutex and take down unrelated functionality.

**Consequences:** A single streaming error in one tool call can poison a mutex and crash the entire TUI render loop, requiring the user to restart.

**Prevention:**
- Standardize on `parking_lot::Mutex` (already a workspace dependency) — it never poisons.
- Replace `unwrap()` on `Mutex::lock()` with `parking_lot::Mutex::lock()` (infallible).
- Replace `panic!` in production `match` arms (`crates/tui/src/elicitation_dialog.rs:641,762`, `crates/commands/src/named_commands.rs:1178,1238`) with `Err(...)` returns.

**Detection:** CONCERNS.md catalogues the highest-risk sites. Warning sign: any crash report where the panic message is "Expected Ask, got ..." or "Expected Message" — these are production panics, not test panics.

**Phase:** Technical debt cleanup. Standardize `parking_lot::Mutex` first (mechanical change), then address `panic!` in match arms per crate.

---

### Pitfall 12: Web Fetch OOM via Unbounded Response Buffering

**What goes wrong:** `crates/tools/src/web_fetch.rs:351` calls `resp.text().await` before any size check. The 100 KB truncation is applied *after* the full response is read into memory. A malicious or unexpectedly large HTTP response (e.g., a CDN serving a multi-GB binary at a well-known URL) will exhaust heap before truncation applies.

**Why it happens:** `reqwest::Response::text()` is the simplest API for getting response body as string. Size limiting requires streaming with `Response::chunk()`.

**Consequences:** OOM crash of the entire claurst process; potential DoS if the AI is instructed to fetch a large URL.

**Prevention:**
```rust
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024; // 10 MB hard cap
let mut bytes = Vec::with_capacity(65536);
while let Some(chunk) = resp.chunk().await? {
    bytes.extend_from_slice(&chunk);
    if bytes.len() > MAX_BODY_BYTES {
        break;
    }
}
```
Apply the existing 100 KB text truncation downstream.

**Detection:** `crates/tools/src/web_fetch.rs:351`. Warning sign: claurst OOM-crashes while a web fetch tool is active.

**Phase:** Security/stability hardening. Low implementation effort.

---

### Pitfall 13: OAuth State/PKCE Not Validated — CSRF Risk

**What goes wrong:** `crates/cli/src/oauth_flow.rs:234` parses the callback URL but there is no confirmed assertion that the returned `state` parameter matches the originally generated state. If state validation is absent, an attacker who can redirect the browser callback (e.g., via a malicious page open at the same time) can inject a valid OAuth code with a forged state and hijack the authentication.

**Why it happens:** OAuth PKCE flows in native CLI applications are more complex than web flows. State validation is easy to omit when focusing on the happy path.

**Consequences:** OAuth CSRF — attacker links their account to the victim's claurst session, gaining access to conversations.

**Prevention:**
- Verify that `returned_state == generated_state` before calling the token exchange endpoint.
- If already implemented elsewhere, add a comment at `oauth_flow.rs:234` citing the exact validation location.
- Add a test: callback with a mismatched state must return `Err`.

**Detection:** `crates/cli/src/oauth_flow.rs`. Warning sign: no `assert_eq!(state, original_state)` or equivalent check in the callback handler.

**Phase:** Security hardening. Verify or implement during the auth/credentials phase.

---

### Pitfall 14: SSRF via `WebFetch` to Internal Network Endpoints

**What goes wrong:** `WebFetchTool` accepts any URL from the AI model without checking whether the target resolves to a loopback, RFC 1918, or APIPA address. A prompt-injected or adversarial model response could instruct the tool to probe `http://169.254.169.254/` (AWS IMDS), `http://localhost:6379/` (Redis), or any other internal service.

**Why it happens:** SSRF protection requires resolving the hostname before connecting and checking the resulting IP against blocklists — a non-obvious step not in `reqwest`'s default API.

**Consequences:** Exfiltration of cloud instance metadata (IAM credentials), probing internal services, potential lateral movement.

**Prevention:**
- Resolve the hostname via DNS before passing to `reqwest`.
- Reject resolved IPs in: loopback (`127.0.0.0/8`, `::1`), link-local (`169.254.0.0/16`, `fe80::/10`), RFC 1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), APIPA.
- Use the `ipnet` crate for CIDR matching.

**Detection:** `crates/tools/src/web_fetch.rs:327`. Warning sign: no IP range validation before the `reqwest::Client::get()` call.

**Phase:** Security hardening. Medium complexity; requires DNS pre-resolution.

---

## Minor Pitfalls

Issues that are annoying but bounded in impact.

---

### Pitfall 15: Crossterm Version Mismatch Creates Silent Event Loss

**What goes wrong:** If a dependency upgrades `crossterm` to a major version different from the one ratatui uses, both versions coexist in the binary. Raw mode is tracked separately per version, so `disable_raw_mode()` from one version does not disable the mode set by the other. On exit, raw mode may not be fully restored, leaving the terminal corrupted.

**Prevention:** Pin crossterm version via ratatui's feature flags (`crossterm_0_28` etc.). Add `cargo deny` or `cargo tree` checks to CI that fail if two major versions of crossterm coexist.

**Detection:** Warning sign: `cargo tree -d | grep crossterm` shows multiple versions.

**Phase:** CI/dependency hygiene. Add a check once, never revisit.

---

### Pitfall 16: Windows Key Events Fire Twice — Duplicate Action Bug

**What goes wrong:** On Windows, crossterm emits `KeyEventKind::Press` and `KeyEventKind::Release` for every key. If the event handler does not filter to `KeyEventKind::Press`, every keystroke triggers the action twice.

**Prevention:** Filter at the event router: `if key.kind != KeyEventKind::Press { return; }`. Verify this filter is present in `crates/tui/src/app.rs`.

**Detection:** Windows users report that Enter submits twice, or Ctrl+C exits immediately without confirmation. Warning sign: missing `key.kind == KeyEventKind::Press` guard in the primary key dispatch match.

**Phase:** Cross-platform compatibility. Low effort, high impact for Windows users.

---

### Pitfall 17: Hardcoded Model Registry Goes Stale

**What goes wrong:** `crates/api/src/model_registry.rs` hard-codes model names, context windows, and pricing for all providers. When Anthropic or OpenAI releases a new model, users cannot access it without a code change and a new claurst release.

**Prevention:** Add a JSON manifest fetch from a versioned hosted URL on startup, with a 24-hour cache and fallback to the bundled snapshot on failure. For immediate relief, document how users can add custom model entries via config.

**Detection:** Warning sign: users file issues like "claude-opus-5 is not in the model list." The model registry was last audited 2026-05-04.

**Phase:** Low-priority feature polish. Can be deferred until after security and parity work.

---

### Pitfall 18: Cron Task Runaway Costs

**What goes wrong:** `crates/query/src/cron_scheduler.rs` spawns sub-agent API calls for every due cron task without failure tracking. A perpetually-failing task fires every minute, consuming API credits indefinitely.

**Prevention:** Track consecutive failures per cron entry; exponential backoff after 3 failures; disable after 10 consecutive failures with a user-visible warning in the TUI notification area.

**Detection:** Warning sign: API usage dashboard shows unexpected recurring charges at regular intervals with no active user session.

**Phase:** Cron/agent features phase. Address when cron scheduler is productionized.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| Security fixes (#123, #79, #96) | Incomplete path canonicalization for pre-creation paths (Pitfall 2) | Canonicalize parent dir + append filename |
| Security fixes | MCP project-config approval UI skipped in `--yes` mode (Pitfall 1) | Hardcode: no auto-approve for MCP server launch from project config |
| TUI mouse / scroll fix (#104) | Removing `EnableMouseCapture` globally breaks scroll (Pitfall 5) | Make mouse capture opt-in with fallback keyboard scroll |
| Feature parity — slash commands | Translation of concurrent TS async to sequential Rust async (Pitfall 7) | Audit every `await` chain for concurrency intent |
| Feature parity — tools | `file_edit.rs` has zero tests; a translation bug causes data loss | Add golden-file tests before any edit logic changes |
| Upstream sync | Merge conflict explosion in 8,576-line `lib.rs` (Pitfall 9) | Split module before next sync |
| Upstream sync | Upstream adds voice or computer-use code behind broken feature flags (Pitfall 10) | Add per-feature CI jobs |
| Async / query loop work | `select!` drops in-flight query results on concurrent terminal event (Pitfall 8) | Separate tasks + channels instead of large `select!` |
| CI expansion | `std::env::set_var` in parallel Tokio tests causes intermittent failures (Pitfall 6) | Add `#[serial]` or refactor to `Config` injection |
| Auth / OAuth work | CSRF via missing OAuth state validation (Pitfall 13) | Verify or implement state check before token exchange |
| WebFetch improvements | Unbounded buffering before size check (Pitfall 12) | Stream with chunk loop and byte counter |
| Plugin marketplace | `sh -c` hook execution with future user-editable hooks (Pitfall 4) | Enforce static-manifest-only trust boundary now |

---

## Sources

- Codebase audit: `.planning/codebase/CONCERNS.md` (2026-05-04) — HIGH confidence
- [EscapeRoute: CVE-2025-53109 & CVE-2025-53110 — Anthropic MCP Filesystem Sandbox Escape](https://cymulate.com/blog/cve-2025-53109-53110-escaperoute-anthropic/) — HIGH confidence
- [MCP STDIO transport command execution: 200,000 servers exposed](https://venturebeat.com/security/mcp-stdio-flaw-200000-ai-agent-servers-exposed-ox-security-audit) — HIGH confidence
- [A Timeline of MCP Security Breaches](https://authzed.com/blog/timeline-mcp-breaches) — MEDIUM confidence
- [Ratatui FAQ — common mistakes](https://ratatui.rs/faq/) — HIGH confidence
- [Ratatui Mouse Capture](https://ratatui.rs/concepts/backends/mouse-capture/) — HIGH confidence
- [Tokio select! cancellation safety](https://tokio.rs/tokio/tutorial/select) — HIGH confidence
- [Cancel safety in async Rust](https://sunshowers.io/posts/cancelling-async-rust/) — HIGH confidence
- [Migrating from TypeScript to Rust — corrode.dev](https://corrode.dev/learn/migration-guides/typescript-to-rust/) — MEDIUM confidence
- [Beyond Ctrl-C: Unix signal handling dark corners](https://sunshowers.io/posts/beyond-ctrl-c-signals/) — MEDIUM confidence
- [GitHub Blog: Strategies for friendly fork management](https://github.blog/2022-05-02-friend-zone-strategies-friendly-fork-management/) — MEDIUM confidence
- [Anthropic: making Claude Code more secure and autonomous](https://www.anthropic.com/engineering/claude-code-sandboxing) — MEDIUM confidence

---

*Pitfalls audit: 2026-05-04*
