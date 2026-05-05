# Phase 1: Welcome Screen Fix - Context

**Gathered:** 2026-05-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix the first-launch welcome/onboarding dialog so that pressing Enter advances through pages (Welcome → KeyBindings → dismiss) rather than causing claurst to exit silently. No new features, no credential-setup UI changes.

</domain>

<decisions>
## Implementation Decisions

### Dialog Page Flow
- **D-01:** Keep `show()` call when no credentials on first run — Welcome page is the correct starting page. `show_provider_setup()` is not called; this is intentional.
- **D-02:** Page order stays as-is: Welcome → KeyBindings → Done. No reordering.

### Enter Key Behavior
- **D-03:** The bug manifests on the Welcome page — pressing Enter causes a full app exit instead of advancing to KeyBindings. Root cause must be traced in research. The fix must ensure Enter while `onboarding_dialog.visible = true` is intercepted before any quit/submit path.
- **D-04:** ProviderSetup → Done on Enter (single press) is fine to leave as-is since `show_provider_setup()` is never called today.

### Post-Dismiss State
- **D-05:** No status message needed after dialog dismissal. The dialog content already communicates what to do (run `/connect`).

### Regression Testing
- **D-06:** Add a unit test in `app.rs` or a dedicated test module that calls `handle_key_event(KeyCode::Enter)` while `onboarding_dialog.visible = true`, and asserts:
  - `should_quit` remains `false`
  - The page advances from Welcome to KeyBindings (not dismissed)
  - A second Enter advances to Done and dismisses the dialog

### Claude's Discretion
- Exact mechanism of the silent exit (likely a missing guard or premature `should_quit = true` / `break 'main` path that fires before the onboarding handler). Researcher should trace through `handle_key_event` → `bypass_permissions_dialog` ordering → quit paths in `app.rs` and the `'main` loop in `main.rs`.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Onboarding Dialog
- `src-rust/crates/tui/src/onboarding_dialog.rs` — dialog state machine (`OnboardingDialogState`, `OnboardingPage`, `next_page()`, `dismiss()`), render functions for each page, existing unit tests

### App Key Event Handling
- `src-rust/crates/tui/src/app.rs` — `handle_key_event()` (line ~2718), onboarding handler (lines ~2768–2786), `bypass_permissions_dialog` handler (lines ~2742–2766), quit paths (`should_quit = true` at lines ~1973, 3670, 4245, 5502)
- `src-rust/crates/tui/src/app.rs` — `should_quit` field (line ~659)

### Main Event Loop
- `src-rust/crates/cli/src/main.rs` — `'main: loop` (line ~1621), `any_dialog_open` guard (lines ~1675–1705) that includes `app.onboarding_dialog.visible`, `handle_key_event` call (line ~2272), `should_quit` check (line ~3143)
- `src-rust/crates/cli/src/main.rs` — onboarding show logic (lines ~1430–1443)

### Requirements
- `.planning/REQUIREMENTS.md` — BUG-01 definition
- `.planning/ROADMAP.md` — Phase 1 success criteria

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `OnboardingDialogState::next_page()`: already correct state machine; returns `true` only when Done. Tests for this exist in `onboarding_dialog.rs`.
- `bypass_permissions_dialog` handler in `handle_key_event`: pattern to copy for guard ordering — returns early to prevent any fall-through.

### Established Patterns
- All dialogs in `handle_key_event` use early-return (`return false`) to prevent event propagation. The onboarding handler already follows this pattern.
- `any_dialog_open` in main.rs's `'main: loop` is the guard for the Enter-submit path. It already includes `onboarding_dialog.visible` (confirmed at line ~1683).

### Integration Points
- The `bypass_permissions_dialog` check runs BEFORE the `onboarding_dialog` check in `handle_key_event`. If both were somehow visible, bypass_permissions_dialog would fire first and could set `should_quit = true`. However, `bypass_permissions_dialog` is only shown when `permission_mode == BypassPermissions` (non-default config), making this an unlikely but noteworthy ordering issue.
- `persist_onboarding_complete()` is synchronous I/O (`load_sync`/`save_sync`) called inside the async event loop. Best-effort (wrapped in `let _ =`), not a bug source but worth noting.

</code_context>

<specifics>
## Specific Ideas

- The user confirmed: pressing Enter on the Welcome page causes a full app exit (claurst returns to the shell). The ProviderSetup page is never reached in this flow. This is the exact failure mode.
- Success criteria from ROADMAP.md must all pass: (1) Enter on welcome → main TUI proceeds, (2) no unexpected termination or blank output on first launch, (3) consistent behavior on macOS and Linux.

</specifics>

<deferred>
## Deferred Ideas

- Showing `ProviderSetup` page as the first page for no-credentials users — declined by user, Welcome page first is correct.
- Showing a status message after dialog dismissal with no credentials — declined by user, not needed.

</deferred>

---

*Phase: 1-welcome-screen-fix*
*Context gathered: 2026-05-04*
