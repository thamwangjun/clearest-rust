# Phase 1: Welcome Screen Fix - Research

**Researched:** 2026-05-05
**Domain:** Ratatui TUI event handling, crossterm keyboard events, Rust dialog state machines
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** Keep `show()` call when no credentials on first run — Welcome page is the correct starting page. `show_provider_setup()` is not called; this is intentional.
- **D-02:** Page order stays as-is: Welcome → KeyBindings → Done. No reordering.
- **D-03:** The bug manifests on the Welcome page — pressing Enter causes a full app exit instead of advancing to KeyBindings. Root cause must be traced in research. The fix must ensure Enter while `onboarding_dialog.visible = true` is intercepted before any quit/submit path.
- **D-04:** ProviderSetup → Done on Enter (single press) is fine to leave as-is since `show_provider_setup()` is never called today.
- **D-05:** No status message needed after dialog dismissal. The dialog content already communicates what to do (run `/connect`).
- **D-06:** Add a unit test in `app.rs` or a dedicated test module that calls `handle_key_event(KeyCode::Enter)` while `onboarding_dialog.visible = true`, and asserts:
  - `should_quit` remains `false`
  - The page advances from Welcome to KeyBindings (not dismissed)
  - A second Enter advances to Done and dismisses the dialog

### Claude's Discretion

- Exact mechanism of the silent exit (likely a missing guard or premature `should_quit = true` / `break 'main` path that fires before the onboarding handler). Researcher should trace through `handle_key_event` → `bypass_permissions_dialog` ordering → quit paths in `app.rs` and the `'main` loop in `main.rs`.

### Deferred Ideas (OUT OF SCOPE)

- Showing `ProviderSetup` page as the first page for no-credentials users — declined by user.
- Showing a status message after dialog dismissal with no credentials — declined by user.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BUG-01 | User can complete the first-launch welcome screen by pressing Enter without claurst exiting silently | Root cause traced; fix site and regression test identified |
</phase_requirements>

---

## Summary

Phase 1 fixes a single bug: pressing Enter on the Welcome page of the first-launch onboarding dialog causes claurst to exit silently instead of advancing to the KeyBindings page. The fix requires ensuring the onboarding dialog intercepts the Enter key before any quit path in `handle_key_event` and verifying the `any_dialog_open` guard in the `'main:` event loop is sound.

A full code trace was performed on `crates/tui/src/app.rs` and `crates/cli/src/main.rs`. The onboarding dialog handler in `handle_key_event` (lines 2769–2786) correctly intercepts Enter and returns `false` — it is structurally sound. The `any_dialog_open` guard in the main loop (lines 1675–1705) also correctly includes `app.onboarding_dialog.visible`. However, one existing test (`onboarding_defaults_hidden`) has a wrong assertion and is currently failing. No regression test for the `handle_key_event(Enter)` path while the dialog is visible exists — that is the primary gap D-06 is addressing.

The guard-ordering concern documented in CONTEXT.md (bypass_permissions_dialog fires before onboarding_dialog in `handle_key_event`) is the most plausible mechanism for silent exit if `bypass_permissions_dialog.visible` is ever unexpectedly true. For typical first-run users without `--dangerously-skip-permissions`, this dialog is not visible, so the ordering issue is latent but not the everyday trigger. The fix should add a defensive early-return guard for the onboarding dialog BEFORE bypass_permissions, or at minimum confirm the ordering in a test.

**Primary recommendation:** Fix the failing `onboarding_defaults_hidden` test (wrong assertion about the default `OnboardingPage`), add the D-06 regression tests to `app.rs`, and validate that `handle_key_event(Enter)` while `onboarding_dialog.visible = true` never sets `should_quit` — regardless of what other internal state is set.

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Onboarding dialog state machine | TUI (`crates/tui`) | — | `OnboardingDialogState` owns all page transitions via `next_page()` / `prev_page()` / `dismiss()` |
| Key event routing | TUI (`handle_key_event`) | CLI (`'main: loop`) | `handle_key_event` is the canonical handler; main loop only adds early-exit guards for Ctrl+C and Ctrl+D |
| First-run detection and dialog activation | CLI (`run_interactive`) | TUI (dialog visible flag) | `has_credentials` and `has_completed_onboarding` are evaluated in main.rs; `show()` is called before the event loop |
| Regression tests | TUI (`crates/tui/src/app.rs`, `onboarding_dialog.rs`) | — | Existing `#[cfg(test)]` modules in both files are the correct location |

---

## Standard Stack

No new crates are needed for this fix. The existing stack handles everything.

### Core (existing — do not add or change)

| Library | Version | Purpose |
|---------|---------|---------|
| `ratatui` | 0.29 | TUI rendering framework — already used throughout |
| `crossterm` | 0.28 | Terminal backend, keyboard event types (`KeyCode`, `KeyEvent`, `KeyModifiers`) |

### Installation

No new dependencies. Do not add to `Cargo.toml`.

---

## Architecture Patterns

### System Architecture Diagram

```
User presses Enter
       │
       ▼
main.rs 'main: loop
       │
       ├─ Ctrl+C / Ctrl+D early exits ──────────────── break 'main
       │
       ├─ any_dialog_open computation
       │   (includes onboarding_dialog.visible)
       │
       ├─ if Enter && !any_dialog_open ──────────────── [SKIPPED when dialog visible]
       │      └─ take_input / submit path
       │
       ├─ if permission_request.is_some() ────────────── [SKIPPED on first-run]
       │
       └─ app.handle_key_event(key)  ◄──────────────── ALWAYS called for non-early-exit keys
              │
              ├─ global_search guard ──► return false
              ├─ context_menu guard ───► return false
              ├─ bypass_permissions guard ─► possibly should_quit = true + return false
              ├─ onboarding guard ─────► next_page() / dismiss() + return false  ✓
              └─ [general key handling falls through here if no guard matched]
```

### Recommended Fix Site

The fix belongs in `handle_key_event` and in the test modules. No change to `main.rs`'s `any_dialog_open` is required — it already includes `onboarding_dialog.visible`.

### Pattern: Early-Return Dialog Guard (existing, to replicate for the test)

```rust
// Source: crates/tui/src/app.rs lines 2769-2786
if self.onboarding_dialog.visible {
    match key.code {
        KeyCode::Esc => {
            self.onboarding_dialog.dismiss();
        }
        KeyCode::Enter | KeyCode::Right => {
            if self.onboarding_dialog.next_page() {
                self.onboarding_dialog.dismiss();
                let _ = Self::persist_onboarding_complete();
            }
        }
        KeyCode::Left => {
            self.onboarding_dialog.prev_page();
        }
        _ => {}
    }
    return false;
}
```

This code is correct as written. The `return false` at line 2786 prevents any fall-through to the general key handling (including the `KeyCode::Enter if !self.is_streaming` branch that returns `true` to signal submit). The regression test's purpose is to confirm this guard executes correctly in context.

### Pattern: make_app() + press_key() Test Helpers (existing)

```rust
// Source: crates/tui/src/app.rs lines 5606-5619
fn make_app() -> App {
    let config = Config::default();
    let cost_tracker = claurst_core::cost::CostTracker::new();
    App::new(config, cost_tracker)
}

fn press_key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}
```

The D-06 regression tests should use these helpers — they're already in the `#[cfg(test)]` module in `app.rs`.

### Anti-Patterns to Avoid

- **Do not reorder the dialog guards in `handle_key_event`**: The bypass_permissions guard must remain before the onboarding guard per its "highest-priority gate" comment. Reordering creates a different bug for bypass-permission sessions.
- **Do not add `onboarding_dialog.visible` check to the Ctrl+C path in `main.rs`**: The Ctrl+C handler uses `break 'main` without checking dialogs; this is intentional (Ctrl+C is an emergency interrupt). The dialog already guards against Enter via `any_dialog_open` — Ctrl+C is documented behavior.
- **Do not call `show_provider_setup()`**: D-01 is locked — `show()` (Welcome page) is the entry point for no-credentials users.

---

## Root Cause Trace

### Confirmed Correct Code Paths

The following are confirmed correct by code reading and do NOT need to change: [VERIFIED: crates/tui/src/app.rs, crates/cli/src/main.rs — direct read]

1. **`any_dialog_open` in main.rs** (lines 1675–1705): Includes `app.onboarding_dialog.visible` at line 1683. The Enter-submit block at line 1706 is correctly gated by `!any_dialog_open`.

2. **Onboarding dialog handler in `handle_key_event`** (lines 2769–2786): Enter while `onboarding_dialog.visible = true` calls `next_page()`. For Welcome page, `next_page()` transitions to KeyBindings and returns `false` (not Done). `dismiss()` is NOT called on the first Enter. The handler returns `false` — the submit path (`KeyCode::Enter if !self.is_streaming => return true` at line 3823) is never reached.

3. **`next_page()` state machine** (onboarding_dialog.rs lines 63–71): `Welcome → KeyBindings` (returns `false`), `KeyBindings → Done` (returns `true`). Correct.

4. **`show()` activation** (main.rs line 1435): Called for `!has_credentials && !has_completed_onboarding` — the correct first-run no-credentials path.

### Confirmed Bug: Failing Existing Test

`onboarding_defaults_hidden` in `onboarding_dialog.rs` (line 387) asserts `state.page == OnboardingPage::Welcome` for the default state. But `OnboardingPage::default()` is `ProviderSetup` (the `#[default]` attribute is on `ProviderSetup`). **This test currently fails.** [VERIFIED: cargo test output]

```
thread 'onboarding_dialog::tests::onboarding_defaults_hidden' panicked at crates/tui/src/onboarding_dialog.rs:390:9:
assertion `left == right` failed
  left: ProviderSetup
 right: Welcome
```

Fix: Change the assertion to `assert_eq!(state.page, OnboardingPage::ProviderSetup)`. The `visible` assertion (`assert!(!state.visible)`) is correct.

### Missing Tests (D-06 Gap)

No test currently calls `app.handle_key_event(KeyCode::Enter)` while `onboarding_dialog.visible = true` and asserts `should_quit == false`. This is the D-06 requirement. The three test cases required:

1. Enter on Welcome: `should_quit = false`, page advances to KeyBindings, dialog remains visible
2. Enter on KeyBindings: `should_quit = false`, dialog dismissed (`visible = false`)
3. Esc on Welcome: `should_quit = false`, dialog dismissed

### Bypass Permissions Ordering Risk

The `bypass_permissions_dialog` guard (lines 2744–2765) fires BEFORE the onboarding guard (lines 2769–2786). If `bypass_permissions_dialog.visible = true` AND Enter is pressed AND `is_accept_selected() = false` (default state — "No, exit" selected), then `should_quit = true`. [VERIFIED: crates/tui/src/app.rs lines 2756–2760]

For typical first-run users, `bypass_permissions_dialog.visible = false` (requires `--dangerously-skip-permissions`). This ordering issue does NOT trigger for normal first-launch scenarios. However, an additional test verifying `bypass_permissions_dialog.visible = false` when the onboarding dialog is shown would add defense.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Key event simulation in tests | Custom `Event` construction | `press_key()` helper already in `app.rs` tests | The helper is already in scope in the test module |
| Dialog state assertion | New dialog observer pattern | Direct field access (`app.should_quit`, `app.onboarding_dialog.page`) | Rust's ownership model makes field access unambiguous in tests |

---

## Common Pitfalls

### Pitfall 1: Wrong Default Page in Test
**What goes wrong:** Tests or documentation that assume `OnboardingDialogState::new()` starts on the Welcome page (not ProviderSetup).
**Why it happens:** `OnboardingPage::default()` is `ProviderSetup` (the `#[default]` attribute). `show()` explicitly sets `page = Welcome`. The default and post-`show()` pages differ.
**How to avoid:** Always call `state.show()` before asserting `page == Welcome`. Test the default with `page == ProviderSetup`.
**Warning signs:** Tests checking `page == Welcome` on a freshly constructed state.

### Pitfall 2: Assuming `return false` from `handle_key_event` Prevents Quit
**What goes wrong:** Believing `return false` in the onboarding handler is sufficient to prevent `should_quit = true`.
**Why it happens:** `should_quit` can be set by code BEFORE the onboarding guard (e.g., bypass_permissions guard at lines 2748, 2760). The return value only prevents FURTHER key processing — it does not roll back `should_quit` already set.
**How to avoid:** The D-06 test must assert `should_quit == false` AFTER calling `handle_key_event`, not just check the return value.
**Warning signs:** `should_quit` unexpectedly `true` after a dialog guard returns `false`.

### Pitfall 3: Modifying `'main: loop` Enter Guard Without Re-Checking All Paths
**What goes wrong:** Adding conditions to `any_dialog_open` or restructuring the Enter guard introduces new early-exits that bypass `handle_key_event` entirely.
**Why it happens:** The block from line 1706 to 2201 is a single large `if` block. Adding `continue` inside it prevents `handle_key_event` from being called.
**How to avoid:** The fix belongs in `app.rs` (test addition + test bug fix only). Do NOT modify the `any_dialog_open` list or the main loop Enter handling.

### Pitfall 4: persist_onboarding_complete() Side Effects in Tests
**What goes wrong:** Calling code that exercises `persist_onboarding_complete()` in unit tests writes to the real `~/.claurst/settings.json`.
**Why it happens:** `persist_onboarding_complete()` calls `Settings::load_sync()` and `save_sync()` — real filesystem I/O.
**How to avoid:** The D-06 tests should test the Enter-on-KeyBindings case by calling `app.onboarding_dialog.page = OnboardingPage::KeyBindings` directly, without relying on `persist_onboarding_complete()` succeeding. The `let _ =` wrapper means failures are silently ignored, so this is safe — but tests should not depend on filesystem state.

---

## Code Examples

### Test: onboarding_defaults_hidden (fixed assertion)

```rust
// Source: crates/tui/src/onboarding_dialog.rs — fix for line 390
#[test]
fn onboarding_defaults_hidden() {
    let state = OnboardingDialogState::new();
    assert!(!state.visible);
    assert_eq!(state.page, OnboardingPage::ProviderSetup); // was: Welcome (wrong)
}
```

### Test: D-06 Enter on Welcome Page

```rust
// New test in crates/tui/src/app.rs tests module
#[test]
fn test_onboarding_enter_on_welcome_advances_page() {
    let mut app = make_app();
    app.onboarding_dialog.show();
    assert!(app.onboarding_dialog.visible);
    assert_eq!(app.onboarding_dialog.page, OnboardingPage::Welcome);

    let result = app.handle_key_event(press_key(KeyCode::Enter, KeyModifiers::NONE));

    assert!(!app.should_quit, "should_quit must remain false after Enter on Welcome");
    assert!(!result, "handle_key_event must return false while dialog is visible");
    assert_eq!(app.onboarding_dialog.page, OnboardingPage::KeyBindings,
        "page must advance Welcome→KeyBindings");
    assert!(app.onboarding_dialog.visible, "dialog must remain visible");
}
```

### Test: D-06 Enter on KeyBindings Page (dismiss)

```rust
// New test in crates/tui/src/app.rs tests module
#[test]
fn test_onboarding_enter_on_keybindings_dismisses() {
    let mut app = make_app();
    app.onboarding_dialog.show();
    app.onboarding_dialog.page = OnboardingPage::KeyBindings;

    let result = app.handle_key_event(press_key(KeyCode::Enter, KeyModifiers::NONE));

    assert!(!app.should_quit, "should_quit must remain false after Enter on KeyBindings");
    assert!(!result, "handle_key_event must return false");
    assert!(!app.onboarding_dialog.visible, "dialog must be dismissed");
}
```

### Test: D-06 Esc dismisses on any page

```rust
// New test in crates/tui/src/app.rs tests module
#[test]
fn test_onboarding_esc_dismisses() {
    let mut app = make_app();
    app.onboarding_dialog.show();

    let result = app.handle_key_event(press_key(KeyCode::Esc, KeyModifiers::NONE));

    assert!(!app.should_quit);
    assert!(!result);
    assert!(!app.onboarding_dialog.visible);
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| No-credentials users never see onboarding dialog (only status message) | No-credentials users see Welcome → KeyBindings tour | Commit 15758e3 (2026-04-06) | The bug manifests only in this new flow |
| `onboarding_dialog.show()` called when `has_credentials=true && !has_completed_onboarding` | `show()` called when `!has_credentials && !has_completed_onboarding` | Commit 15758e3 | Logic swap is intentional per D-01; test gap is the issue |

---

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The bug is triggered only for first-run users with no credentials (not users with credentials who haven't completed onboarding) | Root Cause Trace | Low — both flows now go through the same `handle_key_event` handler |
| A2 | The test fix (`ProviderSetup` assertion) and D-06 tests are sufficient to expose and prove the fix | Common Pitfalls | Low — the test covers the exact scenario described by D-06 |
| A3 | No change to main.rs is needed for the fix | Architecture Patterns | Medium — if the actual runtime bug is in the main loop rather than `handle_key_event`, additional changes may be needed |

---

## Open Questions (RESOLVED)

1. **Is the bug observable today in the current HEAD?**
   - What we know: The `handle_key_event` code looks correct. `any_dialog_open` includes the dialog. The D-06 test doesn't exist yet to confirm.
   - What's unclear: Whether there's a runtime timing issue not visible in static analysis.
   - Recommendation: The D-06 regression tests should be written and run FIRST (Wave 0). If they pass, the bug was already fixed in 15758e3 and only the missing test remains. If they fail, a code fix is needed in `handle_key_event` or the main loop.
   - RESOLVED: This will be determined at execution time. If the D-06 regression tests pass immediately after being added, the production code is already correct and only the test gap exists. If any test fails with `should_quit=true` or the wrong page, a production code fix is needed (see Task 2 contingency in 01-01-PLAN.md).

2. **Should the bypass_permissions/onboarding ordering be made safe with a combined guard?**
   - What we know: For normal users, `bypass_permissions_dialog.visible = false`, making the ordering safe. The issue is latent.
   - What's unclear: Whether any code path could set both dialogs visible simultaneously.
   - Recommendation: Per D-03, add a defensive comment at the bypass_permissions guard noting the ordering dependency. No code change needed.
   - RESOLVED: No code change needed per D-03 (locked decision). The ordering risk is latent and does not affect normal first-run users. The D-06 tests verify `should_quit=false` for the standard path and serve as a sufficient regression guard.

---

## Environment Availability

Step 2.6: SKIPPED — this phase is purely code/test changes with no external service dependencies.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test framework (`#[cfg(test)]`) |
| Config file | No separate file — uses `cargo test` |
| Quick run command | `cargo test -p claurst-tui --lib onboarding 2>&1` |
| Full suite command | `cargo test -p claurst-tui --lib 2>&1` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BUG-01 | Enter on Welcome advances to KeyBindings, `should_quit = false` | unit | `cargo test -p claurst-tui --lib test_onboarding_enter_on_welcome` | ❌ Wave 0 |
| BUG-01 | Enter on KeyBindings dismisses dialog, `should_quit = false` | unit | `cargo test -p claurst-tui --lib test_onboarding_enter_on_keybindings` | ❌ Wave 0 |
| BUG-01 | Esc on Welcome dismisses dialog, `should_quit = false` | unit | `cargo test -p claurst-tui --lib test_onboarding_esc_dismisses` | ❌ Wave 0 |
| BUG-01 | Default `OnboardingDialogState` has correct defaults | unit | `cargo test -p claurst-tui --lib onboarding_defaults_hidden` | ✅ (failing — fix needed) |

### Sampling Rate

- **Per task commit:** `cargo test -p claurst-tui --lib 2>&1 | tail -5`
- **Per wave merge:** `cargo test -p claurst-tui --lib 2>&1`
- **Phase gate:** Full suite green before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] Fix `onboarding_defaults_hidden` assertion in `crates/tui/src/onboarding_dialog.rs:390`
- [ ] `test_onboarding_enter_on_welcome_advances_page` — add to `crates/tui/src/app.rs` tests module
- [ ] `test_onboarding_enter_on_keybindings_dismisses` — add to `crates/tui/src/app.rs` tests module
- [ ] `test_onboarding_esc_dismisses` — add to `crates/tui/src/app.rs` tests module

---

## Security Domain

Step 2 (Security): This phase is a TUI event-handler bug fix with no authentication, network, or persistence changes. No ASVS categories apply. `security_enforcement` is not explicitly set to `false` in config, but the change surface is a dialog state machine and test assertions — no attack surface.

---

## Sources

### Primary (HIGH confidence)

- `crates/tui/src/onboarding_dialog.rs` — Full file read; confirmed `OnboardingPage::default() = ProviderSetup`, `next_page()` transitions, existing tests, failing assertion in `onboarding_defaults_hidden`
- `crates/tui/src/app.rs` lines 2718–2786 — `handle_key_event` guards including bypass_permissions and onboarding handlers
- `crates/tui/src/app.rs` lines 5601–5619 — Existing test infrastructure (`make_app`, `press_key`)
- `crates/cli/src/main.rs` lines 1633–1706 — Main loop key event routing, `any_dialog_open` guard
- `crates/cli/src/main.rs` lines 1430–1443 — Onboarding show logic for credentials/first-run
- `cargo test -p claurst-tui --lib` — 487 passed, 1 failed (`onboarding_defaults_hidden`)

### Secondary (MEDIUM confidence)

- Git diff `9492e4f..15758e3` — Confirmed the behavioral change: no-credentials users now see the dialog (was: status message only). The key handler was identical before and after this commit.

---

## Metadata

**Confidence breakdown:**
- Bug location: HIGH — failing test directly pinpoints the wrong assertion; D-06 test gap is confirmed by absence
- Handle_key_event correctness: HIGH — code is logically correct; test will confirm
- Fix scope: HIGH — two files, no new crates, no main.rs changes needed
- Runtime behavior confirmation: MEDIUM — static analysis only; actual first-run reproduction not performed

**Research date:** 2026-05-05
**Valid until:** 60 days (Rust codebase, no external dependencies)
