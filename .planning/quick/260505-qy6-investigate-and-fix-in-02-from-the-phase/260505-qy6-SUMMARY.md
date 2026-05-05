---
phase: quick
plan: 260505-qy6
subsystem: planning-docs
tags: [investigation, mcp, code-review, not-applicable]
dependency_graph:
  requires: []
  provides: [IN-02 closed]
  affects: [02-REVIEW.md]
tech_stack:
  added: []
  patterns: []
key_files:
  created: []
  modified:
    - .planning/phases/02-fix-uat-gaps-thinking-block-collapsed-leak-and-welcome-dialo/02-REVIEW.md
decisions:
  - "McpManager::call_tool expects the full prefixed name and strips the prefix internally — passing the prefixed name at line 95 is correct; IN-02 is not applicable"
metrics:
  duration: "~5 minutes"
  completed: "2026-05-05"
---

# Phase quick Plan 260505-qy6: Investigate and close IN-02 Summary

**One-liner:** Traced MCP call_tool contract through 3-hop chain; confirmed prefix-stripping is internal to McpManager — IN-02 closed as not applicable with no code change.

## What Was Investigated

IN-02 from the phase 02 code review raised the question of whether `McpToolWrapper::execute` should pass the bare tool name or the full prefixed name to `McpManager::call_tool`. The concern was that MCP server implementations might expect the bare name and tool-not-found errors would result.

## Call Chain Traced (3 hops)

1. **`crates/cli/src/main.rs:95`** — `McpToolWrapper::execute` calls:
   ```rust
   self.manager.call_tool(&self.tool_def.name, args).await
   ```
   `self.tool_def.name` is the **full prefixed name** (e.g. `filesystem_read_file`).

2. **`crates/mcp/src/lib.rs:1034-1051`** — `McpManager::call_tool` receives the prefixed name and strips the server prefix itself:
   ```rust
   if let Some(tool_name) = prefixed_name.strip_prefix(&prefix) {
       return client.call_tool(tool_name, arguments).await;
   }
   ```
   The manager routes to the correct client and forwards the **bare name**.

3. **`crates/mcp/src/lib.rs:677-690`** — `McpClient::call_tool` receives the bare name and sends it directly to the MCP backend.

## Conclusion: Not Applicable

`McpManager::call_tool` expects the **full prefixed name** and performs the prefix stripping internally. Passing `&self.tool_def.name` (the prefixed name) at line 95 is correct. The `bare_name` local variable is correctly scoped to the `Err` branch error message at line 104.

No code change was required.

## Deviations from Plan

None — plan executed exactly as written.

## Commits

| Task | Description | Commit | Files |
|------|-------------|--------|-------|
| 1 | Annotate IN-02 in 02-REVIEW.md as not applicable | 6000b4f | 02-REVIEW.md |

## Self-Check: PASSED

- 02-REVIEW.md contains "not applicable" in the IN-02 section: confirmed
- `git diff --name-only` shows only 02-REVIEW.md was modified: confirmed
- No Rust source files were changed: confirmed
