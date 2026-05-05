# Phase 2: Fix UAT gaps — thinking block collapsed leak and welcome dialog startup routing - Context

**Gathered:** 2026-05-05
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix two pre-existing bugs surfaced by Phase 1 UAT. No new features. Scope is exactly the two UAT gaps:
1. `thinking_block_collapsed` test fails because `render_thinking_block` leaks content text in collapsed mode.
2. First-run no-credentials users see "Connect A Provider" instead of the Welcome page because `main.rs:1435` calls `show_provider_setup()` instead of `show()`.

</domain>

<decisions>
## Implementation Decisions

### Collapsed thinking heading
- **D-01:** In collapsed mode (`expanded = false`), `render_thinking_block` must NOT call `reasoning_heading(text)`. Instead, render an animated `...` placeholder that cycles through `.` → `..` → `...` every 4 frames (~250ms at 60fps). The heading span shows `"..."` (current dot count), not any content derived from the thinking text.
- **D-02:** Animation is driven by `frame_count`. Add `frame_count: u64` (default 0) to `RenderContext`. Update both `RenderContext` construction sites in `render.rs` to supply `app.frame_count`. Pass `ctx.frame_count` through to `render_thinking_block` as a new parameter.
- **D-03:** The existing `thinking_block_collapsed` test in `render_snapshots.rs` must be updated to pass `frame_count = 0` (or equivalent) so it still asserts `contains("Thinking")` and `!contains("hidden thoughts")`. At frame 0, dots cycle to `"."` which satisfies both assertions.

### Welcome dialog startup routing
- **D-04:** Change `main.rs:1435` from `app.onboarding_dialog.show_provider_setup()` to `app.onboarding_dialog.show()`. One line. No other call sites touched.
- **D-05:** After the Welcome → KeyBindings → dismiss flow completes, a no-credentials user sees the existing status hint: `"No provider configured. Run /connect to set one up."` — same as the `else` branch that fires when `has_completed_onboarding = true` and no credentials. No new behavior, no auto-opening of ProviderSetup.

### Test coverage
- **D-06:** Fix the existing failing `thinking_block_collapsed` test (render_snapshots.rs line 159) — update its `render_thinking_block` call to include the new `frame_count` parameter.
- **D-07:** Add a new test file `crates/tui/tests/startup_routing.rs`. It must contain at least one test that:
  - Calls `onboarding_dialog.show()` directly
  - Asserts the dialog is visible and the starting page is Welcome (not ProviderSetup)
  This confirms the correct starting state that `main.rs:1435` should produce after the fix.

### Claude's Discretion
- Exact cycling formula for dots (`(frame_count / 4) % 3` or similar — planner picks the cleanest arithmetic)
- Whether `frame_count` is added directly to `render_thinking_block`'s parameter list or accessed via `RenderContext` — whichever requires fewer call-site changes
- Exact span styling for the animated `"..."` (match existing DarkGray italic or a slight variation)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Collapsed thinking render
- `crates/tui/src/messages/mod.rs` §`render_thinking_block` (line ~1245) — current implementation that calls `reasoning_heading` unconditionally; this is what needs fixing
- `crates/tui/src/transcript_turn.rs` §`reasoning_heading` — function that leaks content text; do NOT modify it, just stop calling it in collapsed mode
- `crates/tui/tests/render_snapshots.rs` §`thinking_block_collapsed` (line 159) — failing test that must pass after the fix
- `crates/tui/src/messages/mod.rs` §`RenderContext` (line 30) — struct to add `frame_count: u64` field to

### Frame count threading
- `crates/tui/src/render.rs` §`render_live_thinking_lines` (line ~1072) — reference for how `shimmer_spans` uses `frame_count`; follow the same pattern
- `crates/tui/src/render.rs` — two `RenderContext { ... }` construction sites (lines ~1115 and ~1271) that must include `frame_count: app.frame_count`

### Welcome routing
- `crates/cli/src/main.rs` lines 1430–1443 — startup routing block; line 1435 is the bug (`show_provider_setup()` → `show()`)
- `crates/tui/src/onboarding_dialog.rs` — `show()` vs `show_provider_setup()` implementations; confirms `show()` starts at the Welcome page

### Phase 1 decisions (locked)
- `.planning/phases/01-welcome-screen-fix/01-CONTEXT.md` — D-01 through D-06; especially D-01 (show() is correct entry) and D-06 (regression test requirements)
- `.planning/phases/01-welcome-screen-fix/01-UAT.md` — root cause analysis for both bugs (authoritative source of truth for what's broken and why)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `shimmer_spans(text, frame_count)` in `render.rs` (line ~1858): existing frame_count animation utility. The dots animation can use the same `frame_count % N` arithmetic directly, without needing shimmer_spans itself.
- `RenderContext` struct: already the threading mechanism for rendering options into `render_message`. Adding `frame_count: u64` here is the minimal-surface change.
- `thinking_block_collapsed` test: exists and is exactly targeted. Fixing it is the goal, not replacing it.

### Established Patterns
- `render_thinking_block` is a standalone `pub fn` called from `render_message(msg, ctx: &RenderContext)`. The cleanest path is to either add `frame_count` as a direct parameter or thread it through `ctx`.
- Both `RenderContext` construction sites in `render.rs` are struct literals with explicit fields. Adding `frame_count: app.frame_count` at each site is a 1-line change per site.
- `startup_routing.rs` test should follow the same pattern as `render_snapshots.rs` — direct unit test against public TUI functions, no full App init needed.

### Integration Points
- `messages/mod.rs` → `render.rs` → `app.rs`: frame_count flows from `App.frame_count` through render into message rendering. The new `RenderContext.frame_count` field bridges this gap.
- `main.rs` startup block (lines 1430–1443): the `show_provider_setup()` call is in a condition that won't be touched by any other phase change. Safe to fix in isolation.

</code_context>

<deferred>
## Deferred Ideas

- Auto-opening ProviderSetup after the Welcome→KeyBindings flow completes — declined, status hint is sufficient
- Auditing all `show_provider_setup()` call sites — out of scope, targeted fix only
- Replacing `reasoning_heading()` with a smarter summary for collapsed mode — out of scope; animated `...` is the chosen approach

</deferred>

---

*Phase: 02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo*
*Context gathered: 2026-05-05*
