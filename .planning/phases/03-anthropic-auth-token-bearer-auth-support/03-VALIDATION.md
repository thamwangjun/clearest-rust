---
phase: 3
slug: anthropic-auth-token-bearer-auth-support
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-06
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) + serial_test 3.2.0 |
| **Config file** | `crates/core/Cargo.toml` (dev-dep) |
| **Quick run command** | `cargo test -p claurst-core --test bearer_auth 2>&1` |
| **Full suite command** | `cargo test --workspace 2>&1` |
| **Estimated runtime** | ~15 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p claurst-core --test bearer_auth 2>&1`
- **After every plan wave:** Run `cargo test --workspace 2>&1`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Wave 0 Notes

Plan 03-01 IS the Wave 0 plan. It creates `crates/core/tests/bearer_auth.rs` with all 5 RED test stubs and adds `serial_test = "3.2.0"` to `crates/core/Cargo.toml`. No separate Wave 0 setup step is needed.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Decision | Threat Ref | Expected Behavior | Test Type | Automated Command | Status |
|---------|------|------|----------|------------|-------------------|-----------|-------------------|--------|
| 3-01-01 | 01 | 1 | D-07/D-08/D-09 | — | RED test file created; `cargo test -p claurst-core --test bearer_auth` exits non-zero (compile or runtime failure) | tdd-red | `cargo check -p claurst-core --tests 2>&1` | ⬜ pending |
| 3-02-01 | 02 | 2 | D-04 | — | ProviderConfig accepts `use_bearer_auth: Option<bool>` | execute | `cargo check -p claurst-core 2>&1` | ⬜ pending |
| 3-02-02 | 02 | 2 | D-01/D-02/D-03 | — | Conflict conditions return Err; bearer path returns Ok(Some((token, true))) | tdd-green | `cargo test -p claurst-core --test bearer_auth 2>&1` | ⬜ pending |
| 3-03-01 | 03 | 3 | D-05/D-06 | — | main.rs call site propagates Err via ?; config.env injection confirmed before auth call | execute | `cargo build --workspace 2>&1` | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Manual-Only Verifications

| Behavior | Decision | Why Manual | Test Instructions |
|----------|----------|------------|-------------------|
| Bearer header sent to proxy (http://epsilon.net.tham.one:53080) | D-01 | Requires live proxy and real btr-... token | Set ANTHROPIC_AUTH_TOKEN in settings.json config.env, run claurst, inspect proxy logs for `Authorization: Bearer` header |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 is Plan 03-01 itself (test file creation)
- [x] No watch-mode flags
- [x] Feedback latency < 30s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
