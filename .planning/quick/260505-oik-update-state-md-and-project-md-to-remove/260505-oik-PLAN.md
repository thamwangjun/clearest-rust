---
phase: quick
plan: 260505-oik
type: execute
wave: 1
depends_on: []
files_modified:
  - .planning/STATE.md
  - .planning/PROJECT.md
autonomous: true
requirements: []

must_haves:
  truths:
    - "STATE.md contains no references to `src-rust/`"
    - "PROJECT.md contains no references to `src-rust/`"
    - "Both files accurately reflect Cargo.toml at the repo root as the workspace root"
  artifacts:
    - path: ".planning/STATE.md"
      provides: "Updated project state with correct workspace root path"
      contains: "Relevant Files section without src-rust/ prefix"
    - path: ".planning/PROJECT.md"
      provides: "Updated project context with correct workspace root path"
      contains: "Context section without src-rust/ reference"
  key_links:
    - from: ".planning/STATE.md"
      to: "repo root Cargo.toml"
      via: "Relevant Files section"
      pattern: "workspace root"
---

<objective>
Remove all references to `src-rust/` from .planning/STATE.md and .planning/PROJECT.md, updating them to reflect that the Rust workspace now lives at the repo root (Cargo.toml at `/`).

Purpose: The repo was refactored so Rust source is at the repo root, not under `src-rust/`. Planning docs must match reality so executors and future planners don't get confused about project structure.
Output: Two updated planning files with no stale `src-rust/` path references.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/STATE.md
</context>

<tasks>

<task type="auto">
  <name>Task 1: Remove src-rust/ references from STATE.md</name>
  <files>.planning/STATE.md</files>
  <action>
    In the `### Relevant Files` section, change:
      `- \`src-rust/\` — Rust workspace root (12 crates)`
    to:
      `- Rust workspace root: repo root (Cargo.toml at `/`) — 12 crates`

    No other changes needed to STATE.md. Do not touch quick task history, metrics, session continuity, or any other section.
  </action>
  <verify>
    <automated>grep -n "src-rust" /Users/thamw/development/local/clearest-rust/.planning/STATE.md | wc -l</automated>
  </verify>
  <done>Output of grep command is 0 — no `src-rust` occurrences remain in STATE.md.</done>
</task>

<task type="auto">
  <name>Task 2: Remove src-rust/ references from PROJECT.md</name>
  <files>.planning/PROJECT.md</files>
  <action>
    In the `## Context` section, find the sentence:
      "The parent repo (`claurst/`) holds the git history and `.planning/`; Rust source lives under `src-rust/`."
    Replace it with:
      "The parent repo (`claurst/`) holds the git history and `.planning/`; Rust source lives at the repo root (Cargo.toml at `/`)."

    No other changes needed to PROJECT.md. Do not touch Requirements, Key Decisions, Constraints, Evolution, or any other section.
  </action>
  <verify>
    <automated>grep -n "src-rust" /Users/thamw/development/local/clearest-rust/.planning/PROJECT.md | wc -l</automated>
  </verify>
  <done>Output of grep command is 0 — no `src-rust` occurrences remain in PROJECT.md.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| planning docs | Internal documentation only; no trust boundary crossed |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-oik-01 | Tampering | STATE.md / PROJECT.md | accept | Doc-only edit; no code execution or secrets involved |
</threat_model>

<verification>
After both tasks complete:
- `grep -rn "src-rust" .planning/STATE.md .planning/PROJECT.md` returns no output
- Both files otherwise unchanged (quick task history, metrics, requirements, decisions, etc. untouched)
</verification>

<success_criteria>
- STATE.md `### Relevant Files` section describes the workspace root as the repo root, not `src-rust/`
- PROJECT.md `## Context` section describes Rust source at the repo root, not `src-rust/`
- Zero remaining `src-rust` occurrences in either file
- No other content modified in either file
</success_criteria>

<output>
After completion, create `.planning/quick/260505-oik-update-state-md-and-project-md-to-remove/260505-oik-SUMMARY.md`
</output>
