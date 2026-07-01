# RPC-395 — Board '.' key starts new agent

## Problem

In the TypeScript reference implementation (`src/tui/components/BoardView.tsx`),
pressing the **`/`** key on the Kanban board starts a new agent (opens the
AgentView / create-session flow):

```tsx
// src/tui/components/BoardView.tsx (~line 358-362)
// '/' key to navigate to first session or show create dialog (same as Shift+Right)
if (input === '/') {
  handleShiftRight();
  return true;
}
```

In the **Rust TUI port** (`codelet/fspec-tui`), the only ways to start a new
agent from the board are:

1. **Shift+Right** — `board.rs:113-118` emits `Action::OpenAgentView(target)`.
2. **Enter** on a selected work unit — `board.rs:120-127` emits
   `Action::EnterWorkUnit(unit.id)`.

The board header hint row advertises a `/ New Agent` chord
(`keybinding_shortcuts.rs:32`), but **no key handler is wired** for `/` (it is
"hint-only" per the module doc comment). The desired behavior for the Rust port
is to use **`.` (period)** as the single-key new-agent shortcut and to update
the header hint accordingly.

## Goal

1. Pressing **`.`** on the board starts a new agent, mirroring the Shift+Right
   handler exactly: emit `Action::OpenAgentView(self.selected_session(store))`.
2. Update the board header hint from `/ New Agent` to `. New Agent`.

> Note: The Rust port deliberately diverges from the TS key (`/` → `.`) per the
> product owner's decision. This is a conscious, documented divergence.

## Scope of Changes

### Source
| File | Change |
|------|--------|
| `codelet/fspec-tui/src/views/board.rs` | Add modifier-free `KeyCode::Char('.')` handler emitting `Action::OpenAgentView(self.selected_session(store))`. |
| `codelet/fspec-tui/src/views/board/keybinding_shortcuts.rs` | Line 32 rendered string `/ New Agent` → `. New Agent`; sync doc comments (lines 8, 10-11). |

### Tests / snapshots
| File | Change |
|------|--------|
| `codelet/fspec-tui/tests/view_board_unit_rpc015.rs` | Update assertion `"/ New Agent"` → `". New Agent"` (lines ~128-132). |
| `tests/snapshots/app_with_mock_backend__help_dialog_dismissed.snap` | `/ New Agent` → `. New Agent`. |
| `tests/snapshots/app_with_mock_backend_repl__repl_bootstrap_rpc012.snap` | `/ New Agent` → `. New Agent`. |
| `tests/snapshots/app_with_mock_backend__help_dialog_visible.snap` | `/ New Agent` → `. New Agent`. |
| `codelet/fspec-tui/tests/board_period_new_agent_rpc395.rs` (NEW) | RPC-395 behavior tests for `.` key + header string. |

## Acceptance Criteria (Gherkin)

Feature: `spec/features/board-key-starts-new-agent.feature`

1. **Pressing `.` with a selected work unit** → `Action::OpenAgentView` emitted
   for that work unit's session.
2. **Pressing `.` with no work unit selected** → `Action::OpenAgentView(None)`
   emitted (mirrors Shift+Right with nothing selected).
3. **Header hint row** → rendered buffer contains `". New Agent"` and does NOT
   contain `"/ New Agent"`.

## Business Rules

- **R1**: Pressing `.` on the board opens the AgentView using the selected work
  unit's session (same behavior as Shift+Right).
- **R2**: The `.` key handler is modifier-free (no Ctrl/Shift) so it does not
  conflict with reserved chords (e.g. Ctrl+D hard-quit).
- **R3**: The board header hint text reads `. New Agent` instead of `/ New Agent`.

## Implementation Notes

- Mirror the existing Shift+Right handler at `board.rs:113-118`. The cleanest
  placement is a new arm in the `match key.code { ... }` block (board.rs:129-207)
  before `_ => {}`, guarded modifier-free like the `a`/`d` arms:

  ```rust
  // RPC-395: '.' starts a new agent (mirror of Shift+Right).
  KeyCode::Char('.') if !key.modifiers.contains(KeyModifiers::CONTROL) => {
      let target = self.selected_session(store);
      self.emit(Action::OpenAgentView(target));
      return EventResult::consumed();
  }
  ```

- Keep `board.rs` under 300 LoC; if it grows, factor the key dispatch into a
  submodule (check current LoC first).

## ACDD Workflow

1. Write the new RPC-395 test file (failing) covering all 3 scenarios.
2. Update the RPC-015 assertion + 3 snapshots to `. New Agent` (these will fail
   until the string change lands — expected red phase).
3. Implement the `.` handler + header string change.
4. Run `cargo test -p codelet-fspec-tui`, `cargo clippy -p codelet-fspec-tui`
   (0 warnings), `cargo fmt --check`.
5. Link coverage for each scenario.
