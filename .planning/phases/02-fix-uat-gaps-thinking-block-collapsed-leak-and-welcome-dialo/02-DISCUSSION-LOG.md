# Phase 2: Fix UAT gaps — thinking block collapsed leak and welcome dialog startup routing - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-05
**Phase:** 02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo
**Areas discussed:** Collapsed heading label, Welcome routing fix scope, Test coverage

---

## Collapsed heading label

| Option | Description | Selected |
|--------|-------------|----------|
| Static 'Thinking' only | Remove reasoning_heading() call entirely. Render just "Thinking: " with no heading suffix. | |
| Elided summary | Still call reasoning_heading() but replace returned text with a placeholder when expanded=false. | ✓ |
| You decide | Either approach is fine — Claude picks simplest path. | |

**User's choice:** Elided summary

**Follow-up — placeholder content:**

| Option | Description | Selected |
|--------|-------------|----------|
| "Thinking: ..." (static) | Replace reasoning_heading() result with static "..." | |
| "Thinking" (no suffix) | Empty heading span | |
| Animated "..." | Cycling dots (. → .. → ...) driven by frame_count | ✓ |

**User's choice (freeform):** "Thinking: ... with the ... animated appropriately."

**Follow-up — animation speed:**

| Option | Description | Selected |
|--------|-------------|----------|
| Every 4 frames (~250ms at 60fps) | Matches rustle blink cadence | ✓ |
| Every 8 frames (~500ms at 60fps) | Slower, more relaxed | |
| You decide | Pick based on existing codebase cadence | |

**User's choice:** Every 4 frames (~250ms at 60fps)

**Notes:** Animation requires threading `frame_count` through `RenderContext` and updating `render_thinking_block` signature. Both `RenderContext` construction sites in `render.rs` will supply `app.frame_count`. The existing `thinking_block_collapsed` test will be updated to pass `frame_count = 0`.

---

## Welcome routing fix scope

| Option | Description | Selected |
|--------|-------------|----------|
| Status hint only | After dismiss, set status_message "No provider configured. Run /connect to set one up." | ✓ |
| Auto-open ProviderSetup after dismiss | Automatically show ProviderSetup dialog after Welcome→KeyBindings flow | |
| Nothing extra | Just land on main TUI | |

**User's choice:** Status hint only (Recommended)

**Follow-up — audit other show_provider_setup() call sites:**

| Option | Description | Selected |
|--------|-------------|----------|
| No — targeted fix only | Only fix line 1435 | ✓ |
| Yes — audit all call sites | grep for show_provider_setup() and verify each | |

**User's choice:** No — targeted fix only

**Notes:** One-line fix: `show_provider_setup()` → `show()` at main.rs:1435. Status hint behavior already exists in the else branch for has_completed_onboarding=true + no credentials — no new code needed for the post-dismiss state.

---

## Test coverage

| Option | Description | Selected |
|--------|-------------|----------|
| Unit test: show() called, not show_provider_setup() | New test asserting Welcome page is start | ✓ |
| Integration test: full startup flow | Full main.rs startup path with mock App | |
| None — rely on UAT re-run | Skip automated tests | |

**User's choice:** Unit test: show() called, not show_provider_setup()

**Follow-up — test location:**

| Option | Description | Selected |
|--------|-------------|----------|
| onboarding_dialog.rs | Add to existing #[cfg(test)] block | |
| New file: tests/startup_routing.rs | Dedicated test file alongside render_snapshots.rs | ✓ |
| You decide | Planner picks location per existing conventions | |

**User's choice:** New file: tests/startup_routing.rs

---

## Claude's Discretion

- Exact cycling formula for animated dots
- Whether frame_count is a direct parameter or accessed via RenderContext (whichever requires fewer call-site changes)
- Exact span styling for the "..." placeholder

## Deferred Ideas

- Auto-opening ProviderSetup after Welcome→KeyBindings flow — declined
- Auditing all show_provider_setup() call sites — out of scope
- Smarter collapsed mode summary from reasoning_heading() — out of scope; animated "..." chosen
