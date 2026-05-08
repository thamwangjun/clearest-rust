---
phase: 03-anthropic-auth-token-bearer-auth-support
reviewed: 2026-05-09T00:00:00Z
depth: standard
files_reviewed: 10
files_reviewed_list:
  - crates/core/src/lib.rs
  - crates/core/tests/bearer_auth.rs
  - crates/cli/src/main.rs
  - crates/commands/src/lib.rs
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

**Reviewed:** 2026-05-09
**Depth:** standard
**Files Reviewed:** 10
**Status:** issues_found

## Summary

This phase introduces `ANTHROPIC_AUTH_TOKEN` bearer-auth support in the Anthropic credential resolver and propagates the resulting `use_bearer_auth` flag through the CLI, commands, and bridge subsystems. The core resolver logic is sound, and the conflict-detection checks are well-structured. Two critical bugs were found: a UTF-8 byte-boundary panic in bridge error handling and another in the TUI user-prompt truncation function — both index a `String` by raw bytes where character-level indexing is required. Four warnings cover a dead HTTP-client parameter in the bridge subsystem, a `tool_name` field that is always empty in `ToolEnd` events sent to the web UI, a comment numbering gap in the resolver, and a non-poison-tolerant mutex lock in the commands test helpers. Three informational items cover a constant naming inconsistency, per-call HTTP client construction, and an always-true assertion in a bridge test.

---

## Critical Issues

### CR-01: UTF-8 byte-boundary panic in bridge error body truncation

**File:** `crates/bridge/src/lib.rs:1039`

**Issue:** `&body_text[..body_text.len().min(200)]` indexes into a `String` by raw byte offset. If a server returns a UTF-8 body where byte 200 falls inside a multi-byte codepoint (e.g. a CJK character, emoji, or accented Latin), Rust panics with `byte index N is not a char boundary`. This crash occurs in the error branch of `start_bridge_session`, which is reached for any unexpected HTTP status code from the registration server. Any API server that returns non-ASCII in its error messages will trigger the panic.

**Fix:**
```rust
// Replace the byte slice with a char-boundary-safe truncation:
let preview: String = body_text.chars().take(200).collect();
if body_text.is_empty() {
    String::new()
} else {
    format!("Response: {}", preview)
}
```

---

### CR-02: UTF-8 byte-boundary panic in TUI user-prompt truncation

**File:** `crates/tui/src/messages/mod.rs:1331,1335,1337`

**Issue:** `truncate_user_prompt_text` compares and slices a `&str` by byte lengths throughout, while the constants are named `*_CHARS` suggesting character counts:

- Line 1331: `text.len() <= MAX_USER_PROMPT_DISPLAY_CHARS` — compares byte length against 10 000, not char count.
- Line 1335: `&text[..TRUNCATE_USER_PROMPT_HEAD_CHARS.min(text.len())]` — byte-indexes at offset 2 500; panics if byte 2 500 splits a multi-byte codepoint.
- Line 1337: `&text[tail_start..]` — `tail_start = text.len() - 2500` is a byte offset; panics at a non-char-boundary.

Any user prompt containing multi-byte characters (CJK, emoji, Arabic, accented text) longer than 10 000 bytes triggers a panic in the TUI message renderer.

**Fix:**
```rust
fn truncate_user_prompt_text(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= MAX_USER_PROMPT_DISPLAY_CHARS {
        return text.to_string();
    }

    let head: String = text.chars().take(TRUNCATE_USER_PROMPT_HEAD_CHARS).collect();
    let tail: String = text
        .chars()
        .skip(char_count.saturating_sub(TRUNCATE_USER_PROMPT_TAIL_CHARS))
        .collect();

    let head_newlines = head.chars().filter(|c| *c == '\n').count();
    let tail_newlines = tail.chars().filter(|c| *c == '\n').count();
    let total_newlines = text.chars().filter(|c| *c == '\n').count();
    let hidden_lines = total_newlines.saturating_sub(head_newlines + tail_newlines);

    format!("{head}\n… +{hidden_lines} lines …\n{tail}")
}
```

---

## Warnings

### WR-01: Dead `_http` parameter silently discards the shared HTTP client in `BridgeManager`

**File:** `crates/bridge/src/lib.rs:851-886`

**Issue:** `start_bridge_with_client` accepts `_http: reqwest::Client` but never uses it. `BridgeSession::new` always builds its own fresh `reqwest::Client` internally. When `BridgeManager::start` passes `self.http.clone()` to `start_bridge_with_client`, the clone is dropped immediately at function entry. The shared connection pool in `BridgeManager` is never reused; every bridge session creates a new pool, consuming extra OS sockets and TLS sessions.

**Fix:** Either thread the client into `BridgeSession::new` so it is actually reused, or remove the dead parameter from `start_bridge_with_client`:

```rust
// Option A: remove the parameter entirely
async fn start_bridge_with_client(
    config: BridgeConfig,
    cancel: CancellationToken,
) -> anyhow::Result<(...)> { ... }

// Option B: use the passed-in client in BridgeSession
pub fn new(config: BridgeConfig, http: reqwest::Client) -> Self {
    Self { config, session_id: uuid::Uuid::new_v4().to_string(), state: ..., http, ... }
}
```

---

### WR-02: `BridgeOutbound::ToolEnd` has no `name` field — web UI receives empty `tool_name` on every tool completion

**File:** `crates/bridge/src/lib.rs:1326-1330,1551-1558`

**Issue:** `BridgeOutbound::ToolEnd` carries only `id`, `output`, and `is_error`. When mapped to `BridgeEvent::ToolEnd` in `run_bridge_loop`, `tool_name` is hardcoded to `String::new()`:

```rust
Some(BridgeOutbound::ToolEnd { id, output, is_error }) => {
    let _ = bridge_ev_tx.send(BridgeEvent::ToolEnd {
        tool_name: String::new(),   // always empty
        tool_id: id,
        result: output,
        is_error,
    }).await;
}
```

The web UI receives a `tool_end` event with `tool_name: ""` for every tool call, making it impossible to display which tool completed. `BridgeEvent::ToolEnd.tool_name` is serialized over the wire in `#[serde(tag = "type")]` JSON, so the web UI cannot recover the name from any other field.

**Fix:** Add `name: String` to `BridgeOutbound::ToolEnd` and thread it through:

```rust
// Enum variant:
ToolEnd { id: String, name: String, output: String, is_error: bool },

// Mapping in run_bridge_loop:
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

### WR-03: Resolver comments number "Priority 4" and "D-02 condition 4/5" with no Priority 3 or conditions 2/3

**File:** `crates/core/src/lib.rs:1293,1301,1309,1325`

**Issue:** The comment sequence in `resolve_anthropic_auth_async` jumps from `// Priority 2` directly to `// Priority 4`, and the D-02 conflict-check comments jump from `condition 1` to `condition 4` and `condition 5` with conditions 2 and 3 absent. This creates false ambiguity: a future maintainer may believe there are missing code paths and introduce duplicate or conflicting logic to fill the apparent gaps.

**Fix:** Renumber comments to match the actual code paths (1, 2, 3) or add inline notes explaining which originally-planned conditions were consolidated into existing checks:

```rust
// Priority 1: ANTHROPIC_AUTH_TOKEN env — bearer path, checked before x-api-key
// Priority 2: x-api-key path (provider_configs.api_key → ANTHROPIC_API_KEY env → top-level api_key)
// Priority 3: OAuth tokens (stored credentials)
```

---

### WR-04: Commands test helpers use `.unwrap()` on mutex — cascading panics on mutex poison

**File:** `crates/commands/src/lib.rs:8616,8632,8647`

**Issue:** All three bearer-auth `StatusCommand` tests lock the serialization mutex with `.lock().unwrap()`. If any of the three tests panics while holding the lock (e.g. due to an assertion failure after env mutation), the mutex becomes poisoned. All subsequent tests calling `.lock().unwrap()` then also panic on `PoisonError`, producing cascading spurious failures. The fix for the identical pattern in `crates/cli/src/main.rs` (using `.unwrap_or_else(|p| p.into_inner())`) was not applied here.

**Fix:**
```rust
let _guard = env_test_mutex().lock().unwrap_or_else(|p| p.into_inner());
```

---

## Info

### IN-01: `CLAUDE_ORANGE` constant is named "orange" but is a pink/magenta colour

**File:** `crates/tui/src/theme_colors.rs:10`

**Issue:** The constant `CLAUDE_ORANGE` has the value `Color::Rgb(233, 30, 99)`, which is a deep pink/magenta, not orange. The doc comment on the same line says "brand pink/magenta", which directly contradicts the identifier name. The name is used in over 15 call sites across `render.rs` and `messages/mod.rs`. Additionally, `crates/tui/src/prompt_input.rs:20` contains a duplicate local definition (`const CLAUDE_ORANGE: Color = Color::Rgb(233, 30, 99)`) that bypasses the shared constant in `theme_colors.rs`, creating a risk that the two go out of sync.

**Fix:** Rename to `CLAURST_BRAND_PINK` or `BRAND_ACCENT` throughout all 15+ call sites, and replace the duplicate definition in `prompt_input.rs` with an import of the shared constant.

---

### IN-02: `poll_bridge_messages`, `post_bridge_response`, and `post_bridge_event` each build a fresh HTTP client per call

**File:** `crates/bridge/src/lib.rs:1079-1083,1158-1162,1222-1226`

**Issue:** Each of the three high-level bridge API functions constructs a new `reqwest::Client` on every invocation. `reqwest::Client` is designed to be cloned and reused; it maintains an internal connection pool. Building a new client per call discards keep-alive connections, forces fresh TCP/TLS handshakes on every poll cycle, and wastes memory. For `poll_bridge_messages`, which is called in a tight loop, this is a per-iteration allocation.

**Fix:** Accept an `http: &reqwest::Client` parameter in each function, or store a shared client in `BridgeSessionInfo` so callers reuse a single pool:

```rust
pub async fn poll_bridge_messages(
    info: &BridgeSessionInfo,
    http: &reqwest::Client,   // caller-provided
    since_id: Option<&str>,
) -> anyhow::Result<Vec<SimpleMessage>> { ... }
```

---

### IN-03: `test_jwt_decode_invalid` contains an always-true assertion

**File:** `crates/bridge/src/lib.rs:1652`

**Issue:** The test contains:
```rust
assert!(JwtClaims::decode("only.two").is_ok() == false || true); // either way, must not panic
```
The expression `... || true` is always `true` regardless of the left operand. This assertion can never fail and provides zero test coverage. The comment acknowledges uncertainty about the expected outcome; the fix is to decide and commit to a contract.

**Fix:** If a two-segment string (missing the payload segment at `parts[1]`) should succeed because the code checks `parts.len() < 2` and `"only.two".split('.').count() == 2`, then the test should assert `is_ok()`. If the payload segment `"two"` is not valid base64url, it should assert `is_err()`. Either way, make the assertion falsifiable:

```rust
// "only.two" splits to ["only", "two"]; parts.len() == 2 passes the length guard
// but "two" is not valid base64url -> decode returns Err
assert!(JwtClaims::decode("only.two").is_err(), "invalid base64 payload should be Err");
```

---

_Reviewed: 2026-05-09_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
