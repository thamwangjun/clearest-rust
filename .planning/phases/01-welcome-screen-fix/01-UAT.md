---
status: complete
phase: 01-welcome-screen-fix
source:
  - .planning/phases/01-welcome-screen-fix/01-01-SUMMARY.md
started: 2026-05-05T00:00:00Z
updated: 2026-05-05T00:02:00Z
---

## Current Test

[testing complete]

## Tests

### 1. Test Suite Green
expected: Run `cargo test -p claurst-tui` from the src-rust directory. 491 tests pass, 0 fail. The previously-failing `onboarding_defaults_hidden` test is now green.
result: issue
reported: "thread 'thinking_block_collapsed' panicked at crates/tui/tests/render_snapshots.rs:164:5: assertion failed: !text.contains(\"hidden thoughts\"). 25 passed; 1 failed."
severity: major

### 2. Onboarding Enter Advances Pages
expected: Run the binary (`cargo run`). On the welcome/onboarding dialog, press Enter. The dialog advances from the Welcome page to KeyBindings — the app does NOT quit, and the dialog remains visible on KeyBindings.
result: issue
reported: "Welcome dialog does not appear. Connect A Provider dialog appears instead."
severity: major

### 3. Onboarding Enter Dismisses on Last Page
expected: With the onboarding dialog on the KeyBindings page, press Enter. The dialog closes/dismisses — the app does NOT quit and continues to the main screen.
result: skipped
reason: Welcome dialog not shown — Connect A Provider dialog appears instead, blocking navigation tests

### 4. Onboarding Esc Dismisses Without Quitting
expected: With the onboarding dialog visible, press Esc. The dialog dismisses — the app does NOT quit and continues to the main screen.
result: skipped
reason: Welcome dialog not shown — Connect A Provider dialog appears instead, blocking navigation tests

### 5. Welcome Screen Visual Render
expected: Run `cargo run`. The welcome/onboarding screen renders with visible content — no blank output or empty dialog on fresh start.
result: pass

## Summary

total: 5
passed: 1
issues: 2
pending: 0
skipped: 2

## Gaps

- truth: "cargo test -p claurst-tui passes with 0 failures"
  status: failed
  reason: "User reported: thread 'thinking_block_collapsed' panicked at crates/tui/tests/render_snapshots.rs:164:5: assertion failed: !text.contains(\"hidden thoughts\"). 25 passed; 1 failed."
  severity: major
  test: 1
  artifacts:
    - crates/tui/src/messages/mod.rs (render_thinking_block, line ~1245)
    - crates/tui/src/transcript_turn.rs (reasoning_heading, line ~53)
    - crates/tui/tests/render_snapshots.rs (thinking_block_collapsed, line 159)
  root_cause: |
    render_thinking_block() calls reasoning_heading(text) to derive a heading for collapsed mode.
    reasoning_heading() returns the first non-empty line of text verbatim (up to 72 chars).
    For input "hidden thoughts", it returns Some("hidden thoughts"), which is set as the heading span.
    The collapsed view renders "Thinking: hidden thoughts", so the assertion !text.contains("hidden thoughts") fails.
    The fix: in collapsed mode (expanded=false), skip reasoning_heading and use a static "Thinking" label.
    Pre-existing bug — not introduced by Phase 1.
  missing: []

- truth: "Onboarding opens with Welcome page first, then advances to KeyBindings on Enter"
  status: failed
  reason: "User reported: Welcome dialog does not appear. Connect A Provider dialog appears instead."
  severity: major
  test: 2
  artifacts:
    - crates/cli/src/main.rs (onboarding show logic, lines 1430-1443)
    - crates/tui/src/onboarding_dialog.rs (show() vs show_provider_setup())
  root_cause: |
    main.rs:1435 calls app.onboarding_dialog.show_provider_setup() when !has_credentials && !has_completed_onboarding.
    show_provider_setup() goes directly to the "Connect A Provider" / ProviderSetup page, bypassing the Welcome page.
    D-01 states show() should be the entry point (Welcome page first) for first-run users without credentials.
    The production code contradicts D-01: show_provider_setup() is called instead of show().
    Pre-existing bug — Phase 1 fixed the test layer but did not audit main.rs startup logic against D-01.
  missing:
    - main.rs:1435 should call app.onboarding_dialog.show() instead of show_provider_setup() for first-run no-credentials users
