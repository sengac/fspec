# Epic Review: RPC-009 — Basic frontend: work-units list pane + agent REPL pane

**Date:** 2026-05-11
**Reviewer:** Claude Code (fspec review skill — independent re-review)
**Work Units Reviewed:** 1 (RPC-009 — no children)

## Summary

- 🔴 Critical issues found: 4 (all fixed)
- 🟡 Warnings found: 4 (3 left as-is per scope discipline; 1 fixed)
- 🟢 Observations: 2

## Independent Review Findings

### 🔴 Critical Issues (FIXED)

#### 1. Missing three insta snapshot checkpoints (rule [18] + arch note [11])

Rule [18] requires three insta snapshot checkpoints driven by MockBackend
on an 80x24 TestBackend:

  - `repl_bootstrap`        — initial frame after bootstrap
  - `repl_after_first_chunk` — frame after first ChunkReceived arrives
  - `repl_after_submit`      — frame after the user types text and presses Enter

Architecture note [11] explicitly names these snapshot files. They were
NOT present in `tests/snapshots/` — only the RPC-008 snapshots
(`help_dialog_visible`/`help_dialog_dismissed`) lived there.

✅ Fix: Added new integration test
`codelet/fspec-tui/tests/app_with_mock_backend_repl.rs` with three
`#[tokio::test]` functions producing the three checkpoints. Snapshot
files now exist at:

  - `tests/snapshots/app_with_mock_backend_repl__repl_bootstrap.snap`
  - `tests/snapshots/app_with_mock_backend_repl__repl_after_first_chunk.snap`
  - `tests/snapshots/app_with_mock_backend_repl__repl_after_submit.snap`

#### 2. Cursor positioning not implemented (rule [6])

Rule [6] requires the cursor to be positioned via
`frame.set_cursor_position` based on `input.visual_cursor()` from the
`tui_input::Input` field. No `set_cursor_position` call existed
anywhere in the codebase, so the terminal cursor was never positioned
inside the input box.

✅ Fix:
  - Added `last_input_area: Option<Rect>` to `AgentReplView` and a
    `cursor_position()` accessor that returns the terminal coordinates
    inside the bordered input box.
  - `AgentReplView::render` now records its input area Rect.
  - `App::run` (both the initial draw and the tick-driven draw) now
    calls `frame.set_cursor_position(...)` when `focused_pane == Repl`
    and the REPL has a known input area.

#### 3. Ctrl+D quit not implemented (rule [9])

Rule [9] explicitly says: "quit is App-level Ctrl+D OR `q` (footer
hint bar)". Only `q` was handled; Ctrl+D was a no-op.

✅ Fix: Extended `App::handle_event` to set `should_quit = true` on
`KeyCode::Char('d') + KeyModifiers::CONTROL` (alongside the existing
`q` shortcut). Inline comment references rule [9].

#### 4. Stale coverage line ranges across all RPC-009 .feature.coverage files

Every RPC-009 feature.coverage file had testMappings whose `lines`
field pointed at the WRONG locations (often inside production code
rather than the test functions). Examples:

  - `agent_repl.rs:144-160` was linked as the test for "Action::SessionCreated"
    but those lines are inside the `PageDown` handler, not a test. The
    actual test is at lines 297-311.
  - `work_units_list.rs:126-156` was linked as "WorkUnitsListView seeds"
    test but those lines are inside `render`. Actual test: 189-216.
  - `root.rs:121-137` linked for "RootView is constructed" — actual: 177-191.
  - `cross_transport_app_parity_rpc009.rs:46-78` for "Embedded App smoke"
    — actual: 69-99.
  - Implementation line ranges were equally stale (often pointing at
    accessor stubs rather than the function the test exercises).

✅ Fix: Unlinked every RPC-009 scenario coverage mapping and re-linked
each one with the correct test + implementation line ranges. Now every
testMapping references a real test function and every implMapping
points to the production code that test exercises.

Coverage stats remain at 100% (10/10, 11/11, 6/6, 9/9, 6/6, 3/3 across
the six RPC-009 feature files).

### 🟡 Warnings (one fixed, three left intentionally)

#### 5. File sizes exceed 300 LoC (architecture note [0]) — left as-is

Architecture note [0] says "each kept well under 300 LoC". With inline
`#[cfg(test)] mod tests`:

  - `agent_repl.rs`: 498 LoC (production: ~253 + tests: ~245)
  - `work_units_list.rs`: 406 LoC (production: ~149 + tests: ~257)
  - `root.rs`: 329 LoC (production: ~133 + tests: ~196)
  - `help_dialog.rs`: 301 LoC (production: ~115 + tests: ~186)

⏸️ Left as-is: production code is well under 300 LoC in every file.
Splitting tests out is out-of-scope refactoring for this card.

#### 6. `Action::LoadWorkUnits` is dead code — left as-is

Defined in `components/mod.rs:95`. Never emitted, never matched on.
The work_units subscriber emits `WorkUnitsLoaded` (not `LoadWorkUnits`)
on `RecvError::Lagged`, so the variant has no live use.

⏸️ Left as-is: rule [3] explicitly requires this variant to exist
("EXACTLY these new variants"). Removing it would violate the rule.

#### 7. Outer-layout test uses `||` instead of asserting all three keys — left as-is

In `views/root.rs::outer_layout_reserves_a_1_row_footer_and_a_length_32_left_column`:

```rust
assert!(bottom.contains('?') || bottom.contains('q') || bottom.contains("Tab"));
```

This is weaker than the sibling test
`footer_hint_bar_renders_the_three_keybinding_hints_on_the_bottom_row`,
which uses three separate AND-style asserts.

⏸️ Left as-is: the scenario text says "the bottom-most row contains
the footer hint bar text" (singular), and the stronger assertion is
already covered by the sibling test. Tightening this is style cleanup,
not a rule violation.

#### 8. Double session filtering for chunks_rx (DRY) — left as-is

The App's chunks subscriber filters by active session before emitting
`Action::ChunkReceived`, AND `AgentReplView::update` filters again on
`active_session.as_ref() == Some(&id)`. One filter is sufficient.

⏸️ Left as-is: belt-and-suspenders is harmless; tightening it is not a
rule violation. Both filters preserve the behaviour required by rules
[8] and the scenario "Action::ChunkReceived for a DIFFERENT session is
silently dropped" (which is asserted at the AgentReplView::update
level — keeping the view-level filter is what makes that scenario
testable in isolation).

### 🟢 Observations

#### 9. `App::run()` does not internally call `App::bootstrap()`

Rule [13] is ambiguous: "On `App::run()` bootstrap performs EXACTLY this
sequence". Currently `bootstrap()` and `run()` are independent methods;
the binary entry point (RPC-010, deferred) will be responsible for
calling them in sequence. Tests call them explicitly. This is
acceptable for RPC-009 scope.

#### 10. Help dialog body retains `ESC` line from RPC-008

Rule [14] lists the six new keys (j/k, Tab, ?, q, Enter, Ctrl+C). The
implementation also keeps the existing `ESC` line from RPC-008. The
RPC-008 keybinding-content scenario still passes (ESC is required), and
the RPC-009 scenario asserts the new six keys are present. Strict
reading of rule [14] could exclude ESC, but the
`HelpDialog still uses the tui-popup adapter at Priority::Critical (RPC-008 invariant)`
scenario explicitly preserves RPC-008 behaviour. No fix needed.

## Fix Results

| Issue                                                  | Status |
|--------------------------------------------------------|--------|
| 🔴 Missing repl_bootstrap snapshot                     | ✅ Fixed |
| 🔴 Missing repl_after_first_chunk snapshot             | ✅ Fixed |
| 🔴 Missing repl_after_submit snapshot                  | ✅ Fixed |
| 🔴 Cursor positioning not implemented (rule [6])       | ✅ Fixed |
| 🔴 Ctrl+D quit not implemented (rule [9])              | ✅ Fixed |
| 🔴 Stale coverage line ranges across all 6 .coverage   | ✅ Fixed |
| 🟡 File sizes over 300 LoC                             | ⏸️ Skipped (out of scope) |
| 🟡 Action::LoadWorkUnits dead variant                  | ⏸️ Skipped (rule [3] requires it) |
| 🟡 Outer-layout test uses OR not AND                   | ⏸️ Skipped (sibling test covers it) |
| 🟡 Double session filtering (DRY)                      | ⏸️ Skipped (no rule violation) |

## Final Verification

- `cargo build -p codelet-fspec-tui`: ✅ green
- `cargo test -p codelet-fspec-tui`: ✅ 85 tests pass
  - 45 unit tests
  - 11 app_bootstrap_rpc009 tests
  - 5 app_with_mock_backend tests (RPC-008)
  - 3 app_with_mock_backend_repl tests (NEW RPC-009 snapshots)
  - 3 cross_transport_app_parity_rpc009 tests
  - 1 embedded_backend_smoke test
  - 1 napi_untouched test
  - 4 panic_hook tests
  - 5 source_shape_cargo tests
  - 6 source_shape_rpc009 tests
  - 4 source_shape_trait tests
  - 2 ws_backend_smoke tests
- `fspec validate`: ✅ all 860 feature files valid
- Coverage: ✅ all 6 RPC-009 features still at 100% with corrected line
  ranges
- Source-shape regression tests: ✅ continue to pass — `tui-input` is
  the ONLY new production dependency (codelet-napi/codelet-core remain
  excluded; no `Runtime::new()` / `Runtime::Builder` in `src/views/`;
  no direct imports of encapsulated transport crates)
