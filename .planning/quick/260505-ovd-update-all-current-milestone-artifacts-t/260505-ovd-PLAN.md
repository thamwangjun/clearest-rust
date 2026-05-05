---
quick_id: 260505-ovd
slug: update-all-current-milestone-artifacts-t
type: quick
date: 2026-05-05
---

## Task

Update all current milestone planning artifacts to reflect the major refactor.

The refactor moved the workspace from `claurst/src-rust/` to the `clearest-rust/` repo root.
Changes needed across phase artifacts and research docs:

1. Strip `src-rust/` prefix from all crate path references (`src-rust/crates/` → `crates/`)
2. Fix absolute paths (`/local/claurst/src-rust` → `/local/clearest-rust`)
3. Fix `@` absolute references in PLAN.md to use relative paths
4. Fix `cargo build -p claurst-cli` → `cargo build -p claurst` (CLI crate is named `claurst`)
5. Update ARCHITECTURE.md narrative about repo structure

## Files Changed

### Phase artifacts (must be accurate for executor)
- `phases/01-welcome-screen-fix/01-01-PLAN.md`
- `phases/01-welcome-screen-fix/01-01-SUMMARY.md`
- `phases/01-welcome-screen-fix/01-CONTEXT.md`
- `phases/01-welcome-screen-fix/01-REVIEW.md`
- `phases/01-welcome-screen-fix/01-REVIEW-FIX.md`
- `phases/02-.../02-CONTEXT.md`
- `phases/02-.../02-VALIDATION.md`

### Research/background docs
- `research/ARCHITECTURE.md`
- `research/PITFALLS.md`
- `research/STACK.md`
- `research/SUMMARY.md`
