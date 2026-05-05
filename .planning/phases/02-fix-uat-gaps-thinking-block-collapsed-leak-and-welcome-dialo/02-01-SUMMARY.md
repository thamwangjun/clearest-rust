---
phase: 02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo
plan: "01"
subsystem: tui/messages
tags: [bug-fix, tui, thinking-block, animation, render-context]
dependency_graph:
  requires: []
  provides: [fixed-thinking-block-collapsed-render]
  affects: [crates/tui/src/messages/mod.rs, crates/tui/src/render.rs]
tech_stack:
  added: []
  patterns: [frame-count-animation, collapsed-thinking-dots]
key_files:
  created: []
  modified:
    - crates/tui/src/messages/mod.rs
    - crates/tui/src/render.rs
    - crates/tui/tests/render_snapshots.rs
decisions:
  - Animated dots use formula ((frame_count / 4) % 3) + 1 to cycle 1-3 dots every 4 frames, matching shimmer_spans cadence already in render.rs
  - Collapsed branch completely avoids reasoning_heading call — no content from thinking text can appear in output
metrics:
  duration: "~8 minutes"
  completed: "2026-05-05T11:03:57Z"
  tasks_completed: 1
  tasks_total: 1
---

# Phase 02 Plan 01: Fix Thinking Block Collapsed Content Leak Summary

**One-liner:** Fixed collapsed thinking block content leak by replacing reasoning_heading call with frame-count-driven animated dots (1-3 dots, cycling every 4 frames).

## What Was Built

Added `frame_count: u64` field to `RenderContext` struct and rewrote `render_thinking_block` to:
- **Collapsed mode:** Show animated dots (`.` / `..` / `...`) derived from `((frame_count / 4) % 3) + 1` — no content from the `text` parameter appears in output (D-01 mitigation)
- **Expanded mode:** Unchanged behavior — calls `reasoning_heading(text)` and emits body lines

Threaded `frame_count` through both `RenderContext` construction sites in `render.rs` and updated the `render_message` call site to pass `ctx.frame_count` as the third argument to `render_thinking_block`.

## Tasks Completed

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add frame_count to RenderContext, fix render_thinking_block, update all call sites and tests | 7a26471 | crates/tui/src/messages/mod.rs, crates/tui/src/render.rs, crates/tui/tests/render_snapshots.rs |

## Verification Results

```
cargo test -p claurst-tui
test result: ok. 26 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

- `thinking_block_collapsed` — PASS (was previously FAIL: text.contains("hidden thoughts") was true)
- `thinking_block_expanded` — PASS
- All 27 tests in claurst-tui suite pass (26 render_snapshots + 1 startup_routing)

## Deviations from Plan

None — plan executed exactly as written.

## Known Stubs

None. The animated dots are real output, not placeholder — they derive from the live `frame_count` value threaded through from application state.

## Threat Flags

None. The fix IS the T-02-01 mitigation (stop calling reasoning_heading in collapsed mode). T-02-02 and T-02-03 accepted as documented in the threat register.

## Self-Check: PASSED

- `crates/tui/src/messages/mod.rs` contains `pub frame_count: u64` — FOUND
- `crates/tui/src/render.rs` contains `frame_count,` and `frame_count: app.frame_count,` — FOUND
- `crates/tui/tests/render_snapshots.rs` calls render_thinking_block with 3 args — FOUND
- Commit 7a26471 exists — FOUND
- Both target tests pass — CONFIRMED
