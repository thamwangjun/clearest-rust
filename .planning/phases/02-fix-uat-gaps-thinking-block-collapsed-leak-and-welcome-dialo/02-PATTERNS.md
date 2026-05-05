# Phase 2: Fix UAT Gaps — Thinking Block Collapsed Leak and Welcome Dialog Startup Routing - Pattern Map

**Mapped:** 2026-05-05
**Files analyzed:** 5 (4 modified, 1 created)
**Analogs found:** 5 / 5

---

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/tui/src/messages/mod.rs` | renderer + struct | transform | self (current impl is the bug site) | self-fix |
| `crates/tui/src/render.rs` | renderer | transform | self (two RenderContext struct literal sites) | self-fix |
| `crates/tui/tests/render_snapshots.rs` | test | — | self (existing `thinking_block_collapsed` test) | self-fix |
| `crates/cli/src/main.rs` | startup routing | request-response | self (line 1435 is the bug site) | self-fix |
| `crates/tui/tests/startup_routing.rs` | test | — | `crates/tui/tests/render_snapshots.rs` | exact role-match |

---

## Pattern Assignments

### `crates/tui/src/messages/mod.rs` — RenderContext struct + render_thinking_block

**Bugs being fixed:**
1. `RenderContext` (line 30) lacks `frame_count: u64` field — add it.
2. `render_thinking_block` (line 1245) calls `reasoning_heading(text)` unconditionally in both collapsed and expanded paths, leaking content text in collapsed mode.

**Current RenderContext struct** (`messages/mod.rs` lines 30–54):
```rust
pub struct RenderContext {
    pub width: u16,
    pub highlight: bool,
    pub show_thinking: bool,
    pub tool_names: HashMap<String, String>,
    pub expanded_thinking: std::collections::HashSet<u64>,
}

impl Default for RenderContext {
    fn default() -> Self {
        Self {
            width: 80,
            highlight: true,
            show_thinking: false,
            tool_names: HashMap::new(),
            expanded_thinking: std::collections::HashSet::new(),
        }
    }
}
```

**Add field pattern** — append `frame_count: u64` to both struct definition and `impl Default`:
```rust
// In struct body after expanded_thinking field:
pub frame_count: u64,

// In impl Default after expanded_thinking value:
frame_count: 0,
```

**Current render_thinking_block** (`messages/mod.rs` lines 1245–1267) — the bug:
```rust
pub fn render_thinking_block(text: &str, expanded: bool) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let heading = reasoning_heading(text).unwrap_or_else(|| "Thinking".to_string());
    lines.push(Line::from(vec![
        Span::styled(
            "Thinking: ",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        ),
        Span::styled(
            heading,  // BUG: leaks content in collapsed mode
            Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
        ),
    ]));
    if expanded {
        for line in text.lines() {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(line.to_string(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    }
    lines
}
```

**frame_count animation pattern** — from `shimmer_spans` at `render.rs` line 1868:
```rust
// shimmer_spans reference — one step every 4 frames (~200ms at 50ms/frame)
let cycle_pos = (frame_count as usize / 4) % cycle_len;
```

Apply the same cadence for dots (D-01):
```rust
let dot_count = ((frame_count / 4) % 3) + 1; // 1, 2, or 3
let dots = ".".repeat(dot_count as usize);
```
At `frame_count = 0`: `(0/4) % 3 = 0`, `dot_count = 1`, dots = `"."` — satisfies test assertions `contains("Thinking")` and `!contains("hidden thoughts")`.

**Span styling pattern** — match the existing `DarkGray` + `ITALIC` style used for the dots span (note: current collapsed span uses `Color::Gray` for the heading; fix changes it to `Color::DarkGray` to match — content should not draw attention):
```rust
Span::styled(
    dots,
    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
)
```

**Call site for `render_thinking_block`** (`messages/mod.rs` line 1456) — currently passes no frame_count:
```rust
lines.extend(prefix_message_lines(
    render_thinking_block(&thinking, expanded),
    &msg.role,
    ctx.width,
));
```
After fix, passes `ctx.frame_count` as third argument:
```rust
lines.extend(prefix_message_lines(
    render_thinking_block(&thinking, expanded, ctx.frame_count),
    &msg.role,
    ctx.width,
));
```

---

### `crates/tui/src/render.rs` — Two RenderContext construction sites

**Context:** `RenderContext` is imported at `render.rs` line 38 and constructed as an explicit struct literal at two sites. Both sites must add `frame_count: app.frame_count`.

**First site** (`render.rs` lines 1115–1121) — inside `append_turn_items`-like function, `app` not in direct scope at this level (but `frame_count` is passed as a direct parameter — see line 1261 `app.frame_count`). The struct literal at lines 1115–1121 is inside `render_transcript_assistant_message_tagged`; the enclosing function receives `frame_count: u64` as a parameter (confirmed: line 1130 uses `frame_count` directly):
```rust
&RenderContext {
    width,
    highlight: true,
    show_thinking: false,
    tool_names: tool_names.clone(),
    expanded_thinking: expanded_thinking.clone(),
    // ADD: frame_count,  ← already in scope as parameter
},
```

**Second site** (`render.rs` lines 1271–1277) — inside the VirtualList build closure where `app` is in scope (confirmed: `app.thinking_expanded.clone()` at line 1276 and `app.frame_count` at line 1261 and 1287):
```rust
&RenderContext {
    width,
    highlight: true,
    show_thinking: false,
    tool_names: tool_names.clone(),
    expanded_thinking: app.thinking_expanded.clone(),
    // ADD: frame_count: app.frame_count,  ← app is in scope
},
```

**Verification that `app` is in scope at second site** — line 1287 already reads `app.frame_count`:
```rust
render_tool_block_lines(&mut lines, block, app.frame_count);
```

---

### `crates/tui/tests/render_snapshots.rs` — Update `thinking_block_collapsed` test

**Failing test** (`render_snapshots.rs` lines 158–165):
```rust
#[test]
fn thinking_block_collapsed() {
    let lines = render_thinking_block("hidden thoughts", false);
    assert_eq!(lines.len(), 1);
    let text = flatten(&lines);
    assert!(text.contains("Thinking"));
    assert!(!text.contains("hidden thoughts"));
}
```

**Fix pattern** — add `0` as third argument (D-03/D-06). Frame 0 produces `"."` so `contains("Thinking")` passes and `!contains("hidden thoughts")` passes:
```rust
#[test]
fn thinking_block_collapsed() {
    let lines = render_thinking_block("hidden thoughts", false, 0);
    assert_eq!(lines.len(), 1);
    let text = flatten(&lines);
    assert!(text.contains("Thinking"));
    assert!(!text.contains("hidden thoughts"));
}
```

**Existing expanded test** (`render_snapshots.rs` lines 167–173) also needs `frame_count` added — `0` is correct here too (frame_count only affects collapsed path, but signature requires it):
```rust
#[test]
fn thinking_block_expanded() {
    let lines = render_thinking_block("my thoughts here", true, 0);
    // ...
}
```

**Imports pattern** (`render_snapshots.rs` lines 1–12) — `render_thinking_block` and `RenderContext` are already imported:
```rust
use claurst_tui::messages::{
    render_assistant_text, render_user_text, render_tool_use,
    render_tool_result_success, render_tool_result_error,
    render_compact_boundary, render_summary_message,
    render_unseen_divider, render_system_message, render_thinking_block,
    render_rate_limit_banner, render_hook_progress, render_code_block,
    render_user_command, render_user_memory_input, render_user_local_command_output,
    RenderContext,
};
```

**`RenderContext` struct literal pattern used in tests** (`render_snapshots.rs` line 31):
```rust
let ctx = RenderContext { width: 80, highlight: true, show_thinking: false, ..Default::default() };
```
After adding `frame_count: 0` to `impl Default`, this spread syntax continues to work — the new field gets `0` from Default automatically.

---

### `crates/cli/src/main.rs` — Startup routing one-word fix

**Bug context** (`main.rs` lines 1430–1443):
```rust
// Show onboarding: status hint if no credentials, welcome tour if first run.
if !has_credentials {
    if !settings.has_completed_onboarding {
        app.onboarding_dialog.show_provider_setup();  // BUG: line 1435
    } else {
        app.status_message = Some("No provider configured. Run /connect to set one up.".to_string());
    }
} else if !settings.has_completed_onboarding {
    let _ = claurst_tui::App::persist_onboarding_complete_pub();
}
```

**Fix** — change `show_provider_setup()` to `show()` on line 1435 only. No other call sites touched (D-04):
```rust
app.onboarding_dialog.show();  // was show_provider_setup()
```

**Method definitions confirming the fix** (`onboarding_dialog.rs` lines 47–56):
```rust
/// Show the normal onboarding (first-run with credentials already configured).
pub fn show(&mut self) {
    self.visible = true;
    self.page = OnboardingPage::Welcome;  // ← correct starting page
}

/// Show the provider setup page (no credentials configured).
pub fn show_provider_setup(&mut self) {
    self.visible = true;
    self.page = OnboardingPage::ProviderSetup;  // ← was incorrectly used
}
```

---

### `crates/tui/tests/startup_routing.rs` — New test file (D-07)

**Analog:** `crates/tui/tests/render_snapshots.rs` — exact same structure: `use` imports from `claurst_tui`, `fn flatten()` helper, plain `#[test]` functions with direct assertions, no App init needed.

**File header pattern** (from `render_snapshots.rs` lines 1–3):
```rust
//! T5-2: Message renderer snapshot tests.
//! Renders each message type and verifies key content in returned Lines.
```

**Test structure pattern** (from `render_snapshots.rs` lines 29–36):
```rust
#[test]
fn assistant_text_renders_lines() {
    let ctx = RenderContext { width: 80, highlight: true, show_thinking: false, ..Default::default() };
    let lines = render_assistant_text("Hello, world!\n\nSecond paragraph.", &ctx);
    assert!(!lines.is_empty());
    let combined = flatten(&lines);
    assert!(combined.contains("Hello"));
}
```

**Full new test file** (`crates/tui/tests/startup_routing.rs`):
```rust
//! Startup routing tests — verifies that show() opens the Welcome page,
//! not ProviderSetup. Covers D-07 from phase 2 CONTEXT.md.

use claurst_tui::onboarding_dialog::{OnboardingDialogState, OnboardingPage};

#[test]
fn show_starts_at_welcome_page() {
    let mut dialog = OnboardingDialogState::new();
    dialog.show();
    assert!(dialog.visible, "dialog should be visible after show()");
    assert_eq!(dialog.page, OnboardingPage::Welcome, "show() must start at Welcome, not ProviderSetup");
}
```

**Exported types confirming test compiles** (`onboarding_dialog.rs` lines 23–39):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OnboardingPage {
    #[default]
    ProviderSetup,
    Welcome,
    KeyBindings,
    Done,
}

#[derive(Debug, Default, Clone)]
pub struct OnboardingDialogState {
    pub visible: bool,
    pub page: OnboardingPage,
}
```
`OnboardingPage` derives `PartialEq` — `assert_eq!` compiles. `OnboardingDialogState::new()` delegates to `Default::default()`.

---

## Shared Patterns

### Span styling — DarkGray Italic
**Source:** `crates/tui/src/messages/mod.rs` lines 1249–1252
**Apply to:** The new animated dots span in collapsed `render_thinking_block`
```rust
Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC)
```

### Frame-count animation cadence
**Source:** `crates/tui/src/render.rs` line 1868 (`shimmer_spans`)
**Apply to:** Dot cycling in collapsed `render_thinking_block`
```rust
// One step every 4 frames ≈ 200ms at 50ms/frame
let cycle_pos = (frame_count as usize / 4) % cycle_len;
```
Adapted for dots: `((frame_count / 4) % 3) + 1`

### Struct-literal construction with spread
**Source:** `crates/tui/tests/render_snapshots.rs` line 31
**Apply to:** All test files using `RenderContext`
```rust
let ctx = RenderContext { width: 80, highlight: true, show_thinking: false, ..Default::default() };
```
After `frame_count: 0` is added to `impl Default`, this syntax automatically covers the new field.

---

## No Analog Found

None — all five files have clear analogs or are self-fixes with verified line numbers.

---

## Metadata

**Analog search scope:** `crates/tui/src/`, `crates/tui/tests/`, `crates/cli/src/`
**Files scanned:** 6 source files read directly
**Pattern extraction date:** 2026-05-05
