# Phase 2: Fix UAT Gaps — Thinking Block Collapsed Leak and Welcome Dialog Startup Routing - Research

**Researched:** 2026-05-05
**Domain:** Rust TUI rendering (ratatui), test update, startup routing
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-01:** In collapsed mode (`expanded = false`), `render_thinking_block` must NOT call `reasoning_heading(text)`. Instead, render an animated `...` placeholder that cycles through `.` → `..` → `...` every 4 frames (~250ms at 60fps). The heading span shows `"..."` (current dot count), not any content derived from the thinking text.
- **D-02:** Animation is driven by `frame_count`. Add `frame_count: u64` (default 0) to `RenderContext`. Update both `RenderContext` construction sites in `render.rs` to supply `app.frame_count`. Pass `ctx.frame_count` through to `render_thinking_block` as a new parameter.
- **D-03:** The existing `thinking_block_collapsed` test in `render_snapshots.rs` must be updated to pass `frame_count = 0` (or equivalent) so it still asserts `contains("Thinking")` and `!contains("hidden thoughts")`. At frame 0, dots cycle to `"."` which satisfies both assertions.
- **D-04:** Change `main.rs:1435` from `app.onboarding_dialog.show_provider_setup()` to `app.onboarding_dialog.show()`. One line. No other call sites touched.
- **D-05:** After the Welcome → KeyBindings → dismiss flow completes, a no-credentials user sees the existing status hint: `"No provider configured. Run /connect to set one up."` — same as the `else` branch that fires when `has_completed_onboarding = true` and no credentials. No new behavior, no auto-opening of ProviderSetup.
- **D-06:** Fix the existing failing `thinking_block_collapsed` test (render_snapshots.rs line 159) — update its `render_thinking_block` call to include the new `frame_count` parameter.
- **D-07:** Add a new test file `crates/tui/tests/startup_routing.rs`. It must contain at least one test that:
  - Calls `onboarding_dialog.show()` directly
  - Asserts the dialog is visible and the starting page is Welcome (not ProviderSetup)
  This confirms the correct starting state that `main.rs:1435` should produce after the fix.

### Claude's Discretion

- Exact cycling formula for dots (`(frame_count / 4) % 3` or similar — planner picks the cleanest arithmetic)
- Whether `frame_count` is added directly to `render_thinking_block`'s parameter list or accessed via `RenderContext` — whichever requires fewer call-site changes
- Exact span styling for the animated `"..."` (match existing DarkGray italic or a slight variation)

### Deferred Ideas (OUT OF SCOPE)

- Auto-opening ProviderSetup after the Welcome→KeyBindings flow completes — declined, status hint is sufficient
- Auditing all `show_provider_setup()` call sites — out of scope, targeted fix only
- Replacing `reasoning_heading()` with a smarter summary for collapsed mode — out of scope; animated `...` is the chosen approach
</user_constraints>

---

## Summary

This phase fixes exactly two pre-existing bugs surfaced by Phase 1 UAT. Both bugs are fully reproduced and root-caused; no exploratory research is needed — the fixes are surgical and fully constrained by CONTEXT.md decisions.

**Bug 1 — Thinking block collapsed content leak:** `render_thinking_block` at `crates/tui/src/messages/mod.rs:1245` unconditionally calls `reasoning_heading(text)` in both collapsed and expanded mode. `reasoning_heading` returns the first non-empty line of the thinking content (up to 72 chars), making it leak verbatim into the collapsed header span. The fix requires (a) adding `frame_count: u64` to `RenderContext`, (b) updating two `RenderContext` construction sites in `render.rs` to pass `app.frame_count`, (c) changing `render_thinking_block`'s signature to accept `frame_count`, and (d) replacing the `reasoning_heading` call in the collapsed branch with an animated dot string derived from `frame_count`.

**Bug 2 — Welcome dialog startup routing:** `main.rs:1435` calls `show_provider_setup()` instead of `show()` for first-run no-credentials users. `show_provider_setup()` sets the initial page to `OnboardingPage::ProviderSetup`; `show()` sets it to `OnboardingPage::Welcome`. This is a one-character change (method name swap) with no other logic involved.

**Primary recommendation:** Implement all locked decisions in order. `RenderContext.frame_count` is the right threading mechanism — it keeps call-site changes minimal (two struct literals in `render.rs`, one function signature in `messages/mod.rs`).

---

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Collapsed thinking animation | TUI render layer (`messages/mod.rs`) | App state (`App.frame_count`) | Animation state lives in App; renderer reads it via RenderContext |
| frame_count threading | TUI render layer (`render.rs`) | — | RenderContext is the established data-passing pattern for render options |
| Startup page routing | CLI startup (`main.rs`) | TUI state (`onboarding_dialog.rs`) | Routing decision is made at startup; dialog state just records which page |
| Test coverage | Test layer (`crates/tui/tests/`) | — | Pure unit tests against public TUI functions |

---

## Standard Stack

No new crates needed for either fix. All work uses existing stack.

### Core (unchanged)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.29 | TUI rendering, `Line`, `Span`, `Style` | Project standard — do not change |
| tokio | 1.44 | Async runtime | Project standard |

### Existing Utilities Referenced
| Utility | Location | Purpose |
|---------|----------|---------|
| `shimmer_spans(text, frame_count)` | `crates/tui/src/render.rs:1858` | Reference implementation for frame-count animation; dots animation uses the same arithmetic (`frame_count / 4 % N`) |
| `RenderContext` | `crates/tui/src/messages/mod.rs:30` | Context struct already threaded through all renderer calls; `frame_count: u64` field added here |
| `OnboardingDialogState` | `crates/tui/src/onboarding_dialog.rs:34` | Exported via `lib.rs:151`; fully testable without App init |
| `reasoning_heading(text)` | `crates/tui/src/transcript_turn.rs:53` | Do NOT modify; just stop calling it in collapsed mode |

---

## Architecture Patterns

### System Architecture Diagram

```
App.frame_count (u64, incremented every render tick)
        │
        ▼
RenderContext { frame_count: u64 }   ← new field (D-02)
        │
        ▼
render_thinking_block(text, expanded, frame_count)   ← new param (D-02)
        │
   expanded == false?
        │
   YES: dots = [".", "..", "..."][(frame_count / 4) % 3]
        │       render "Thinking: {dots}"  ← no content leak
        │
   NO:  reasoning_heading(text) → render header + body lines  (unchanged)
```

```
main.rs startup block (lines 1430–1443)
        │
   !has_credentials && !has_completed_onboarding?
        │
   YES: app.onboarding_dialog.show()   ← D-04 fix (was show_provider_setup())
        │
        ▼
   OnboardingDialogState { visible: true, page: Welcome }
```

### Recommended File Touch List
```
crates/tui/src/messages/mod.rs         # RenderContext field + render_thinking_block signature + collapsed branch logic
crates/tui/src/render.rs               # Two RenderContext struct literals (lines ~1115, ~1271)
crates/tui/tests/render_snapshots.rs   # Update call at line 160 to pass frame_count
crates/cli/src/main.rs                 # One-word change at line 1435
crates/tui/tests/startup_routing.rs    # New test file (D-07)
```

### Pattern 1: frame_count animation (dots cycling)

The `shimmer_spans` implementation at `render.rs:1858` establishes the cadence: one step every 4 frames (`frame_count / 4`), cycling through a range with `% cycle_len`. Apply the same arithmetic for dots:

```rust
// Source: render.rs:1868 (shimmer_spans reference), adapted for dots
let dot_count = ((frame_count / 4) % 3) + 1; // 1, 2, or 3
let dots = ".".repeat(dot_count as usize);
```

At `frame_count = 0`: `(0 / 4) % 3 = 0`, so `dot_count = 1`, dots = `"."`. The test at frame 0 gets `"Thinking: ."` — satisfies `contains("Thinking")` and `!contains("hidden thoughts")`.

The span styling should match existing collapsed-mode DarkGray italic to be consistent with `render_transcript_reasoning_block`:

```rust
// Source: crates/tui/src/messages/mod.rs:1248-1257 (current render_thinking_block)
// Match existing style; just replace heading content with dots
Span::styled(
    dots,  // was: heading derived from reasoning_heading(text)
    Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
)
```

### Pattern 2: RenderContext field addition

`RenderContext` uses explicit struct literals at both construction sites in `render.rs`. Both sites already have access to `app.frame_count` via the `app: &App` parameter. The addition is:

```rust
// Source: crates/tui/src/messages/mod.rs:30 (RenderContext definition)
pub struct RenderContext {
    pub width: u16,
    pub highlight: bool,
    pub show_thinking: bool,
    pub tool_names: HashMap<String, String>,
    pub expanded_thinking: std::collections::HashSet<u64>,
    pub frame_count: u64,  // NEW — add with default 0
}
```

Default impl must be updated:
```rust
impl Default for RenderContext {
    fn default() -> Self {
        Self {
            // ...existing fields...
            frame_count: 0,  // NEW
        }
    }
}
```

Both construction sites in `render.rs` (~line 1115 and ~line 1271) need `frame_count: app.frame_count` added. The second site at ~1271 is inside a plain `render_transcript_assistant_message_tagged` call that does NOT have `frame_count` available in its local scope — but `app` is in scope there (confirmed: `app.thinking_expanded.clone()` is used on line 1276).

### Pattern 3: render_thinking_block signature

D-02 says: "Pass `ctx.frame_count` through to `render_thinking_block` as a new parameter." The decision is to use `RenderContext` as the threading mechanism. The function's current callers are:

1. `render_transcript_assistant_message(msg, ctx)` at `mod.rs:1456` — already has `ctx`; passes `frame_count` from ctx
2. `thinking_block_collapsed` test at `render_snapshots.rs:160` — must pass explicit `frame_count = 0` (D-03/D-06)

Updated signature:
```rust
pub fn render_thinking_block(text: &str, expanded: bool, frame_count: u64) -> Vec<Line<'static>>
```

### Pattern 4: startup_routing.rs test

Follow `render_snapshots.rs` pattern — direct unit test against public TUI types, no App init needed. `OnboardingDialogState` is re-exported at `claurst_tui::onboarding_dialog::OnboardingDialogState` (or via `lib.rs`). Test:

```rust
// crates/tui/tests/startup_routing.rs
use claurst_tui::onboarding_dialog::{OnboardingDialogState, OnboardingPage};

#[test]
fn show_starts_at_welcome_page() {
    let mut dialog = OnboardingDialogState::new();
    dialog.show();
    assert!(dialog.visible);
    assert_eq!(dialog.page, OnboardingPage::Welcome);
}
```

`OnboardingPage` derives `PartialEq` (confirmed at `onboarding_dialog.rs:23`). `OnboardingDialogState` is exported from `lib.rs:151`. No full App init required.

### Anti-Patterns to Avoid

- **Modifying `reasoning_heading()`:** The function is correct. The bug is at the call site. Do not change `reasoning_heading`.
- **Touching other `show_provider_setup()` call sites:** D-04 says "No other call sites touched." There is exactly one call in `main.rs:1435` to fix.
- **Using `shimmer_spans` for the dots animation:** `shimmer_spans` produces a multi-span shimmer effect. The dots animation is simpler — just a string count derived from `frame_count`. Use `".".repeat(dot_count)` directly.
- **Making `render_thinking_block` take `ctx: &RenderContext` instead of `frame_count: u64`:** The function is a pure standalone renderer; adding a full context dependency widens coupling. A single `u64` parameter is sufficient and consistent with how `render_live_thinking_lines` takes `frame_count` directly.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Frame-count animation timing | Custom timer | `frame_count % N` arithmetic | App.frame_count already increments every tick; shimmer_spans proves the pattern |
| Dialog state tracking | Custom state enum | `OnboardingDialogState` (existing) | Already handles page transitions, visible flag |

---

## Common Pitfalls

### Pitfall 1: `RenderContext` Default Not Updated

**What goes wrong:** Adding `frame_count: u64` to the struct but forgetting the `impl Default` block. Tests using `RenderContext { ..Default::default() }` (e.g., `render_snapshots.rs:31`) will fail to compile.

**Why it happens:** Struct literal `..Default::default()` relies on `impl Default` being complete.

**How to avoid:** Update `impl Default for RenderContext` to include `frame_count: 0` at the same time as the struct field addition.

**Warning signs:** Compiler error "missing field `frame_count` in initializer of `RenderContext`" at any `..Default::default()` usage.

### Pitfall 2: Test at frame 0 asserts `contains("Thinking")`

**What goes wrong:** The dot string at frame 0 is `"."`. The heading span becomes `"Thinking: ."`. If the test asserts `contains("Thinking")` this passes — but if the developer changes the label to something other than `"Thinking"`, the test breaks.

**Why it happens:** The test was written expecting the static label `"Thinking"`.

**How to avoid:** The fixed `render_thinking_block` must keep `"Thinking: "` as the label prefix (matching the current implementation's first span), with only the second span changing from `reasoning_heading(text)` to `dots`. Do not rename the label.

### Pitfall 3: Misidentifying the correct `render_thinking_block` call site

**What goes wrong:** There are two separate collapsed-thinking render paths in `messages/mod.rs`:
- `render_transcript_reasoning_block` (line 400) — used by `render_transcript_assistant_message_tagged` (line 480), the tagged path
- `render_thinking_block` (line 1245) — used by the non-tagged assistant message path (line 1456)

The failing test calls `render_thinking_block` directly. Only `render_thinking_block` needs the content-leak fix. `render_transcript_reasoning_block` already correctly uses `reasoning_heading` (its collapsed behavior is intentional — it shows a heading summary, not a dot animation).

**Why it happens:** Both functions serve similar purposes; code navigation may confuse them.

**How to avoid:** The canonical reference is `messages/mod.rs:1245`. The test at `render_snapshots.rs:159` confirms which function is failing.

### Pitfall 4: `app` not in scope at second RenderContext site

**What goes wrong:** The second `RenderContext` construction at `render.rs:1271` — need to confirm `app` is in scope. Review the enclosing function signature.

**How to avoid:** Read the enclosing function before editing. Confirmed: `app.thinking_expanded.clone()` is already used at line 1276, so `app` is in scope. `app.frame_count` is safe to use.

---

## Code Examples

### Current render_thinking_block (the bug)

```rust
// Source: crates/tui/src/messages/mod.rs:1245 [VERIFIED: directly read]
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
        for line in text.lines() { /* ... */ }
    }
    lines
}
```

### Fixed render_thinking_block (target state)

```rust
// Based on: D-01, D-02, shimmer_spans pattern at render.rs:1868 [VERIFIED: directly read]
pub fn render_thinking_block(text: &str, expanded: bool, frame_count: u64) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if expanded {
        let heading = reasoning_heading(text).unwrap_or_else(|| "Thinking".to_string());
        lines.push(Line::from(vec![
            Span::styled(
                "Thinking: ",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
            Span::styled(
                heading,
                Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
            ),
        ]));
        for line in text.lines() {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(line.to_string(), Style::default().fg(Color::DarkGray)),
            ]));
        }
    } else {
        // Collapsed: animated dots, no content derived from text
        let dot_count = ((frame_count / 4) % 3) + 1;
        let dots = ".".repeat(dot_count as usize);
        lines.push(Line::from(vec![
            Span::styled(
                "Thinking: ",
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
            Span::styled(
                dots,
                Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ),
        ]));
    }
    lines
}
```

### startup_routing.rs test (D-07)

```rust
// Source: D-07 from CONTEXT.md; pattern from render_snapshots.rs [VERIFIED: directly read]
use claurst_tui::onboarding_dialog::{OnboardingDialogState, OnboardingPage};

#[test]
fn show_starts_at_welcome_page() {
    let mut dialog = OnboardingDialogState::new();
    dialog.show();
    assert!(dialog.visible, "dialog should be visible after show()");
    assert_eq!(dialog.page, OnboardingPage::Welcome, "show() must start at Welcome, not ProviderSetup");
}
```

---

## Runtime State Inventory

> Not applicable — this is a code fix phase, not a rename/refactor/migration phase. No stored data, live service config, OS-registered state, secrets, or build artifacts are affected.

---

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | cargo test | ✓ | (workspace) | — |
| claurst-tui crate | all tests | ✓ | workspace | — |

No external tools, services, or databases are needed. Both fixes are pure Rust code changes.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (cargo test) |
| Config file | none (Cargo.toml workspace) |
| Quick run command | `cargo test -p claurst-tui thinking_block_collapsed` |
| Full suite command | `cargo test -p claurst-tui` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BUG-A (D-01/D-03) | Collapsed thinking block shows dots, not content | unit | `cargo test -p claurst-tui thinking_block_collapsed` | ✅ (must be updated) |
| BUG-B (D-04) | show() routes to Welcome page, not ProviderSetup | unit | `cargo test -p claurst-tui show_starts_at_welcome_page` | ❌ Wave 0 (new file) |

### Sampling Rate

- **Per task commit:** `cargo test -p claurst-tui thinking_block_collapsed show_starts_at_welcome_page`
- **Per wave merge:** `cargo test -p claurst-tui`
- **Phase gate:** `cargo test -p claurst-tui` must be 0 failures before `/gsd-verify-work`

### Wave 0 Gaps

- [ ] `crates/tui/tests/startup_routing.rs` — covers D-07 (BUG-B routing test); new file to create

*(No framework install needed — existing cargo test infrastructure covers all phase requirements)*

---

## Security Domain

No security-sensitive changes. This phase modifies:
1. A TUI rendering function (no auth, no data persistence, no input handling)
2. A startup routing method call (onboarding dialog page selection, not credential handling)

ASVS categories do not apply to these changes.

---

## Open Questions

None. All code paths are fully read and root-caused. Both fixes are unambiguous.

---

## Assumptions Log

> All claims in this research were verified by directly reading the source files — no assumed claims.

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| — | — | — | — |

**This table is empty:** All claims in this research were verified or cited — no user confirmation needed.

---

## Sources

### Primary (HIGH confidence — directly read source files)

- `crates/tui/src/messages/mod.rs:1245` — `render_thinking_block` current implementation (bug confirmed) [VERIFIED: codebase]
- `crates/tui/src/messages/mod.rs:30` — `RenderContext` struct definition [VERIFIED: codebase]
- `crates/tui/src/transcript_turn.rs:53` — `reasoning_heading` function [VERIFIED: codebase]
- `crates/tui/src/render.rs:1858` — `shimmer_spans` animation reference pattern [VERIFIED: codebase]
- `crates/tui/src/render.rs:1115,1271` — `RenderContext` construction sites [VERIFIED: codebase]
- `crates/tui/src/onboarding_dialog.rs:34,47,53` — `OnboardingDialogState`, `show()`, `show_provider_setup()` [VERIFIED: codebase]
- `crates/cli/src/main.rs:1430-1443` — startup routing block [VERIFIED: codebase]
- `crates/tui/tests/render_snapshots.rs:159-165` — failing test [VERIFIED: codebase + cargo test output]
- `crates/tui/src/lib.rs:107,151` — `onboarding_dialog` module export [VERIFIED: codebase]
- `.planning/phases/01-welcome-screen-fix/01-UAT.md` — root cause analysis [VERIFIED: codebase]

---

## Metadata

**Confidence breakdown:**
- Bug root causes: HIGH — confirmed by reading source and running failing test
- Fix implementation: HIGH — all code paths read; patterns confirmed by existing shimmer_spans reference
- Test strategy: HIGH — existing test infrastructure, public exports confirmed
- Scope boundaries: HIGH — CONTEXT.md decisions are fully locked

**Research date:** 2026-05-05
**Valid until:** 2026-06-05 (stable Rust codebase, no fast-moving dependencies)
