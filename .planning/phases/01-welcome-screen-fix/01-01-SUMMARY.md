---
phase: "01-welcome-screen-fix"
plan: "01"
subsystem: "tui/onboarding"
tags: ["bug-fix", "tdd", "onboarding", "regression-tests"]
dependency_graph:
  requires: []
  provides: ["BUG-01 regression guard", "onboarding_defaults_hidden fix"]
  affects: ["src-rust/crates/tui/src/onboarding_dialog.rs", "src-rust/crates/tui/src/app.rs"]
tech_stack:
  added: []
  patterns: ["TDD RED/GREEN", "unit test assertions", "ratatui test backend"]
key_files:
  created: []
  modified:
    - src-rust/crates/tui/src/onboarding_dialog.rs
    - src-rust/crates/tui/src/app.rs
decisions:
  - "D-01: show() call preserved as welcome page entry point"
  - "D-02: Page order Welcome -> KeyBindings -> Done unchanged"
  - "D-03: Enter while dialog visible is intercepted before any quit path"
  - "D-04: show_provider_setup() never called"
  - "D-05: No status message added after dialog dismissal"
  - "D-06: Three regression tests added to lock in Enter/Esc behavior"
metrics:
  duration: "~15 minutes"
  completed: "2026-05-05T07:17:01Z"
  tasks_completed: 2
  tasks_total: 2
---

# Phase 01 Plan 01: Fix onboarding_defaults_hidden and Add D-06 Regression Tests Summary

**One-liner:** Fixed wrong test assertion (`Welcome` -> `ProviderSetup`) and added three regression tests proving Enter/Esc on the onboarding dialog never sets `should_quit=true`.

## What Was Changed

### Task 1: Fix onboarding_defaults_hidden (dc691f8)

**File:** `src-rust/crates/tui/src/onboarding_dialog.rs`
**Line:** 390

The test `onboarding_defaults_hidden` was asserting `OnboardingPage::Welcome` on a freshly constructed `OnboardingDialogState::new()`. This was wrong because:
- `OnboardingPage`'s `#[default]` derive attribute is on `ProviderSetup` (line 26 of the file)
- `new()` calls `Self::default()` which produces `page = ProviderSetup`
- `show()` explicitly overrides `page = Welcome` — a separate call not made in `new()`

**Change:** `assert_eq!(state.page, OnboardingPage::Welcome)` -> `assert_eq!(state.page, OnboardingPage::ProviderSetup)`

No other lines were touched. The `assert!(!state.visible)` on the preceding line was already correct.

### Task 2: Add D-06 Regression Tests (f7cced5)

**File:** `src-rust/crates/tui/src/app.rs`
**Lines added:** 5919-5963 (three test functions + import)

**Import added** at line 5604 (inside `mod tests`):
```rust
use crate::onboarding_dialog::OnboardingPage;
```
(Required because `OnboardingPage` is not re-exported through `super::*` — deviation from plan that was caught at compile time and fixed inline.)

**Three tests added:**

1. `test_onboarding_enter_on_welcome_advances_page` — Calls `show()`, presses Enter, asserts:
   - `page == KeyBindings` (Welcome advanced)
   - `visible == true` (dialog still open)
   - `should_quit == false`
   - `handle_key_event` returns `false`

2. `test_onboarding_enter_on_keybindings_dismisses` — Calls `show()`, sets `page = KeyBindings`, presses Enter, asserts:
   - `visible == false` (dialog dismissed)
   - `should_quit == false`
   - `handle_key_event` returns `false`

3. `test_onboarding_esc_dismisses` — Calls `show()`, presses Esc, asserts:
   - `visible == false`
   - `should_quit == false`
   - `handle_key_event` returns `false`

## Test Results

**Before this plan:**
- `claurst-tui` lib tests: 488 passed, **1 failed** (`onboarding_defaults_hidden`)

**After this plan:**
- `claurst-tui` lib tests: **491 passed, 0 failed**
- Onboarding-specific tests: 10 passed (7 existing + 3 new), 0 failed

## Decisions Honored

| Decision | Status | Evidence |
|----------|--------|----------|
| D-01: show() preserved as entry point | Honored | `show()` called in all three new tests; `new()` alone does not set Welcome |
| D-02: Page order Welcome->KeyBindings->Done unchanged | Honored | test 1 asserts page advances to KeyBindings, not skipped |
| D-03: Enter intercepted before quit path | Honored | `should_quit == false` in all Enter/Esc tests |
| D-04: show_provider_setup() never called | Honored | No production code changed; guard at lines 2769-2786 confirmed correct |
| D-05: No status message added after dismissal | Honored | No production code changed |
| D-06: Three regression tests lock in behavior | Honored | All three tests pass |

## Runtime Behavior Observed

The production code in `handle_key_event` (lines 2769-2786) was **already correct**. The onboarding guard:
1. Intercepts when `onboarding_dialog.visible == true`
2. Routes Enter to `next_page()` which returns `false` for Welcome->KeyBindings and `true` for KeyBindings->Done
3. Returns `false` (not `should_quit = true`) unconditionally after handling

The bug was entirely in the test layer:
- One wrong assertion in `onboarding_defaults_hidden`
- Three missing regression tests that would have caught any future regression

No production code modifications were needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Missing import] Added `use crate::onboarding_dialog::OnboardingPage` to test module**
- **Found during:** Task 2 compilation
- **Issue:** Plan stated `OnboardingPage` would be in scope via `use super::*`. However, `OnboardingPage` is defined in `crate::onboarding_dialog` and not re-exported from `app.rs`'s module root, so `super::*` does not bring it into the test module's scope.
- **Fix:** Added `use crate::onboarding_dialog::OnboardingPage;` as the second import in `mod tests`
- **Files modified:** `src-rust/crates/tui/src/app.rs` (line 5604)
- **Commit:** f7cced5 (included in Task 2 commit)

## Known Stubs

None — no placeholder data, hardcoded empty values, or TODO markers introduced.

## Threat Flags

None — this plan adds test-only code. No new network endpoints, auth paths, file access patterns, or schema changes.

## Deferred Items

**Pre-existing workspace failure (out of scope):** `claurst-api::codex_adapter::tests::test_anthropic_to_openai_request_basic` fails due to floating-point precision (`0.699999988079071 != 0.7`). This failure exists in the main branch before this plan and is unrelated to onboarding. Logged in deferred-items.

## Self-Check: PASSED

Files modified:
- `src-rust/crates/tui/src/onboarding_dialog.rs` — FOUND
- `src-rust/crates/tui/src/app.rs` — FOUND
- `.planning/phases/01-welcome-screen-fix/01-01-SUMMARY.md` — FOUND

Commits:
- dc691f8 — fix(01-01): correct onboarding_defaults_hidden test assertion — FOUND
- f7cced5 — test(01-01): add D-06 regression tests for onboarding Enter/Esc behavior — FOUND
