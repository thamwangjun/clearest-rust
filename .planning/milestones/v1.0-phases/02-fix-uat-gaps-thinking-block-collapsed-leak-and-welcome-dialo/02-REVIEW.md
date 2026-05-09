---
phase: 02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo
reviewed: 2026-05-05T00:00:00Z
depth: standard
files_reviewed: 5
files_reviewed_list:
  - crates/cli/src/main.rs
  - crates/tui/src/messages/mod.rs
  - crates/tui/src/render.rs
  - crates/tui/tests/render_snapshots.rs
  - crates/tui/tests/startup_routing.rs
findings:
  critical: 2
  warning: 6
  info: 4
  total: 12
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-05-05
**Depth:** standard
**Files Reviewed:** 5
**Status:** issues_found

## Summary

The implementation targets three UAT gaps: thinking-block collapsed-mode content leak (D-01), the welcome/onboarding dialog routing to the wrong page on `show()` (D-07), and related rendering regressions. The core thinking-block fix in `render_thinking_block` is correct and confirmed by the snapshot test. The startup routing fix in `startup_routing.rs` is minimal and asserts the right invariant.

However, two security/correctness issues were found — one a bridge permission response that silently swallows the actual allow/deny decision without forwarding it to the tool, and one a truncation bug in the history-search overlay that can slice multi-byte UTF-8 strings mid-codepoint. Six additional warnings cover logic gaps and dead/duplicate code.

---

## Critical Issues

### CR-01: Bridge `PermissionResponse` handler ignores the decision — tool blocks forever

**File:** `crates/cli/src/main.rs:2668-2679`

**Issue:** When a remote `TuiBridgeEvent::PermissionResponse` arrives, the code clears `app.permission_request` but **never sends the decision through the `pending_permissions` channel**. The pending tool-use entry is left in `pending_permissions.lock().waiting` with its `decision_tx` unsignalled. The spawned query task is waiting on that channel; it will block indefinitely (or until the OS kills the process), and the agent appears to hang.

The `_allow` variable is computed but intentionally discarded (note the `_` prefix). This is the path that was left unimplemented.

```rust
// current (broken):
Ok(TuiBridgeEvent::PermissionResponse { tool_use_id, response }) => {
    if let Some(ref pr) = app.permission_request {
        if pr.tool_use_id == tool_use_id {
            use claurst_bridge::PermissionResponseKind;
            let _allow = matches!(
                response,
                PermissionResponseKind::Allow | PermissionResponseKind::AllowSession
            );
            app.permission_request = None;
            // BUG: nothing is ever sent on decision_tx!
        }
    }
}
```

**Fix:**

```rust
Ok(TuiBridgeEvent::PermissionResponse { tool_use_id, response }) => {
    use claurst_bridge::PermissionResponseKind;
    // Clear the UI dialog regardless of ID match so we don't leave a stale dialog.
    if let Some(ref pr) = app.permission_request {
        if pr.tool_use_id == tool_use_id {
            app.permission_request = None;
        }
    }
    // Forward the decision to the waiting tool task.
    if let Some(mut pending) = pending_permissions.lock().waiting.remove(&tool_use_id) {
        let decision = match response {
            PermissionResponseKind::Allow | PermissionResponseKind::AllowSession =>
                claurst_core::permissions::PermissionDecision::Allow,
            _ => claurst_core::permissions::PermissionDecision::Deny,
        };
        if let Some(tx) = pending.decision_tx.take() {
            let _ = tx.send(decision);
        }
    }
}
```

---

### CR-02: History search truncates multi-byte UTF-8 strings at a byte offset, causing a panic

**File:** `crates/tui/src/render.rs:2485-2489`

**Issue:** The history search truncation uses `s.truncate(dialog_width as usize - 9)` which operates on **byte position**, not character boundary. If the entry string contains any non-ASCII text (emoji, accented characters, CJK, etc.) and the truncation point lands inside a multi-byte codepoint, Rust will panic at runtime with `byte index N is not a char boundary`.

```rust
// current (panics on multi-byte input):
let truncated = if UnicodeWidthStr::width(entry) > (dialog_width as usize - 6) {
    let mut s = entry.to_string();
    s.truncate(dialog_width as usize - 9);   // <-- byte index, not char boundary
    format!("{}\u{2026}", s)
} else {
    entry.to_string()
};
```

The `truncate_end` helper already defined in the same file handles Unicode correctly. Use it.

**Fix:**

```rust
let truncated = if UnicodeWidthStr::width(entry) > (dialog_width as usize - 6) {
    truncate_end(entry, dialog_width as usize - 9)
} else {
    entry.to_string()
};
```

---

## Warnings

### WR-01: `is_modal_open` checks `import_config_dialog.visible` twice

**File:** `crates/tui/src/render.rs:113-118`

**Issue:** `app.import_config_dialog.visible` appears at both line 113 and line 118, making it impossible to reason about which modal combination is being excluded and silently allowing one of the guards to be wrong.

```rust
|| app.import_config_dialog.visible   // line 113
...
|| app.import_config_picker.visible
|| app.import_config_dialog.visible   // line 118 — duplicate
```

**Fix:** Remove the second occurrence at line 118. If `app.import_config_picker` was intended to be checked there, verify it is already covered at line 117 and remove only the duplicate.

---

### WR-02: `truncate_user_prompt_text` counts hidden lines incorrectly

**File:** `crates/tui/src/messages/mod.rs:1339-1344`

**Issue:** The `hidden_lines` counter subtracts the newlines in `tail` from the newlines in `head`, not from the newlines in the omitted middle. This produces a wrong (potentially negative-before-saturate) count when the tail contains more newlines than the head. The resulting `+{hidden_lines} lines` banner will be misleading.

```rust
let hidden_lines = text
    .chars()
    .take(TRUNCATE_USER_PROMPT_HEAD_CHARS)
    .filter(|c| *c == '\n')
    .count()
    .saturating_sub(tail.chars().filter(|c| *c == '\n').count()); // wrong
```

The correct count is the newlines in the omitted middle section:

**Fix:**

```rust
let head_newlines = text.chars().take(TRUNCATE_USER_PROMPT_HEAD_CHARS)
    .filter(|c| *c == '\n').count();
let tail_newlines = tail.chars().filter(|c| *c == '\n').count();
let total_newlines = text.chars().filter(|c| *c == '\n').count();
let hidden_lines = total_newlines.saturating_sub(head_newlines + tail_newlines);
```

---

### WR-03: `render_welcome_box` always sets `box_width = area.width` — dead calculation

**File:** `crates/tui/src/render.rs:1357`

**Issue:** `let box_width = area.width.min(area.width)` always equals `area.width`. The `.min(area.width)` is a no-op and the comment says "at most the full area width" but the code cannot be less than `area.width` either. This looks like a placeholder that was never finished and may hide an intended constraint (e.g. `min(80)` to cap the welcome box width for very wide terminals).

**Fix:** Either remove the `min` (if unconstrained width is intentional) or apply the actual desired maximum:

```rust
let box_width = area.width.min(120); // or just: let box_width = area.width;
```

---

### WR-04: `spawn_models_cache_refresh` is called twice in `run_interactive`

**File:** `crates/cli/src/main.rs:940,1396`

**Issue:** `spawn_models_cache_refresh()` is called once inside `refresh_provider_runtime_state` (line 940) and again unconditionally inside `run_interactive` at startup (line 1396), regardless of whether a refresh was already triggered. On normal startup this doubles the outbound HTTP requests to models.dev and writes the same data to the two cache files twice in quick succession.

**Fix:** Move the startup call before `refresh_provider_runtime_state` is called, or add a guard (e.g. `OnceLock`) so the fetch fires at most once per process.

---

### WR-05: `InboundPrompt` bridge path skips the `UserPromptSubmit` hook

**File:** `crates/cli/src/main.rs:2616-2656`

**Issue:** When a remote `InboundPrompt` triggers a query turn, it bypasses the `UserPromptSubmit` hook block that runs for keyboard-submitted messages (around line 2085). Hook-based workflows (e.g. logging, input sanitisation) that fire on normal user input will not execute for remote prompts. This is likely unintentional given the hooks are registered for that event.

**Fix:** Extract the hook-execution logic into a helper or call it in both code paths before pushing the user message and starting the query.

---

### WR-06: `render_thinking_block` snapshot test does not verify absence of thinking content in search_text

**File:** `crates/tui/tests/render_snapshots.rs:159-165`

**Issue:** The `thinking_block_collapsed` test asserts that `lines.len() == 1` and that the rendered text contains "Thinking" but not "hidden thoughts". However, `RenderedLineItem::search_text` (used for global search matching) is populated from `flatten_line_text` which uses the same rendered spans. The test does not verify that `search_text` on a collapsed thinking item doesn't contain the thinking content — the only way to confirm the leak is truly plugged at the search-indexing layer. If a future change re-adds span content to collapsed blocks, search would expose the thoughts while the visual test still passes.

**Fix:** Add an assertion that the search text (or an equivalent flat string from the rendered line) does not contain "hidden thoughts":

```rust
// Verify no thinking content bleeds into global-search indexable text
assert!(!text.contains("hidden"), "collapsed thinking must not leak to search text");
```

---

## Info

### IN-01: `CLAUDE_ORANGE` colour constant is defined in both `messages/mod.rs` and `render.rs`

**File:** `crates/tui/src/messages/mod.rs:66`, `crates/tui/src/render.rs:71`

**Issue:** Both files independently define `const CLAUDE_ORANGE: Color = Color::Rgb(233, 30, 99)`. If the colour is ever updated, both must be changed in sync. Extract to a shared `theme.rs` or `constants.rs` within the `tui` crate.

---

### IN-02: `bare_name` is computed but unused in `McpToolWrapper::execute`

**Resolution: not applicable**

**File:** `crates/cli/src/main.rs:85-90,103`

**Issue:** `bare_name` strips the server-name prefix from the tool name for use in the error message, but the `Err` branch at line 103 correctly uses `bare_name`. However at line 94, `call_tool` is passed `&self.tool_def.name` (the full prefixed name) rather than `bare_name`. If MCP server implementations expect the bare name in their dispatch, this will cause tool-not-found errors. Worth confirming the MCP server contract.

**Resolution (not applicable):** `McpManager::call_tool` (`crates/mcp/src/lib.rs:1034`) accepts the full prefixed name and strips the server-name prefix internally before forwarding the bare name to `McpClient::call_tool`. Passing `&self.tool_def.name` (the prefixed name) at line 95 is therefore correct. The `bare_name` local variable is correctly scoped to the `Err` branch error message at line 104. No code change required.

---

### IN-03: `STATUS_COMMAND` environment variable shell injection vector (low-severity, mitigated)

**File:** `crates/cli/src/main.rs:1480-1490`

**Issue:** The contents of `CLAUDE_STATUS_COMMAND` are passed literally to `sh -c` / `cmd /C` without any sanitisation. A value containing shell metacharacters (`;`, `|`, `$(…)`) would execute arbitrary commands. Since the variable must be set by the user running the process this is self-inflicted — but if the value is ever populated from a settings file or network source the risk escalates. At minimum, document the caveat in the `--help` or nearby comments.

---

### IN-04: Named command "fall-through" returns `Ok(())` after intercepting an unrecognised result variant

**File:** `crates/cli/src/main.rs:421-424`

**Issue:** When a named command returns a result variant other than `Message`, `UserMessage`, or `Error`, the code falls through the match and then returns `Ok(())` unconditionally, without attempting normal startup. Any `CommandResult` that isn't one of those three variants (e.g. a future `ConfigChange` or `Exit`) will silently exit the process without warning, making it hard to diagnose.

```rust
_ => {
    // For any other result variant, fall through to normal startup
}
```

Then immediately after the if-let block: `return Ok(());`

This means _any_ named command result causes early return — the "fall through" comment is misleading. If the intent is that `Silent`/`None` should continue to normal startup, the `return Ok(())` must be moved inside the `is_some()` branches.

---

_Reviewed: 2026-05-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
