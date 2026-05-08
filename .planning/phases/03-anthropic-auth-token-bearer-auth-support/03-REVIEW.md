---
phase: 03-anthropic-auth-token-bearer-auth-support
reviewed: 2026-05-08T00:00:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - crates/cli/src/main.rs
  - crates/commands/src/lib.rs
  - crates/core/src/lib.rs
  - crates/core/tests/bearer_auth.rs
  - crates/bridge/src/lib.rs
  - crates/tui/src/messages/mod.rs
  - crates/tui/src/render.rs
  - crates/tui/src/theme_colors.rs
  - crates/tui/tests/render_snapshots.rs
  - crates/tui/tests/startup_routing.rs
findings:
  critical: 2
  warning: 4
  info: 3
  total: 9
status: issues_found
---

# Phase 03: Code Review Report

**Reviewed:** 2026-05-08
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

This phase adds `ANTHROPIC_AUTH_TOKEN` bearer-auth resolution, conflict detection (D-02), and `use_bearer_auth` pinning to `Config::resolve_anthropic_auth_async()`. The core resolver logic in `crates/core/src/lib.rs` is mostly correct; the bearer-auth tests in `crates/core/tests/bearer_auth.rs` cover the main happy-path and conflict cases.

Two critical issues were found: a security vulnerability in `crates/bridge/src/lib.rs` where a live authentication token is exposed in derived `Debug` and `Serialize` output, and a missing conflict-detection branch in the resolver (silent wrong behaviour when `use_bearer_auth=true` is set alongside the top-level `Config.api_key`). Four warnings cover a busy-loop risk, a retry off-by-one, a data-loss mapping, and incorrect character-vs-width handling in a TUI helper. Three info items cover a misleading constant name, a dead-code enum variant, and a tautological test assertion.

---

## Critical Issues

### CR-01: `BridgeSessionInfo` exposes auth token via `derive(Debug)` and `derive(Serialize)`

**File:** `crates/bridge/src/lib.rs:894`

**Issue:** `BridgeSessionInfo` holds a live bearer token in its `pub token: String` field and derives both `Debug` and `Serialize` without any redaction. Any call to `format!("{:?}", info)`, any `tracing` log that records the struct, or any `serde_json::to_string(&info)` will emit the token in plaintext. The hand-written `Display` implementation correctly omits the token, but `Debug` and `Serialize` do not. By contrast, `BridgeConfig::session_token` (line 192) has a manual `Debug` implementation specifically to prevent this exact leak.

**Fix:**
```rust
// Replace #[derive(Debug, Clone, Serialize, Deserialize)] with:
#[derive(Clone, Serialize, Deserialize)]
pub struct BridgeSessionInfo {
    pub session_id: String,
    pub session_url: String,
    #[serde(skip_serializing)]
    pub token: String,
}

impl std::fmt::Debug for BridgeSessionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BridgeSessionInfo")
            .field("session_id", &self.session_id)
            .field("session_url", &self.session_url)
            .field("token", &"<redacted>")
            .finish()
    }
}
```

---

### CR-02: Missing conflict check — `use_bearer_auth=true` + top-level `Config.api_key` silently produces `Ok(None)`

**File:** `crates/core/src/lib.rs:1316-1334`

**Issue:** The D-02 conflict-detection matrix in `resolve_anthropic_auth_async` has four checks but is missing one combination:

| Checked | Condition |
|---|---|
| Yes | `env_api_key` + `env_auth_token` (condition 1) |
| Yes | `use_bearer_pinned` + `env_api_key` (condition 2) |
| Yes | `use_bearer_pinned` + `provider_api_key` (condition 3) |
| Yes | `top_level_api_key` + `env_auth_token` (condition 4) |
| **No** | **`use_bearer_pinned` + `top_level_api_key`** |

When a user sets `provider_configs.anthropic.use_bearer_auth = true` in settings and also has `api_key` set at the top-level `Config` (e.g. via `--api-key` CLI flag or `"api_key"` at the settings root), the code silently reaches Priority 1 and returns `Ok(env_auth_token.map(|t| (t, true)))`. If no `ANTHROPIC_AUTH_TOKEN` env var is set, this returns `Ok(None)` — ignoring the conflicting api_key entirely with no error. The user believes bearer auth is configured but the session falls through to the "no credentials" onboarding flow.

**Fix:** Add the missing guard immediately after condition 3:
```rust
// D-02 condition 3b: bearer pin + top-level Config.api_key
if use_bearer_pinned && top_level_api_key.is_some() {
    anyhow::bail!(
        "provider_configs.anthropic.use_bearer_auth=true conflicts with \
         top-level api_key in Config (x-api-key mode). \
         Remove api_key or set use_bearer_auth=false."
    );
}
```
Add a corresponding test in `crates/core/tests/bearer_auth.rs`.

---

## Warnings

### WR-01: `run_bridge_loop` near-busy-loops when `outbound_rx` sender is dropped

**File:** `crates/bridge/src/lib.rs:1526-1531`

**Issue:** In the `run_bridge_loop` `tokio::select!` block, when the `outbound_rx` sender is dropped (query loop exits), `outbound_rx.recv()` returns `Poll::Ready(None)` instantly on every poll. `tokio::select!` picks randomly among ready futures, so the `outbound` arm wins most iterations, continuously executing the `None => { /* nothing */ }` branch and starving the `tokio::time::sleep(poll_interval)` arm. This causes near-100% CPU usage for the bridge task after the query loop exits.

**Fix:** Break out of the loop when the outbound channel closes:
```rust
None => {
    // Query loop exited; no more outbound events to forward.
    // Exit cleanly rather than spinning.
    break;
}
```

---

### WR-02: `poll_bridge_messages` off-by-one — retries 4 times when `max_retries = 3`

**File:** `crates/bridge/src/lib.rs:1115-1119`

**Issue:** `attempt` starts at `0` and is incremented before the `> max_retries` check. With `max_retries = 3`, the bail condition `attempt > 3` is only `true` when `attempt = 4`, meaning four 429-retry attempts are made before failing. The error message also lies, reporting "after 3 retries" when 4 were actually performed.

Trace:
- attempt 1: `1 > 3` false → sleep, retry
- attempt 2: `2 > 3` false → sleep, retry
- attempt 3: `3 > 3` false → sleep, retry
- attempt 4: `4 > 3` true → bail! "after 3 retries" (wrong)

**Fix:** Change `>` to `>=`:
```rust
if attempt >= max_retries {
    anyhow::bail!(
        "poll_bridge_messages: rate-limited (HTTP 429) after {} retries",
        max_retries
    );
}
```

---

### WR-03: `BridgeOutbound::ToolEnd` → `BridgeEvent::ToolEnd` mapping sends empty `tool_name`

**File:** `crates/bridge/src/lib.rs:1551-1559`

**Issue:** When translating `BridgeOutbound::ToolEnd` into `BridgeEvent::ToolEnd`, `tool_name` is hardcoded to `String::new()`. The `BridgeOutbound::ToolEnd` struct lacks a `name` field (unlike `BridgeOutbound::ToolStart` which carries `name: String`), so the tool name is silently dropped. The web UI receives a `tool_end` event with `tool_name: ""`, breaking any UI logic that correlates `ToolStart`/`ToolEnd` events by name.

```rust
// Current — tool_name is always empty:
Some(BridgeOutbound::ToolEnd { id, output, is_error }) => {
    let _ = bridge_ev_tx.send(BridgeEvent::ToolEnd {
        tool_name: String::new(),  // data loss
        tool_id: id,
        result: output,
        is_error,
    }).await;
}
```

**Fix:** Add a `name` field to `BridgeOutbound::ToolEnd` and propagate it:
```rust
// BridgeOutbound enum:
ToolEnd {
    id: String,
    name: String,
    output: String,
    is_error: bool,
},

// run_bridge_loop mapping:
Some(BridgeOutbound::ToolEnd { id, name, output, is_error }) => {
    let _ = bridge_ev_tx.send(BridgeEvent::ToolEnd {
        tool_name: name,
        tool_id: id,
        result: output,
        is_error,
    }).await;
}
```

---

### WR-04: `truncate_middle` mixes display-width and character-count — overflows layout for wide characters

**File:** `crates/tui/src/render.rs:170-180`

**Issue:** `keep_each_side` is derived from `max_width` (measured in terminal display columns), but `text.chars().take(keep_each_side)` selects by Unicode scalar count. For text containing wide characters (CJK, fullwidth, emoji with display width 2), each character consumes 2 display columns. With 20 wide characters and `max_width = 20`, `keep_each_side = 9`, the resulting string is `9*2 + 1 + 9*2 = 37` columns wide — nearly double `max_width` — causing the string to overflow its TUI layout cell and corrupt adjacent panels.

```rust
let keep_each_side = (max_width.saturating_sub(1)) / 2; // display-width budget
let left: String = text.chars().take(keep_each_side).collect(); // char-count — wrong for wide chars
```

**Fix:** Accumulate characters while tracking display width:
```rust
fn take_chars_by_width(s: &str, budget: usize) -> String {
    let mut out = String::new();
    let mut used = 0;
    for ch in s.chars() {
        let w = UnicodeWidthStr::width(ch.encode_utf8(&mut [0u8; 4]));
        if used + w > budget { break; }
        out.push(ch);
        used += w;
    }
    out
}
// In truncate_middle:
let left = take_chars_by_width(text, keep_each_side);
let right_chars: Vec<char> = text.chars().rev().collect();
let right_rev: String = right_chars.iter().collect();
let right = take_chars_by_width(&right_rev, keep_each_side)
    .chars().rev().collect::<String>();
format!("{left}\u{2026}{right}")
```

---

## Info

### IN-01: `CLAUDE_ORANGE` constant is pink/magenta, not orange

**File:** `crates/tui/src/theme_colors.rs:10`

**Issue:** The constant `CLAUDE_ORANGE` is defined as `Color::Rgb(233, 30, 99)`, which is a deep pink/magenta. The doc comment correctly identifies it as "brand pink/magenta". The name `CLAUDE_ORANGE` is misleading and creates confusion when tracing UI color choices or searching for orange-colored elements.

**Fix:** Rename to `CLAUDE_PINK` or `CLAURST_ACCENT` and update all use sites in `render.rs` and `messages/mod.rs`.

---

### IN-02: `PermissionResponseKind::AllowSession` is dead code — never produced by `run_bridge_loop`

**File:** `crates/bridge/src/lib.rs:1269`

**Issue:** The only producer of `PermissionResponseKind` values is the `PermissionDecision` mapping in `run_bridge_loop` (lines 1493–1499), which only maps to `Allow` and `Deny`. The `AllowSession` variant is never emitted, making it unreachable dead code. Downstream match arms for `AllowSession` can never execute.

**Fix:** Either remove `AllowSession` from the enum, or explicitly map `PermissionDecision::AllowPermanently` to `PermissionResponseKind::AllowSession` if the session-scoped allow semantics are intentional.

---

### IN-03: Tautological assertion in `test_jwt_decode_invalid` — always passes

**File:** `crates/bridge/src/lib.rs:1652`

**Issue:** The second assertion is always `true`:
```rust
assert!(JwtClaims::decode("only.two").is_ok() == false || true);
// Equivalent to: assert!(true)
```
The `|| true` makes the condition vacuous — it passes regardless of what the function returns. The test exercises no code path and provides no safety net against regressions.

**Fix:** Replace with a meaningful check:
```rust
// "only.two" has no valid base64url payload — should either error or parse to empty claims.
// At minimum verify it does not panic (no assert needed), or assert the error case:
assert!(
    JwtClaims::decode("only.two").is_err(),
    "two-segment token with non-base64url payload should be rejected"
);
```

---

_Reviewed: 2026-05-08_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
