---
phase: 2
slug: fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo
status: verified
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-05
audited: 2026-05-05
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml workspace |
| **Quick run command** | `cargo test -p claurst-tui` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p claurst-tui`
- **After every plan wave:** Run `cargo test`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 2-01-01 | 01 | 1 | D-01, D-02, D-03 | T-02-01 | collapsed branch calls no reasoning_heading | unit | `cargo test -p claurst-tui thinking_block_collapsed thinking_block_expanded` | ✅ (existing + updated) | ✅ green |
| 2-02-01 | 02 | 1 | D-07 | — | N/A | unit | `cargo test -p claurst-tui show_starts_at_welcome_page` | ✅ (created Wave 0) | ✅ green |
| 2-02-02 | 02 | 1 | D-04 | T-02-04 | credential check unmodified | build + unit | `cargo test -p claurst-tui && cargo build -p claurst` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `crates/tui/tests/startup_routing.rs` — created by Plan 02 Task 1 (Wave 0 step). Test passes GREEN immediately because `show()` is already correctly implemented in onboarding_dialog.rs. Purpose is regression guard.

*Existing infrastructure covers the thinking_block tests — only startup_routing.rs needs creation.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Animated dots cycle visually in collapsed thinking block | D-01 | Visual animation cannot be asserted by snapshot test alone | Run app, trigger a thinking block, collapse it, observe `...` animates |

---

## Nyquist Compliance Notes

- Plan 01 Task 1 now includes the `render_snapshots.rs` call-site updates alongside the signature change. The codebase compiles and both thinking_block tests run at the end of Task 1 — no broken-compilation window between tasks.
- Plan 02 Task 1 creates `startup_routing.rs` as a Wave 0 step before Task 2 touches main.rs. The test passes GREEN immediately (regression guard, not TDD red-green).
- Every task has an `<automated>` verify command that exercises the behavior under test.

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** execution verified 2026-05-05

---

## Validation Audit 2026-05-05

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
| Tasks verified green | 3 |

All tasks confirmed green post-execution. No new tests required — Wave 0 test (`startup_routing.rs`) was created during plan execution as designed. Full suite: 27 tests passed, 0 failed.

---

## Validation Audit 2026-05-05 (re-audit)

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
| Tasks verified green | 3 |

Re-audit confirmed all tests still green: 26 render_snapshots + 1 startup_routing. All three targeted tests pass (`thinking_block_collapsed`, `thinking_block_expanded`, `show_starts_at_welcome_page`). No drift from original audit.
