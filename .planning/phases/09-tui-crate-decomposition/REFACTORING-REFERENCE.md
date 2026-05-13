# Refactoring Reference: Phase 9 — tui Crate Decomposition

Source: Fowler, *Refactoring* — sections adapted for Rust idioms.
Requirements: BLOT-02 (decompose App struct, 150 fields), COUP-03 (fix Feature Envy in render.rs).
Primary files: `crates/tui/src/app.rs` (5,990 lines), `crates/tui/src/render.rs` (2,800 lines).

---

## Smell: Large Class (→ Large Struct in Rust)

**Signs:** A struct has many fields and a large `impl` block mixing unrelated concerns.
`App` has 150 `pub` fields and 123 methods across 4,853 lines of `impl App`, covering:
input handling, streaming updates, TUI rendering coordination, MCP auth, bridge events,
permission dialogs, diff viewer, tool use overlay, session management — at least 6
distinct concerns in one type.

**Why it hurts:** A developer changing the diff viewer must read past streaming state.
A developer adding a new overlay touches the same struct as one fixing auth. Split borrows
are impossible because the compiler sees one monolithic type. Tests must construct the full
`App` even to test a single concern.

**Treatment decision tree:**
- Does the group of fields represent an independent concept with its own invariants?
  → **Extract Struct** (Rust equivalent of Extract Class)
- Is the group only ever read by one subset of methods?
  → Move those fields + methods together into an extracted struct
- Is a large `impl` block really just a graphical interface (render logic) that could be
  separated from the domain model?
  → Move render functions to `render.rs` with focused sub-state parameters (fixes Feature
  Envy simultaneously — see below)

---

## Technique: Extract Module / Extract Struct (Rust equivalent of Extract Class)

**Problem:** `App` does the work of six structs. One struct means one place to change for
six different reasons (Divergent Change).

**Steps:**

1. **Decide the split before writing a line of code.** Group `App`'s 150 fields by which
   concern they belong to. Candidate groupings for `App`:

   | Sub-struct | Fields (examples) |
   |---|---|
   | `InputState` | `input`, `cursor_pos`, `input_mode`, `history`, `completions` |
   | `SessionState` | `messages`, `session_id`, `cost`, `model`, `tool_use_blocks` |
   | `RenderState` | `scroll_offset`, `frame_count`, `theme`, `overlay_stack` |
   | `RuntimeHandles` | `stream_tx`, `query_handle`, `bridge_handle`, `mcp_manager` |
   | `PermissionState` | `pending_permission`, `bypass_permissions`, `trust_level` |

2. **Create the new structs** in separate files (`input_state.rs`, `session_state.rs`, etc.)
   with `pub(crate)` visibility for fields accessed by sibling modules.

3. **Replace the flat fields in `App`** with a single field per sub-struct:
   ```rust
   pub struct App {
       pub input: InputState,
       pub session: SessionState,
       pub render: RenderState,
       pub handles: RuntimeHandles,
       pub permissions: PermissionState,
   }
   ```

4. **Move methods one at a time** using Move Method (below). Start with private methods —
   they have the smallest blast radius. Test after each move.

5. **Fix borrow conflicts** that arise when a method borrows two sub-structs simultaneously.
   The borrow checker allows `&mut self.input` and `&self.session` simultaneously when they
   are separate fields. This is the core benefit of this refactoring in Rust — it *enables*
   split borrows that were previously impossible.

6. **Rename `App` if warranted** once responsibilities are clearer. If `App` becomes a thin
   coordinator, it may deserve a name like `AppState` or `TuiContext`.

**Rust-specific caution:** If `App` is constructed in tests as `App { field: value, .. }`,
adding a new sub-struct field is a breaking change to any struct literal. Use
`App::new_for_test()` (created in Phase 4) as the single construction point to insulate tests.

---

## Technique: Move Method (supporting)

**Problem:** A method in `impl App` primarily operates on fields that belong to one of the
extracted sub-structs, not on `App` as a whole.

**Rust translation:** Move the method to `impl InputState` / `impl SessionState` / etc.
Update all call sites from `app.method()` to `app.input.method()`.

**Steps:**

1. Identify which sub-struct the method primarily reads/writes.
2. Check whether the method also reads fields from *other* sub-structs. If yes:
   - If it reads one other sub-struct: pass it as a parameter to the moved method.
   - If it reads three or more sub-structs: it may belong on `App` as a coordinator method —
     keep it there but have it delegate to smaller sub-methods.
3. Copy the method to the target sub-struct's `impl` block.
4. In the original location, replace the method body with a delegation call:
   `self.input.method()` — keep it temporarily to avoid a cascade of compile errors.
5. Once all call sites are updated, delete the delegating wrapper.

```rust
// BEFORE: method on App that only touches input fields
impl App {
    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }
}

// AFTER: method on InputState where the data lives
impl InputState {
    pub fn move_cursor_right(&mut self) {
        if self.cursor_pos < self.input.len() {
            self.cursor_pos += 1;
        }
    }
}
// App delegates until all callers are updated:
impl App {
    pub fn move_cursor_right(&mut self) { self.input.move_cursor_right() }
}
```

---

## Technique: Move Field (supporting)

**Rule of thumb:** Put a field in the same struct as the methods that use it — or where most
of those methods are.

**Steps:**

1. Make the field `pub(crate)` if it isn't already (to avoid breaking external users during
   the move).
2. Declare the same field in the target sub-struct.
3. Replace all references to `self.field` with `self.sub_struct.field` (or wherever it moved).
4. Delete the original field from `App`.
5. If the field was `pub`, check whether external crates access it — `grep` for the field
   name across the workspace before deleting.

---

## Smell: Feature Envy (render.rs)

**Signs:** A function accesses data from another type more than from its own context.
In `render.rs`, every function takes `app: &App` (or `frame: &mut Frame, app: &App`) —
they exist in a `render` module but their primary data source is `App`. The functions
"envy" `App`'s data.

**Why it hurts:** `render_messages` takes `app: &App` to access `app.messages` — but it
also reads `app.scroll_offset`, `app.frame_count`, `app.theme`. As `App` decomposes into
sub-structs (see above), render functions must be updated to match. If they still take
whole-`App`, the decomposition provides no decoupling benefit.

**Treatment:** After `App` is decomposed, refactor render functions to accept focused
sub-state arguments instead of the whole `App`.

```rust
// BEFORE: render function envies App's internals
fn render_messages(frame: &mut Frame, app: &App, area: Rect) {
    for msg in &app.messages {          // app.session.messages after decomposition
        let color = app.theme.text;     // app.render.theme.text after decomposition
        // ...
    }
}

// AFTER: render function takes only what it needs
fn render_messages(
    frame: &mut Frame,
    messages: &[Message],    // from SessionState
    render: &RenderState,    // scroll, theme, frame_count
    area: Rect,
) {
    for msg in messages {
        let color = render.theme.text;
        // ...
    }
}

// Call site in the top-level render_app:
render_messages(frame, &app.session.messages, &app.render, area);
```

**When to ignore Feature Envy in render.rs:** If a render function genuinely needs 4+
sub-structs to do its job (e.g., `render_app` which orchestrates the entire frame), keep
it accepting the whole `App`. Only refactor functions that use a clearly bounded subset.

**Rust-specific:** In Rust, the "move the method to the class it envies" option often
means moving to a different *module*, not a different *trait impl*. Moving a render helper
into `session_state.rs` as a method would be odd — prefer passing focused parameters.
Extension traits (for types defined in other crates) are the Rust idiom for cases where
you truly want the method to live on the foreign type.

---

## Sequencing note for Phase 9

This is the highest-risk structural phase. Recommended order within the phase:

1. **Decompose supporting modules first**: `prompt_input.rs`, `overlays.rs`, `dialogs.rs`
   are smaller than `app.rs` — practice the Extract Struct + Move Method pattern on them
   before tackling the 150-field `App`.
2. **Map `App`'s 150 fields into groups** (step 1 of Extract Struct above) and get that
   grouping committed as a comment block or doc comment before moving any code.
3. **Extract sub-structs one at a time**, starting with the group with the fewest
   cross-dependencies (likely `InputState` — input fields are mostly self-contained).
4. **Fix `render.rs` Feature Envy last** — depends on the sub-structs being stable so the
   function signatures can be updated once rather than incrementally.
5. **Run `cargo insta test --check`** after each sub-struct extraction to catch visual
   regressions early.
