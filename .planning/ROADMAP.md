# Roadmap: claurst

**Milestone:** v1
**Defined:** 2026-05-04
**Granularity:** Standard
**Coverage:** 1/1 requirements mapped

## Phases

- [ ] **Phase 1: Welcome Screen Fix** - Fix silent exit on Enter keypress at first-launch welcome screen
- [ ] **Phase 2: Fix UAT gaps: thinking block collapsed leak and welcome dialog startup routing**
- [ ] **Phase 3: ANTHROPIC_AUTH_TOKEN Bearer Auth Support** - Full ANTHROPIC_AUTH_TOKEN support with bearer/x-api-key switch logic and user-configurable JSON setting

## Phase Details

### Phase 1: Welcome Screen Fix
**Goal**: Users can complete the first-launch welcome screen without claurst exiting silently
**Depends on**: Nothing (first phase)
**Requirements**: BUG-01
**Success Criteria** (what must be TRUE):
  1. User presses Enter on the welcome screen and claurst proceeds to the main TUI session instead of exiting
  2. User who encounters the welcome screen for the first time sees no unexpected termination or blank output
  3. Pressing Enter on the welcome screen produces the same result across platforms (macOS, Linux)
**Plans**: 1 plan

Plans:
- [x] 01-01-PLAN.md — Fix failing onboarding_defaults_hidden test and add D-06 regression tests (Enter/Esc on Welcome/KeyBindings pages; assert should_quit=false)

### Phase 2: Fix UAT gaps: thinking block collapsed leak and welcome dialog startup routing

**Goal:** Fix two UAT gaps from Phase 1: (1) collapsed thinking block leaks content text via reasoning_heading; (2) first-run no-credentials users see ProviderSetup instead of Welcome page
**Requirements**: TBD
**Depends on:** Phase 1
**Plans:** 2 plans

Plans:
- [x] 02-01-PLAN.md — Fix thinking block collapsed content leak: add frame_count to RenderContext, animated dots in collapsed branch, update render.rs construction sites and render_snapshots.rs tests
- [x] 02-02-PLAN.md — Fix startup routing (show_provider_setup → show) and add startup_routing.rs regression test

### Phase 3: ANTHROPIC_AUTH_TOKEN Bearer Auth Support

**Goal:** Fully support ANTHROPIC_AUTH_TOKEN as a credential source that sends Bearer auth instead of x-api-key; add explicit switch logic for auth header mode; expose user-configurable `use_bearer_auth` setting in settings.json provider config
**Requirements**: D-01 through D-09
**Depends on:** Phase 2
**Plans:** 3 plans

Plans:
- [x] 03-01-PLAN.md — TDD Wave 1: Write failing bearer_auth.rs integration tests + add serial_test dev-dep (RED phase)
- [x] 03-02-PLAN.md — Implement ProviderConfig.use_bearer_auth field + conflict-first resolver returning anyhow::Result (GREEN phase)
- [x] 03-03-PLAN.md — Update main.rs call site with ? propagation; verify config.env injection in place (compile gate)

## Progress Table

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Welcome Screen Fix | 0/1 | Not started | - |
| 2. Fix UAT gaps: thinking block and welcome routing | 0/2 | Not started | - |
| 3. ANTHROPIC_AUTH_TOKEN Bearer Auth Support | 0/3 | Not started | - |

---
*Roadmap defined: 2026-05-04*
*Last updated: 2026-05-06 after phase 3 planning*
