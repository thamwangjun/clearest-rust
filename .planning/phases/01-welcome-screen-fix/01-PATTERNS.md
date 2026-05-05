# Phase 1: Welcome Screen Fix - Pattern Map

**Mapped:** 2026-05-05
**Files analyzed:** 2 modified files
**Analogs found:** 2 / 2

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/tui/src/onboarding_dialog.rs` | test (fix existing) | event-driven | `crates/tui/src/onboarding_dialog.rs` tests (lines 380–460) | exact — same file |
| `crates/tui/src/app.rs` | test (new assertions) | event-driven | `crates/tui/src/app.rs` tests (lines 5601–5814) | exact — same file |

---

## Pattern Assignments

### `crates/tui/src/onboarding_dialog.rs` — fix `onboarding_defaults_hidden` test (line 390)

**Change type:** Single-line assertion fix (wrong expected value).

**Analog:** The same file's existing test pattern (lines 386–391).

**Existing test pattern** (lines 386–391 — what it looks like today):
```rust
#[test]
fn onboarding_defaults_hidden() {
    let state = OnboardingDialogState::new();
    assert!(!state.visible);
    assert_eq!(state.page, OnboardingPage::Welcome); // BUG: wrong — default is ProviderSetup
}
```

**Fix — change line 390 to:**
```rust
assert_eq!(state.page, OnboardingPage::ProviderSetup); // correct: #[default] is ProviderSetup
```

**Why:** `OnboardingPage::default()` has `#[default]` on the `ProviderSetup` variant (line 26 of `onboarding_dialog.rs`). `show()` explicitly sets `page = Welcome`; constructing `OnboardingDialogState::new()` without calling `show()` leaves it at the default.

**Key fact — `show()` vs default** (lines 41–50):
```rust
impl OnboardingDialogState {
    pub fn new() -> Self {
        Self::default()  // page = ProviderSetup, visible = false
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.page = OnboardingPage::Welcome;  // explicit override
    }
```

---

### `crates/tui/src/app.rs` — add D-06 regression tests

**Change type:** Add 3 new `#[test]` functions to the existing `#[cfg(test)] mod tests` block at line 5601.

**Analog:** Existing tests at lines 5758–5814 (`test_question_mark_shortcut_*`, `test_ctrl_a_shortcut_opens_model_picker`).

**Test module imports pattern** (lines 5601–5604 — already present, no changes needed):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
```

**Test helper pattern** (lines 5606–5619 — already present, use as-is):
```rust
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

**Closest structural analog** (lines 5758–5766 — open dialog, call handle_key_event, assert state):
```rust
#[test]
fn test_question_mark_shortcut_opens_help_with_shift_modifier() {
    let mut app = make_app();

    app.handle_key_event(press_key(KeyCode::Char('?'), KeyModifiers::SHIFT));

    assert!(app.help_overlay.visible);
    assert!(app.show_help);
}
```

**Pattern for dialog-open-then-keypress** (lines 5768–5778 — sets state before keypress, then asserts toggle):
```rust
#[test]
fn test_question_mark_shortcut_closes_help_with_shift_modifier() {
    let mut app = make_app();
    app.help_overlay.toggle();
    app.show_help = true;

    app.handle_key_event(press_key(KeyCode::Char('?'), KeyModifiers::SHIFT));

    assert!(!app.help_overlay.visible);
    assert!(!app.show_help);
}
```

**Three new tests to add** (copy structure from analog above):

**Test 1 — Enter on Welcome advances to KeyBindings:**
```rust
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
        "page must advance Welcome -> KeyBindings");
    assert!(app.onboarding_dialog.visible, "dialog must remain visible");
}
```

**Test 2 — Enter on KeyBindings dismisses dialog:**
```rust
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

**Test 3 — Esc dismisses on any page:**
```rust
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

**Why `should_quit` must be checked:** `should_quit` can be set by a guard that fires BEFORE the onboarding guard in `handle_key_event`. Specifically, the `bypass_permissions_dialog` guard at lines 2744–2765 can set `should_quit = true` for a `KeyCode::Enter` when `is_accept_selected() = false`. The `return false` from the onboarding guard only prevents further key processing — it does NOT roll back `should_quit` already set by an earlier guard. Asserting `should_quit == false` AFTER calling `handle_key_event` is the correct check per pitfall 2 in RESEARCH.md.

---

## Shared Patterns

### Dialog early-return guard pattern
**Source:** `crates/tui/src/app.rs` lines 2768–2787
**Apply to:** Understanding the existing handler (no change needed — code is correct)
```rust
// Onboarding dialog: shown on first launch, dismissed with Enter/→/Esc.
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

### any_dialog_open guard
**Source:** `crates/cli/src/main.rs` lines 1675–1705
**Apply to:** Confirms `onboarding_dialog.visible` is already in the guard at line 1683 — no main.rs changes needed
```rust
let any_dialog_open = app.connect_dialog.visible
    // ...
    || app.onboarding_dialog.visible   // line 1683 — already present
    || app.bypass_permissions_dialog.visible
    // ...;
if key.code == KeyCode::Enter && !app.is_streaming && !any_dialog_open {
    // submit path — correctly skipped when onboarding dialog is open
```

---

## No Analog Found

None. Both modified files are the primary files (no novel patterns needed from elsewhere in the codebase).

---

## Guard Ordering Note (for planner)

The `bypass_permissions_dialog` guard (lines 2744–2765) fires BEFORE the `onboarding_dialog` guard (lines 2768–2787) in `handle_key_event`. For the normal first-run case, `bypass_permissions_dialog.visible = false` (requires `--dangerously-skip-permissions` flag), so this ordering does not trigger. The fix does not require reordering these guards — RESEARCH.md explicitly flags this as an anti-pattern.

The correct fix is adding the D-06 tests that assert `should_quit == false` after `handle_key_event`. If those tests pass, the production code is sound. If they fail, a defensive guard must be added in `handle_key_event` before the bypass_permissions block.

---

## Metadata

**Analog search scope:** `crates/tui/src/onboarding_dialog.rs`, `crates/tui/src/app.rs`, `crates/cli/src/main.rs`
**Files scanned:** 3
**Pattern extraction date:** 2026-05-05
