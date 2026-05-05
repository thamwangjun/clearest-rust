---
phase: 01-welcome-screen-fix
reviewed: 2026-05-05T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - crates/tui/src/onboarding_dialog.rs
  - crates/tui/src/app.rs
findings:
  critical: 1
  warning: 3
  info: 2
  total: 6
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-05-05
**Depth:** standard
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Reviewed `onboarding_dialog.rs` (state machine + renderer) and the onboarding-related sections of `app.rs` (key handler, persistence, and launch logic in `cli/src/main.rs`). The implementation introduces the dialog state machine and three render pages, wires key events in `handle_key_event`, and adds a `persist_onboarding_complete` helper.

One critical bug was found: pressing **Esc** to dismiss the onboarding dialog does not persist `has_completed_onboarding = true`, so the dialog reappears on every subsequent launch for users who dismiss it early. Three warnings were found: a dead public API (`show_provider_setup`), blocking filesystem I/O on the tokio event thread, and duplicate footer copy rendered on the ProviderSetup page. Two info items cover a silent no-op on Left-arrow from the first page and a visual inconsistency between pages.

---

## Critical Issues

### CR-01: Esc dismissal does not persist `has_completed_onboarding`

**File:** `crates/tui/src/app.rs:2771-2772`

**Issue:** When the user presses Esc to dismiss the onboarding dialog, `dismiss()` is called but `persist_onboarding_complete()` is not. On the next launch `main.rs:1434` re-evaluates `!settings.has_completed_onboarding`, finds it still `false`, and calls `app.onboarding_dialog.show()` again. The dialog re-appears on every launch until the user navigates all the way to the `Done` page via Enter. This is the root cause of the "onboarding silent exit" bug (BUG-01).

```rust
// Current — Esc branch does not persist
KeyCode::Esc => {
    self.onboarding_dialog.dismiss();
    // ← missing: let _ = Self::persist_onboarding_complete();
}
```

**Fix:**

```rust
KeyCode::Esc => {
    self.onboarding_dialog.dismiss();
    let _ = Self::persist_onboarding_complete();
}
```

The same fix applies to the `ProviderSetup` page Esc path — the comment on line 2768 already documents "Enter or Esc" as valid dismissal keys; the persistence must follow both paths.

---

## Warnings

### WR-01: `show_provider_setup()` is dead code — `ProviderSetup` page is unreachable at runtime

**File:** `crates/tui/src/onboarding_dialog.rs:53-56`

**Issue:** `show_provider_setup()` is the only way to set `page = OnboardingPage::ProviderSetup`. A codebase-wide search confirms it is never called: `main.rs:1433-1435` always calls `app.onboarding_dialog.show()` (which sets `page = Welcome`), even in the no-credentials branch. As a result:

- `render_provider_setup_page` is never rendered.
- The `ProviderSetup` arm in `next_page()` (line 65) and `prev_page()` (line 76) is dead.
- The provider picker UI that was presumably the main deliverable for the no-credentials case is silently skipped in favour of the generic Welcome page.

**Fix:** Either call `show_provider_setup()` from `main.rs` when `!has_credentials && !settings.has_completed_onboarding`, replacing the current `show()` call:

```rust
// main.rs ~line 1434
if !has_credentials {
    if !settings.has_completed_onboarding {
-       app.onboarding_dialog.show();
+       app.onboarding_dialog.show_provider_setup();
    } else {
        app.status_message = Some("No provider configured. Run /connect to set one up.".to_string());
    }
}
```

Or, if `ProviderSetup` has been intentionally replaced by the Welcome flow, delete `show_provider_setup()`, the `ProviderSetup` variant, and `render_provider_setup_page` to eliminate dead code.

---

### WR-02: Blocking filesystem I/O called from the async tokio event loop

**File:** `crates/tui/src/app.rs:2704-2707`

**Issue:** `persist_onboarding_complete()` calls `Settings::load_sync()` (which does `std::fs::read_to_string`) and `settings.save_sync()` (which does `std::fs::write`) directly. `handle_key_event` is invoked from the tokio main-thread event loop (`main.rs:1648` and `2272`), which runs inside `#[tokio::main]`. Blocking the tokio thread during a key event stalls all other async tasks (streaming responses, MCP communication, status polling) until the disk I/O completes.

**Fix:** Spawn the persistence as a background task so the event loop is not blocked:

```rust
KeyCode::Enter | KeyCode::Right => {
    if self.onboarding_dialog.next_page() {
        self.onboarding_dialog.dismiss();
        tokio::spawn(async {
            if let Ok(mut s) = claurst_core::config::Settings::load().await {
                s.has_completed_onboarding = true;
                let _ = s.save().await;
            }
        });
    }
}
```

Apply the same pattern to the Esc branch once CR-01 is fixed.

---

### WR-03: Duplicate footer content rendered in `render_provider_setup_page`

**File:** `crates/tui/src/onboarding_dialog.rs:199-218`

**Issue:** The `lines` vector in `render_provider_setup_page` contains two independent "20+ more providers" lines and two independent "Esc: dismiss" lines:

- Lines 199-208: `"  + "` prefix + `"20+ more providers: "` + `"claurst --help"`, followed by `"  Esc dismiss · configure later with /providers"`.
- Lines 210-218: `"  → 20+ more providers: claurst --help"` and `"  Esc: dismiss  (you can configure later with /providers)"`.

Both sets are always rendered, so the user sees the same information twice. When this page eventually becomes reachable (WR-01 fix), it will show a confusing doubled footer.

**Fix:** Remove the duplicate block (lines 210-218). Keep whichever version matches the rest of the UI style (the structured multi-span version at lines 199-208 is more consistent with the rest of the dialog).

---

## Info

### IN-01: Left-arrow on `Welcome` page is a silent no-op with no visual feedback

**File:** `crates/tui/src/onboarding_dialog.rs:77`

**Issue:** `prev_page()` maps `Welcome => Welcome`, meaning Left arrow on the first page does nothing. The Welcome page footer shows `"enter next  ·  esc skip"` (no back arrow), which is correct — but the key handler in `app.rs:2781-2783` still calls `prev_page()` unconditionally on Left. If the ProviderSetup page is ever wired up (WR-01 fix), pressing Left from Welcome would need to navigate to ProviderSetup, not stay on Welcome. The current mapping would silently break backward navigation.

**Fix:** Either add a guard in `handle_key_event` to skip `prev_page()` on the first page, or ensure the `prev_page()` transition table is updated when new pages are added.

---

### IN-02: `render_provider_setup_page` does not call `render_dark_overlay` / `render_dialog_bg`

**File:** `crates/tui/src/onboarding_dialog.rs:115-223`

**Issue:** `render_welcome_page` (line 233) and `render_keybindings_page` (line 304) both call `render_dark_overlay(frame, area)` and `render_dialog_bg(frame, area)` before drawing their content. `render_provider_setup_page` only relies on the `Clear` widget rendered by the outer `render_onboarding_dialog` function (line 105), skipping the overlay and panel background. If this page becomes reachable (WR-01 fix), it will render without the dark overlay and panel background, looking visually inconsistent with the other pages.

**Fix:** Add the same overlay calls at the top of `render_provider_setup_page`:

```rust
fn render_provider_setup_page(frame: &mut Frame, area: Rect) {
    use crate::overlays::{render_dark_overlay, render_dialog_bg};
    render_dark_overlay(frame, area);
    render_dialog_bg(frame, area);
    // ... rest of function
}
```

---

_Reviewed: 2026-05-05_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
