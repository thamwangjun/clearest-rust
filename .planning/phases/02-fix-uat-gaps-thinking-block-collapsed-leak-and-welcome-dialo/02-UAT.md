---
status: partial
phase: 02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo
source:
  - .planning/phases/02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo/02-01-SUMMARY.md
  - .planning/phases/02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo/02-02-SUMMARY.md
started: 2026-05-05T00:00:00Z
updated: 2026-05-06T00:00:00Z
---

## Current Test

[blocked — app cannot start until ANTHROPIC_AUTH_TOKEN is wired to Bearer auth]

## Tests

### 1. Test Suite Green
expected: Run `cargo test -p claurst-tui` from the repo root. All 27 tests pass, 0 fail — including `thinking_block_collapsed` (previously failing) and `show_starts_at_welcome_page` (new regression test).
result: pass

### 2. Collapsed Thinking Block Shows Dots
expected: In the TUI, when an AI message has a collapsed thinking block, the collapsed header shows animated dots (`.` / `..` / `...` cycling) — NOT the text content of the thinking. No fragment of the thinking text should appear in the collapsed state.
result: blocked
blocked_by: other
reason: "App cannot connect — ANTHROPIC_AUTH_TOKEN not wired to Bearer auth; proxy rejects x-api-key header. Deferred to a new phase."

### 3. Welcome Page on First Run
expected: Run `cargo run` with no credentials configured (or on a fresh profile). The onboarding dialog opens to the **Welcome** page first — not "Connect A Provider" / ProviderSetup page.
result: blocked
blocked_by: other
reason: "App cannot connect — blocked by same ANTHROPIC_AUTH_TOKEN issue."

### 4. Onboarding Advance with Enter
expected: With the onboarding dialog on the Welcome page, press Enter. The dialog advances to the next page (KeyBindings) — the app does NOT quit, and the dialog remains visible.
result: blocked
blocked_by: other
reason: "App cannot connect — blocked by same ANTHROPIC_AUTH_TOKEN issue."

## Summary

total: 4
passed: 1
issues: 0
pending: 0
skipped: 0
blocked: 3

## Gaps

[none yet]
