# RPC-102 Research: BoardView Esc Key Handling — TS Parity Gap

## Summary

The Rust TUI app-level Stage-4 fallback shortcut handler in `codelet/fspec-tui/src/app/events.rs` incorrectly binds `KeyCode::Char('q')` to quit the entire application. The TypeScript BoardView on the `codelet-integration` branch never uses `'q'` for exit — it binds `key.escape` to open an exit-confirmation dialog. The Rust port must match TS parity.

## Bug Location

**File**: `codelet/fspec-tui/src/app/events.rs`
**Lines**: 130–133

```rust
// ❌ CURRENT (WRONG)
if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
    self.should_quit = true;
    return Some(EventResult::consumed());
}
```

## TypeScript BoardView Contract (codelet-integration branch)

Source: `git show codelet-integration:src/tui/components/BoardView.tsx`

### 1. Dialog state (lines 72–73)

```typescript
const [showAttachmentDialog, setShowAttachmentDialog] = useState(false);
const [showExitConfirmation, setShowExitConfirmation] = useState(false);
```

### 2. Main input handler (lines 285–305)

```typescript
useInputCompat({
  id: 'board-view-main',
  priority: InputPriority.LOW,
  description: 'Board view main keyboard navigation',
  isActive: viewMode === 'board' && !showAttachmentDialog
    && !showCreateSessionDialog && !showExitConfirmation,
  handler: (input, key) => {
    if (key.escape) {
      if (viewMode === 'checkpoint-viewer' || viewMode === 'changed-files-viewer') {
        setViewMode('board');
        setSelectedWorkUnit(null);
        return true;
      }
      // Show exit confirmation dialog instead of exiting directly
      setShowExitConfirmation(true);
      return true;
    }
    // ...
  }
});
```

### 3. Loading / error state handlers (lines 415–432, 463–481)

Same pattern — separate `useInputCompat` blocks for loading and error states, each guarded by `isActive: !showExitConfirmation`, each maps `key.escape` → `setShowExitConfirmation(true)`. Footer shows **"Press ESC to exit"**.

### 4. Dialog rendering (lines 641–654)

```typescript
{showExitConfirmation && (
  <ConfirmationDialog
    message="Exit fspec?"
    description="Are you sure you want to exit?"
    confirmMode="visual"            // "yesno" in loading/error variants
    riskLevel="medium"
    onConfirm={() => { onExit?.(); }}
    onCancel={() => { setShowExitConfirmation(false); }}
  />
)}
```

**No `'q'` binding exists anywhere in the TS BoardView code.**

## TypeScript AgentView Esc Cascade (parity reference)

Source: `src/tui/components/AgentView.tsx` lines 4794–4834

```typescript
if (key.escape) {
  // Priority 1: Close exit confirmation modal
  if (showExitConfirmation) { setShowExitConfirmation(false); return true; }
  // Priority 2: Close turn modal
  if (showTurnModal) { setShowTurnModal(false); return true; }
  // Priority 4: Disable select mode
  if (isTurnSelectMode) { setIsTurnSelectMode(false); return true; }
  // Priority 5: Interrupt loading or compaction
  if ((displayIsLoading || rustSnapshot.isCompacting) && currentSessionId) {
    sessionInterrupt(currentSessionId);
    refreshRustState(currentSessionId);
    return true;
  }
  // Priority 6: Clear input
  if (inputValue.trim() !== '') { setInputValue(''); return true; }
  // Priority 7: Show exit confirmation if session exists, otherwise exit
  if (currentSessionId) { setShowExitConfirmation(true); } else { onExit(); }
  return true;
}
```

## Rust Port — What's Already Correct

The Esc-cascade machinery for AgentView is already wired:

| File | Lines | Responsibility |
|---|---|---|
| `codelet/fspec-tui/src/views/agent/dispatch.rs` | 176–180 | Emits `Action::AgentEscPressed` on `KeyCode::Esc` |
| `codelet/fspec-tui/src/app/dispatch_rpc051.rs` | 39–84 | Routes `AgentEscPressed`: no session → BackToBoard, running → interrupt, input non-empty → clear, else push ExitConfirmationDialog (guarded by `compositor.contains(EXIT_CONFIRMATION_DIALOG_ID)`) |
| `codelet/fspec-tui/src/components/exit_confirmation_dialog.rs` | 175–199 | Dialog: Esc=Cancel, Left/Right=move, Enter=confirm |

The dialog plumbing is fine. The bug is purely at the **app-level Stage-4 fallback** in `events.rs`.

## Rust Port — Event Dispatch Flow

From `codelet/fspec-tui/src/app/events.rs:30–99`:

```
Stage 1: DisconnectDialog (Critical) — RPC-011 CR-1 (keep 'q'/'r' here)
Stage 2: Compositor (popups, modals)
Stage 3: Navigator (BoardView or AgentView with MultiLineInput)
Stage 4: App-level fallback shortcuts  ← BUG IS HERE
```

BoardView does not have its own `Esc` handler in the Navigator (Stage 3), so the key falls through to `handle_app_shortcut` (Stage 4), which is wired to `'q'` instead of `Esc`.

## Correct Key-Handling Contract (TS → Rust parity)

| Context | Key | Action |
|---|---|---|
| BoardView (normal) | **Esc** | Open "Exit fspec?" `ConfirmationDialog` |
| BoardView (checkpoint / changed-files sub-mode) | **Esc** | Back out to board (no dialog) |
| BoardView (loading / error) | **Esc** | Open "Exit fspec?" (yesno variant) |
| AgentView | **Esc** | Esc-cascade (already correct) |
| ExitConfirmationDialog | **Esc** | Cancel (close dialog) |
| ExitConfirmationDialog | **Left/Right** | Move selection |
| ExitConfirmationDialog | **Enter** | Confirm selected choice |
| DisconnectDialog only | **q** | Quit (legitimate per RPC-011 CR-1 rule [2]) |
| DisconnectDialog only | **r** | Manual reconnect |
| Anywhere else | **q** | ❌ Remove this binding |

## Fix Plan

### 1. Replace `Char('q')` binding in `codelet/fspec-tui/src/app/events.rs`

Change lines 130–133 from:

```rust
if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
    self.should_quit = true;
    return Some(EventResult::consumed());
}
```

To:

```rust
if key.code == KeyCode::Esc && key.modifiers == KeyModifiers::NONE {
    // Parity with TS BoardView: ESC opens "Exit fspec?" confirmation.
    if !self.compositor.contains(BOARD_EXIT_CONFIRMATION_DIALOG_ID) {
        let dialog = BoardExitConfirmationDialog::new()
            .with_action_tx(self.action_tx.clone());
        self.compositor.push(Box::new(dialog));
        self.should_render = true;
    }
    return Some(EventResult::consumed());
}
```

### 2. Add a board exit confirmation dialog component

Either:
- (a) Create a new `BoardExitConfirmationDialog` mirroring the existing `ExitConfirmationDialog` but with TS-parity message "Exit fspec?" and a single confirm action that sets `should_quit = true`, **or**
- (b) Generalize `ConfirmationDialog` to accept a message/action payload and reuse it.

On confirm → emit an action that triggers `should_quit = true`. On Esc/Cancel → remove dialog (callback already supported by Compositor pattern).

### 3. Preserve existing correct bindings

- **Lines 101–122** (`handle_disconnect_dialog_event`): keep `'q'` and `'r'` — these are RPC-011 CR-1 compliant.
- **Lines 134–139** (`Ctrl-D`): TS doesn't have this, but it's a conventional terminal exit. Keep it.
- **Line 125–129** (`?`): keep — HelpDialog binding is TS-parity correct.

### 4. ACDD Test Strategy

Follow the pattern in `codelet/fspec-tui/tests/view_board_unit_rpc012.rs` and `board_agent_navigation_rpc012.rs`:

- Write a failing Rust integration test that asserts pressing `Esc` on the BoardView pushes a `BoardExitConfirmationDialog` onto the compositor and does **NOT** set `should_quit`.
- Write a failing test that asserts pressing `'q'` on the BoardView is **ignored** (no quit, no dialog).
- Write a failing test that asserts pressing `'q'` on the DisconnectDialog still sets `should_quit = true`.
- Write a failing test that pressing `Enter` on the BoardExitConfirmationDialog with "Exit" selected sets `should_quit = true`.
- Write a failing test that pressing `Esc` on the BoardExitConfirmationDialog dismisses it without quitting.

## Acceptance Criteria (to be expanded via Example Mapping)

- **AC-1**: Pressing `Esc` on BoardView (no overlay) pushes BoardExitConfirmationDialog; `should_quit` remains false.
- **AC-2**: Pressing `q` anywhere (except DisconnectDialog) is ignored; `should_quit` remains false.
- **AC-3**: BoardExitConfirmationDialog confirm action sets `should_quit = true`.
- **AC-4**: BoardExitConfirmationDialog `Esc` dismisses the dialog without quitting.
- **AC-5**: DisconnectDialog `q`/`r` bindings unchanged (RPC-011 CR-1 regression guard).
- **AC-6**: AgentView Esc-cascade unchanged (regression guard).
