---
phase: 02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo
plan: "02"
subsystem: tui/onboarding
tags: [bug-fix, startup-routing, onboarding, regression-test]
dependency_graph:
  requires: []
  provides: [welcome-dialog-startup-routing-fix, startup-routing-regression-test]
  affects: [crates/cli/src/main.rs, crates/tui/tests/startup_routing.rs]
tech_stack:
  added: []
  patterns: [unit-test-without-app-init, direct-state-method-test]
key_files:
  created:
    - crates/tui/tests/startup_routing.rs
  modified:
    - crates/cli/src/main.rs
decisions:
  - "Used direct OnboardingDialogState::new().show() unit test — no App init needed; tests the dialog state in isolation"
  - "One-word change in main.rs: show_provider_setup() → show() at line 1435"
metrics:
  duration_minutes: 15
  completed: "2026-05-05T11:03:53Z"
  tasks_completed: 2
  files_changed: 2
---

# Phase 02 Plan 02: Welcome Dialog Startup Routing Fix Summary

Fixed startup routing bug (Bug 2 from Phase 1 UAT): `main.rs:1435` now calls `show()` instead of `show_provider_setup()`, routing first-run no-credentials users to the Welcome page.

## What Was Done

### Task 1: Create startup_routing.rs regression test
Created `crates/tui/tests/startup_routing.rs` with `fn show_starts_at_welcome_page` that:
- Calls `OnboardingDialogState::new().show()` directly (no App init required)
- Asserts `dialog.visible == true` and `dialog.page == OnboardingPage::Welcome`
- Serves as a regression guard for the `show()` method's page-routing behavior

Test passes immediately because `show()` in `onboarding_dialog.rs` was already correctly implemented — it was only the call site in `main.rs` that was wrong.

### Task 2: Fix startup routing in main.rs
Changed one method call at `crates/cli/src/main.rs:1435`:
- Before: `app.onboarding_dialog.show_provider_setup();`
- After: `app.onboarding_dialog.show();`

`show_provider_setup()` sets `page = OnboardingPage::ProviderSetup`, causing first-run no-credentials users to skip the Welcome page and land directly on provider setup. `show()` correctly sets `page = OnboardingPage::Welcome`.

The surrounding `if !has_credentials` condition and the `else` branch (status_message hint) were left completely unchanged (D-05 requirement).

## Verification Results

- `grep -c "show_provider_setup" crates/cli/src/main.rs` → 0 (bug line eliminated)
- `grep -c "onboarding_dialog.show()" crates/cli/src/main.rs` → 1 (fix confirmed)
- `cargo test -p claurst-tui show_starts_at_welcome_page` → PASS
- `cargo build -p claurst` → SUCCESS (0 errors)

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| Task 1 | abdcbc1 | test(02-02): add startup_routing regression test for show() → Welcome page |
| Task 2 | 0e10d15 | fix(02-02): change show_provider_setup() to show() in startup routing |

## Deviations from Plan

None — plan executed exactly as written.

The `cargo test -p claurst-tui` full suite has a pre-existing failure (`thinking_block_collapsed` in `render_snapshots.rs`) that is addressed by the parallel Plan 02-01 agent. This pre-existed before this plan's changes and is out of scope for this plan.

## Known Stubs

None — both changes are complete and wire to real behavior.

## Threat Flags

No new security-relevant surface introduced. The fix only changes which UI page (Welcome vs ProviderSetup) is shown for first-run no-credentials users. The credential check (`!has_credentials`) is unmodified. No network endpoints, auth paths, or schema changes were introduced.

## Self-Check: PASSED

- [x] `crates/tui/tests/startup_routing.rs` exists — FOUND
- [x] `crates/cli/src/main.rs` fix applied — CONFIRMED (grep returns 0 for show_provider_setup, 1 for show())
- [x] Commit abdcbc1 exists — CONFIRMED (git log)
- [x] Commit 0e10d15 exists — CONFIRMED (git log)
- [x] `cargo build -p claurst` exits 0 — CONFIRMED
- [x] `cargo test -p claurst-tui show_starts_at_welcome_page` exits 0 — CONFIRMED
