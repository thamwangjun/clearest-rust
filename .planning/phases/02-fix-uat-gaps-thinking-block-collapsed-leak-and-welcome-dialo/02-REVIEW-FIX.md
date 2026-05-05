---
phase: 02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo
fixed_date: 2026-05-05
findings_in_scope: 12
fixed: 11
skipped: 1
status: partial
iteration: 1
---

# Phase 02: Code Review Fix Report

**Fixed at:** 2026-05-05
**Source review:** .planning/phases/02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo/02-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 12
- Fixed: 11
- Skipped: 1

## Fixed Issues

### CR-01: Bridge `PermissionResponse` handler ignores the decision

**Files modified:** `crates/cli/src/main.rs`
**Commit:** ea033ab
**Applied fix:** Removed the unused `_allow` binding. After clearing the UI dialog, the handler now calls `pending_permissions.lock().waiting.remove(&tool_use_id)` and sends the correct `PermissionDecision` (Allow or Deny) on `pending.decision_tx` — unblocking the waiting tool task.

---

### CR-02: History search truncates multi-byte UTF-8 strings at a byte offset

**Files modified:** `crates/tui/src/render.rs`
**Commit:** 66cd0d0
**Applied fix:** Replaced the `s.truncate(...)` + `format!("{}\u{2026}", s)` pattern with a call to the existing `truncate_end()` helper, which iterates by character width and appends the ellipsis safely, preventing the UTF-8 boundary panic.

---

### WR-01: `is_modal_open` checks `import_config_dialog.visible` twice

**Files modified:** `crates/tui/src/render.rs`
**Commit:** 68e021a
**Applied fix:** Removed the second `|| app.import_config_dialog.visible` occurrence (the one that appeared after `app.import_config_picker.visible`). The first occurrence at line 113 remains.

---

### WR-02: `truncate_user_prompt_text` counts hidden lines incorrectly

**Files modified:** `crates/tui/src/messages/mod.rs`
**Commit:** c97a00e
**Applied fix:** Replaced the subtraction of tail newlines from head newlines with the correct formula: compute head_newlines, tail_newlines, and total_newlines separately, then `hidden_lines = total.saturating_sub(head + tail)`. This gives the actual count of newlines in the omitted middle section.

---

### WR-03: `render_welcome_box` always sets `box_width = area.width` — dead calculation

**Files modified:** `crates/tui/src/render.rs`
**Commit:** 7281f6a
**Applied fix:** Changed `area.width.min(area.width)` to `area.width.min(120)`, applying a meaningful 120-column cap on wide terminals and updating the comment to explain the intent.

---

### WR-04: `spawn_models_cache_refresh` is called twice in `run_interactive`

**Files modified:** `crates/cli/src/main.rs`
**Commit:** fb79dba
**Applied fix:** Removed the unconditional `spawn_models_cache_refresh()` call inside `run_interactive` (the block at the former line 1396). The function is already invoked inside `refresh_provider_runtime_state`. A comment replaces the removed call to explain why it was removed.

---

### WR-05: `InboundPrompt` bridge path skips the `UserPromptSubmit` hook

**Files modified:** `crates/cli/src/main.rs`
**Commit:** ce21478
**Applied fix:** Added a `UserPromptSubmit` hook invocation inside the `TuiBridgeEvent::InboundPrompt` handler, mirroring the hook block that exists for keyboard-submitted messages. The hook fires before the user message is pushed to the conversation.

---

### WR-06: `render_thinking_block` snapshot test does not verify search_text

**Files modified:** `crates/tui/tests/render_snapshots.rs`
**Commit:** ad69f32
**Applied fix:** Added `assert!(!text.contains("hidden"), "collapsed thinking must not leak to search text")` to the `thinking_block_collapsed` test. This catches any future regression where collapsed block spans re-appear in the flattened text used by global search.

---

### IN-01: `CLAUDE_ORANGE` colour constant defined in both `messages/mod.rs` and `render.rs`

**Files modified:** `crates/tui/src/theme_colors.rs`, `crates/tui/src/messages/mod.rs`, `crates/tui/src/render.rs`
**Commit:** cb4cf72
**Applied fix:** Added `pub const CLAUDE_ORANGE: Color = Color::Rgb(233, 30, 99)` to the existing `theme_colors` module. Both `messages/mod.rs` and `render.rs` now import it with `use crate::theme_colors::CLAUDE_ORANGE` — their local definitions were removed.

---

### IN-03: `STATUS_COMMAND` environment variable shell injection vector

**Files modified:** `crates/cli/src/main.rs`
**Commit:** b244127
**Applied fix:** Added a `SECURITY NOTE` comment before the `CLAUDE_STATUS_COMMAND` usage block explaining that shell metacharacters in the value are interpreted by the shell, that this is intentional for user-set values, and that the variable must be validated/escaped if it ever comes from an external or untrusted source.

---

### IN-04: Named command fall-through returns `Ok(())` after intercepting unrecognised result variant

**Files modified:** `crates/cli/src/main.rs`
**Commit:** 73a61de
**Applied fix:** Removed the `return Ok(())` that appeared after the match block (causing all `_` arms to silently exit). Added an explicit `CommandResult::Silent` arm that allows control to fall through to normal startup. The remaining `_` wildcard arm now prints a diagnostic warning and calls `std::process::exit(0)` rather than vanishing silently.

---

## Skipped Issues

### IN-02: `bare_name` is computed but `call_tool` passed full prefixed name

**File:** `crates/cli/src/main.rs:85-90,103`
**Reason:** This is a contract question, not an obvious bug. The reviewer notes "worth confirming the MCP server contract". Changing `&self.tool_def.name` to `bare_name` in the `call_tool` invocation without knowing whether the MCP server dispatch expects the bare or prefixed name risks introducing tool-not-found errors. Skipped pending explicit confirmation of the MCP dispatch contract.
**Original issue:** `call_tool` is passed the full prefixed name (`server_foo_bar`) while `bare_name` (`bar`) is computed but only used in the error message branch. If MCP servers expect the bare name in dispatch this causes tool-not-found errors.

---

_Fixed: 2026-05-05_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
