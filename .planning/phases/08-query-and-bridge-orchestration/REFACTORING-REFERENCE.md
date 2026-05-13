# Refactoring Reference: Phase 8 — query and bridge Orchestration

Source: Fowler, *Refactoring* — sections adapted for Rust idioms.
Requirement: COUP-04 — flatten accessor chains in `query/src/lib.rs` and `tui/src/messages/mod.rs`;
decompose `run_query_loop` (complexity 156).

---

## Smell: Message Chains

**Signs:** A series of calls like `a.b().c().d()` — the caller navigates through the object graph
step by step to reach the thing it actually needs.

**Why it hurts:** Every intermediate hop is a dependency. If the relationship between `b` and `c`
changes, the caller must change too. In Rust, long chains often appear as:

```rust
// SMELL: caller knows the entire path to StreamEvent internals
let text = ctx.query_state.current_turn.content_blocks.last().unwrap().text.clone();

// SMELL: message handling navigating through coordinator → session → turn
app.coordinator.session.current_turn().tool_uses().first().map(|t| t.id.clone())
```

**When to ignore:** A chain of two hops (`a.b().method()`) is usually fine. The smell triggers
at three or more hops where the intermediate objects are not the caller's concern.

---

## Technique: Hide Delegate (primary fix)

**Problem:** The caller reaches through object A to call something on object B.

**Rust translation:** Add an intent-revealing method on the intermediate type that encapsulates
the navigation. The caller calls one method; the navigation lives inside.

**Steps:**

1. Identify what the caller actually *wants* (the intent), not what it's doing (the navigation).
2. Add a method on the closest accessible type that returns or computes that intent directly.
3. Replace all call-chain sites with the new method.
4. If the intermediate object is now only accessed via the new method, consider removing its
   direct accessor.

```rust
// BEFORE: caller navigates the internals
let tool_id = app.coordinator.session.current_turn().tool_uses().first().map(|t| t.id.clone());

// AFTER: intent-revealing method on the type that owns the concept
impl QueryCoordinator {
    pub fn active_tool_id(&self) -> Option<&str> {
        self.session.current_turn()?.tool_uses().first().map(|t| t.id.as_str())
    }
}
let tool_id = app.coordinator.active_tool_id();
```

**Rust-specific note:** Rust's ownership rules mean you often cannot return a mutable reference
through a chain. If extraction requires returning `&mut T` through multiple indirections, prefer
returning owned values or restructuring with `pub(crate)` fields instead of getter chains.

**Risk of overdoing it:** If you add a delegating method for every possible access pattern,
the intermediate type becomes a Middle Man (see below) — a pass-through struct with no real
logic of its own. Prefer adding methods only where the intent is meaningfully different from
"give me the raw field."

---

## Technique: Remove Middle Man (counterbalance)

**Problem:** A struct has accumulated so many delegating methods that it does nothing itself —
every method just calls through to another struct.

**Rust translation:** Delete the delegating methods and let callers access the inner type directly
(make the field `pub(crate)` or `pub`). This is the *opposite* of Hide Delegate — use it when
the "server" struct has been over-delegated and adds no value.

**Steps:**

1. Add a public (or `pub(crate)`) accessor for the delegate field on the server struct.
2. For each delegating method, replace all call sites with a direct call on the delegate.
3. Remove the delegating method from the server struct.

**When to apply in this phase:** If `QueryCoordinator` or `BridgeState` ends up with 10+
methods that all just forward to an inner `Session` or `EventStream`, consider whether the
indirection earns its keep. If not, expose the inner type and delete the wrappers.

---

## Technique: Extract Function (supporting)

**Problem:** A code fragment inside `run_query_loop` does one identifiable thing but is
inline in a 2,400-line function body.

**Rust translation:** Same as the book — pull the fragment into a named `fn`. The challenge
in Rust is that the borrow checker may not allow extracting a fragment that borrows multiple
fields of the same struct simultaneously (split borrow problem).

**Steps:**

1. Name the extracted function after *what it does*, not *how*: `handle_tool_response`,
   `finalize_turn`, `emit_stream_event` — not `process_stuff` or `do_step3`.
2. Identify all variables used in the fragment. Variables declared inside and not used outside
   become local to the new function. Variables declared outside that are read become parameters.
3. If a local variable is mutated and used afterward in the original function, return it.
4. **Rust split-borrow workaround:** If extraction fails because two fields of the same struct
   are borrowed at once, either:
   - Pass fields individually as parameters (not the whole struct)
   - Use `pub(crate)` fields and borrow them at the call site before passing in
   - Decompose the struct first (see Phase 9 for App — same principle applies to
     `QueryState` if it has the same problem)
5. If a `let` temp variable is only used to pass into the extracted function, consider whether
   it even needs a name — you can often inline it into the argument.

```rust
// BEFORE: 50-line block inline in run_query_loop
let event = stream.next().await?;
match event {
    StreamEvent::ContentDelta { text, .. } => {
        current_turn.push_text(&text);
        if let Some(cb) = on_token.as_mut() {
            cb(&text).await?;
        }
        // ... 40 more lines
    }
    // ...
}

// AFTER: named step function
async fn handle_content_delta(
    turn: &mut CurrentTurn,
    text: &str,
    on_token: &mut Option<TokenCallback>,
) -> Result<()> {
    turn.push_text(text);
    if let Some(cb) = on_token.as_mut() {
        cb(text).await?;
    }
    // ...
    Ok(())
}
```

---

## Sequencing note for `run_query_loop`

The research team flagged this function (complexity 156, ~2,400 lines) as the highest-risk
extraction in this phase. Recommended order:

1. **Write characterization tests first** (Phase 4 gate — should already be done).
2. **Map all field accesses** in the function body before extracting anything. List which
   fields of which structs are read vs mutated in each logical block.
3. **Extract leaf helpers first** — pure functions with no struct mutation, moving outward.
4. **Extract async steps last** — each `await` point is a potential ownership transfer.
   Keep each extracted async fn to a single logical step.
5. **Run `cargo test` after each extraction**, not after all of them.
