---
phase: 03-anthropic-auth-token-bearer-auth-support
fixed_at: 2026-05-09T00:00:00Z
review_path: .planning/phases/03-anthropic-auth-token-bearer-auth-support/03-REVIEW.md
iteration: 1
findings_in_scope: 9
fixed: 9
skipped: 0
status: all_fixed
---

# Phase 03: Code Review Fix Report

**Fixed at:** 2026-05-09
**Source review:** `.planning/phases/03-anthropic-auth-token-bearer-auth-support/03-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 9
- Fixed: 9
- Skipped: 0

## Fixed Issues

### CR-01: UTF-8 byte-boundary panic in bridge error body truncation

**Files modified:** `crates/bridge/src/lib.rs`
**Commit:** b8dd187
**Applied fix:** Replaced `&body_text[..body_text.len().min(200)]` byte slice with `body_text.chars().take(200).collect::<String>()` so truncation is char-boundary-safe for any UTF-8 input.

---

### CR-02: UTF-8 byte-boundary panic in TUI user-prompt truncation

**Files modified:** `crates/tui/src/messages/mod.rs`
**Commit:** 3282b2a
**Applied fix:** Rewrote `truncate_user_prompt_text` to use `text.chars().count()` for the length guard and `chars().take(N).collect()` / `chars().skip(N).collect()` for head and tail extraction, eliminating all raw byte indexing. The head/tail newline counting was also updated to operate on the already-collected char strings rather than re-scanning with byte offsets.

---

### WR-01: Dead `_http` parameter silently discards the shared HTTP client in `BridgeManager`

**Files modified:** `crates/bridge/src/lib.rs`
**Commit:** 999b292
**Applied fix:** Removed the `_http: reqwest::Client` parameter from `start_bridge_with_client` (Option A from the review). Updated both call sites in `BridgeManager::start` and `start_bridge`. Removed the now-unused `http: reqwest::Client` field from `BridgeManager` and simplified `BridgeManager::new` accordingly.

---

### WR-02: `BridgeOutbound::ToolEnd` has no `name` field — web UI receives empty `tool_name`

**Files modified:** `crates/bridge/src/lib.rs`, `crates/cli/src/main.rs`
**Commit:** 4d33bf1
**Applied fix:** Added `name: String` field to `BridgeOutbound::ToolEnd`. Updated the pattern match in `run_bridge_loop` to destructure `name` and forward it as `tool_name` to `BridgeEvent::ToolEnd` instead of `String::new()`. Updated the construction site in `cli/src/main.rs` to pass `tool_name.clone()` from `QueryEvent::ToolEnd` (which already carries the name).

---

### WR-03: Resolver comments number "Priority 4" and "D-02 condition 4/5" with no Priority 3 or conditions 2/3

**Files modified:** `crates/core/src/lib.rs`
**Commit:** db668a8
**Applied fix:** Renumbered D-02 conflict-check comments to `condition 1/3`, `condition 2/3`, `condition 3/3` and renamed `Priority 4` to `Priority 3` for the OAuth path. All three actual code paths now have sequential, unambiguous numbering.

---

### WR-04: Commands test helpers use `.unwrap()` on mutex — cascading panics on mutex poison

**Files modified:** `crates/commands/src/lib.rs`
**Commit:** d73e529
**Applied fix:** Replaced all three `.lock().unwrap()` calls in the bearer-auth `StatusCommand` test helpers with `.lock().unwrap_or_else(|p| p.into_inner())`, matching the poison-tolerant pattern already used in `crates/cli/src/main.rs`.

---

### IN-01: `CLAUDE_ORANGE` constant is named "orange" but is a pink/magenta colour

**Files modified:** `crates/tui/src/theme_colors.rs`, `crates/tui/src/render.rs`, `crates/tui/src/messages/mod.rs`, `crates/tui/src/prompt_input.rs`
**Commit:** 5afc2cc
**Applied fix:** Renamed `CLAUDE_ORANGE` to `BRAND_PINK` in `theme_colors.rs` (the definition) and updated all import and usage sites in `render.rs` and `messages/mod.rs`. In `prompt_input.rs`, removed the duplicate local constant definition and replaced it with `use crate::theme_colors::BRAND_PINK;` so it references the single shared constant going forward.

---

### IN-02: `poll_bridge_messages`, `post_bridge_response`, and `post_bridge_event` each build a fresh HTTP client per call

**Files modified:** `crates/bridge/src/lib.rs`, `crates/cli/src/main.rs`
**Commit:** 339da19
**Applied fix:** Added `http: &reqwest::Client` parameter to all three public bridge API functions and removed the per-call `reqwest::Client::builder()` construction inside each. In `cli/src/main.rs`, created a single shared `bridge_http` client at the point where both spawned tasks are created and cloned it into each task (`http_relay` and `http_poll`) so both calls reuse the same connection pool.

---

### IN-03: `test_jwt_decode_invalid` contains an always-true assertion

**Files modified:** `crates/bridge/src/lib.rs`
**Commit:** 550d96a
**Applied fix:** Replaced `assert!(JwtClaims::decode("only.two").is_ok() == false || true)` with `assert!(JwtClaims::decode("only.two").is_err(), "invalid base64 payload should be Err")`. Analysis of `JwtClaims::decode` confirms `"only.two"` passes the `parts.len() < 2` guard but `"two"` is not valid base64url, so the function returns `Err` — making `is_err()` the correct, falsifiable assertion.

---

_Fixed: 2026-05-09_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
