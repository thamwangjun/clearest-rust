# Requirements: claurst

**Defined:** 2026-05-04
**Core Value:** A reliable, feature-complete Rust alternative to Claude Code that stays current with upstream changes and fixes bugs fast enough that contributors trust it for daily use.

## v1 Requirements

### Bug Fixes

- [ ] **BUG-01**: User can complete the first-launch welcome screen by pressing Enter without claurst exiting silently (crash or silent failure on Enter keypress is fixed)

## v2 Requirements

*(Deferred — will be added as new milestones by the owner when new Claude Code features or bug reports surface)*

### Known Deferred Bugs

- **BUGS-01**: Mouse capture does not break native text selection and copy-paste (issue #104) — opt-in via settings
- **BUGS-02**: Remote Ollama server is respected; custom OpenAI API URL routing works (issues #86, #106)
- **BUGS-03**: API key can be pasted into TUI input (issue #76)
- **BUGS-04**: Keyboard shortcuts work on non-English keyboard layouts (issue #47)
- **BUGS-05**: Voice / ALSA connects correctly; voice mode toggle works (issue #88)

### Known Deferred Features

- **FEAT-01**: Managed Agents — manager-executor architecture per plan.md
- **FEAT-02**: MCP security hardening — project-level MCP trust gate (issue #123)
- **FEAT-03**: New Claude Code features — tracked by owner, added as future milestones

## Out of Scope

| Feature | Reason |
|---------|--------|
| TypeScript / Node.js implementation | Rust-only project |
| GUI (non-terminal) interface | ratatui TUI is the target |
| Sherpa-ONNX local ASR (issue #114) | High integration complexity, small audience |
| Kairos mode (issue #103) | Unclear spec, needs separate design phase |
| Feature parity gap-close sprint | Parity is largely achieved; future Claude Code features arrive as new milestones |
| Bulk bug fix sprint | Bugs addressed individually as milestones when prioritized |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| BUG-01 | Phase 1 | Pending |

**Coverage:**
- v1 requirements: 1 total
- Mapped to phases: 1
- Unmapped: 0 ✓

---
*Requirements defined: 2026-05-04*
*Last updated: 2026-05-04 after initial definition*
