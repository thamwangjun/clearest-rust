# Phase 1: Welcome Screen Fix - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-05-04
**Phase:** 1-welcome-screen-fix
**Areas discussed:** First page for no-credentials first-run, Enter key behavior per page, Status message after dismiss, Regression testing

---

## What to Show for No-Credentials First-Run

| Option | Description | Selected |
|--------|-------------|----------|
| ProviderSetup page first | show_provider_setup() — directly actionable, tells user to pick a provider | |
| Welcome page first | Current behavior via show() — orientation before credentials | ✓ |
| ProviderSetup first, Welcome on next launch | Two distinct first-run flows | |

**User's choice:** Welcome page first
**Notes:** Keep the existing `show()` call. No change to which page is shown.

---

## Enter Key Behavior Per Page (ProviderSetup)

| Option | Description | Selected |
|--------|-------------|----------|
| Dismiss and mark done (current logic) | next_page() advances ProviderSetup → Done | |
| Do nothing / require Esc | ProviderSetup is informational-only | |
| You decide | Leave to implementer | |

**User's choice:** Free-text — "Provider Setup page was never reached because it exits once enter is pressed at welcome page."
**Notes:** Confirmed the bug: pressing Enter on the Welcome page causes a full app exit. The ProviderSetup page is unreachable because claurst terminates first. Root cause investigation deferred to research phase.

---

## Status Message After Dismiss With No Credentials

| Option | Description | Selected |
|--------|-------------|----------|
| Show status message | Set app.status_message = Some("No provider configured — run /connect") | |
| No message needed | Dialog already communicates what to do | ✓ |
| You decide | Leave to implementer | |

**User's choice:** No message needed

---

## Regression Test

| Option | Description | Selected |
|--------|-------------|----------|
| Unit test for key handler | handle_key_event(Enter) while dialog visible, assert should_quit=false | ✓ |
| State machine test only | Existing next_page() tests sufficient | |
| No test | Bug fix only | |

**User's choice:** Yes — unit test for the key handler

---

## Claude's Discretion

- Exact mechanism of the silent exit (exit path tracing is delegated to the researcher/planner)
- Whether the fix goes in `handle_key_event`, in the `'main` loop, or both

## Deferred Ideas

- Showing ProviderSetup as first page for no-credentials users (user declined)
- Status hint message after dialog dismiss with no credentials (user declined)
