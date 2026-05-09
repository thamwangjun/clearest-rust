---
phase: 3
slug: anthropic-auth-token-bearer-auth-support
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-05-06
audited: 2026-05-08
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
| 3-01-01 | 01 | 1 | D-07/D-08/D-09 | — | RED test file created; `cargo test -p claurst-core --test bearer_auth` exits non-zero (compile or runtime failure) | tdd-red | `cargo check -p claurst-core --tests 2>&1` | ✅ green |
| 3-02-01 | 02 | 2 | D-04 | — | ProviderConfig accepts `use_bearer_auth: Option<bool>` | execute | `cargo check -p claurst-core 2>&1` | ✅ green |
| 3-02-02 | 02 | 2 | D-01/D-02/D-03 | — | Conflict conditions return Err; bearer path returns Ok(Some((token, true))) | tdd-green | `cargo test -p claurst-core --test bearer_auth 2>&1` | ✅ green |
| 3-03-01 | 03 | 3 | D-05/D-06 | — | main.rs call site propagates Err via ?; config.env injection confirmed before auth call | execute | `cargo build --workspace 2>&1` | ✅ green |

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

**Approval:** complete — all 8 tests green (2026-05-08)

---

## Validation Audit 2026-05-08

| Metric | Count |
|--------|-------|
| Tasks audited | 4 |
| Gaps found | 1 |
| Resolved | 1 |
| Escalated | 0 |

**Gap found:** `config_env_injection_does_not_overwrite_existing_env` (WR-04 test added by code review fix in commit 5598b2d) had a missing `HashMap<String, String>` type annotation on line 166, causing a compile error. Fixed inline (type annotation added). All 8 tests now compile and pass green.

**Note on test count:** Plan originally specified 6 D-09 tests. Two additional regression tests were added during code review fixes: WR-03 (`pin_bearer_with_no_token_returns_none`) and WR-04 (`config_env_injection_does_not_overwrite_existing_env`). Total: 8 tests, all green.

**Pre-existing workspace failure:** `codex_adapter::tests::test_anthropic_to_openai_request_basic` fails in `claurst-api` due to float precision (`0.699999988079071 != 0.7`). Confirmed present in the initial commit (584ae9a) — out of scope for Phase 3.
