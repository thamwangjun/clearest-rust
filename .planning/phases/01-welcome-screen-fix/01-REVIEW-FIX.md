---
phase: 01-welcome-screen-fix
fixed_at: 2026-05-05T00:00:00Z
review_path: .planning/phases/01-welcome-screen-fix/01-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 01: Code Review Fix Report

**Fixed at:** 2026-05-05
**Source review:** `.planning/phases/01-welcome-screen-fix/01-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 6
- Fixed: 6
- Skipped: 0

## Fixed Issues

### CR-01: Esc dismissal does not persist `has_completed_onboarding`

**Files modified:** `src-rust/crates/tui/src/app.rs`
**Commit:** 212d217
**Applied fix:** Added `let _ = Self::persist_onboarding_complete();` to the `KeyCode::Esc` branch in `handle_key_event`, mirroring the persistence call already present in the Enter/Right branch.

---

### WR-01: `show_provider_setup()` is dead code — ProviderSetup page unreachable

**Files modified:** `src-rust/crates/cli/src/main.rs`
**Commit:** 366ab28
**Applied fix:** Changed `app.onboarding_dialog.show()` to `app.onboarding_dialog.show_provider_setup()` in the `!has_credentials && !has_completed_onboarding` branch so the provider picker page is shown to uncredentialed users.

---

### WR-02: Blocking filesystem I/O on tokio event loop

**Files modified:** `src-rust/crates/tui/src/app.rs`
**Commits:** 8849a7a, 6cee96d
**Applied fix:** Replaced both `let _ = Self::persist_onboarding_complete()` call sites in `handle_key_event` with `tokio::spawn` async blocks using `Settings::load().await` / `save().await`. Added a `tokio::runtime::Handle::try_current()` guard so that unit tests running without a tokio runtime fall back to the sync helper instead of panicking. The sync `persist_onboarding_complete()` is retained for the `persist_onboarding_complete_pub()` wrapper used at startup in `main.rs`.

Note: requires human verification that the runtime-guard fallback path is acceptable for production (it should only trigger in tests, not at runtime).

---

### WR-03: Duplicate footer content in `render_provider_setup_page`

**Files modified:** `src-rust/crates/tui/src/onboarding_dialog.rs`
**Commit:** 70c3d7a
**Applied fix:** Removed the second duplicate footer block (the plain `DarkGray` `→ 20+ more providers` and `Esc: dismiss` lines). The structured multi-span version (matching the rest of the dialog style) is retained.

---

### IN-01: Left-arrow on Welcome page is a silent no-op

**Files modified:** `src-rust/crates/tui/src/onboarding_dialog.rs`, `src-rust/crates/tui/src/app.rs`
**Commit:** 7623640
**Applied fix:** Added `is_first_page()` method to `OnboardingDialogState` that returns `true` for `ProviderSetup` and `Welcome` pages. The `KeyCode::Left` handler in `handle_key_event` now guards the `prev_page()` call with `!is_first_page()`, making the no-op explicit and preventing silent breakage if new pages are added.

---

### IN-02: `render_provider_setup_page` missing overlay/background calls

**Files modified:** `src-rust/crates/tui/src/onboarding_dialog.rs`
**Commit:** bd2ea5e
**Applied fix:** Added `use crate::overlays::{render_dark_overlay, render_dialog_bg};` and the two calls at the top of `render_provider_setup_page`, matching the pattern used by `render_welcome_page` and `render_keybindings_page`.

---

_Fixed: 2026-05-05_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
