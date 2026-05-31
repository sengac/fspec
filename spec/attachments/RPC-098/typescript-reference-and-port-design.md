# RPC-098 — AgentView ESC Exit Confirmation Dialog: TS Reference & Rust Port Design

## Problem Statement

Pressing **ESC** inside the Rust `AgentView` currently falls straight through to
`Action::BackToBoard` once levels 1–6 of the Esc-cascade are exhausted
(`codelet/fspec-tui/src/app/dispatch_rpc051.rs:55-65`). The fall-through code
even leaves the TODO comment:

```rust
// L7 (deferred to follow-up card) — exit confirmation. For
// RPC-095 we navigate straight back to the Board.
let _ = self.action_tx.send(Action::BackToBoard);
```

The TypeScript Ink reference frontend (`src/tui/components/AgentView.tsx`) does
**not** behave that way. When ESC is pressed and an active session exists, it
opens a **three-button confirmation modal** offering **Detach**, **Close
Session**, and **Cancel**.

This work unit is the long-deferred follow-up: port the TS exit-confirmation
modal to the Rust frontend with full behavioural parity.

---

## TypeScript Reference (Read-Only Source)

### 1. ESC priority handler — `src/tui/components/AgentView.tsx:4731-4773` (TUI-045)

```ts
if (key.escape) {
  // Priority 1: Close exit confirmation modal
  if (showExitConfirmation) { setShowExitConfirmation(false); return true; }
  // Priority 2: Close turn modal
  if (showTurnModal)        { setShowTurnModal(false);        return true; }
  // Priority 4: Disable select mode
  if (isTurnSelectMode)     { setIsTurnSelectMode(false);     return true; }
  // Priority 5: Interrupt loading / compaction
  if ((displayIsLoading || rustSnapshot.isCompacting) && currentSessionId) {
    sessionInterrupt(currentSessionId);
    refreshRustState(currentSessionId);
    return true;
  }
  // Priority 6: Clear non-empty input
  if (inputValue.trim() !== '') { setInputValue(''); return true; }
  // Priority 7: Show confirmation, or exit if no session
  if (currentSessionId) { setShowExitConfirmation(true); }
  else                  { onExit(); }
  return true;
}
```

### 2. Dialog mount — `src/tui/components/AgentView.tsx:5502-5515` (TUI-046)

```tsx
{showExitConfirmation && (
  <ThreeButtonDialog
    message="Exit Session?"
    description={
      displayIsLoading
        ? 'The agent is currently running. Choose how to exit.'
        : 'Choose how to exit the session.'
    }
    options={['Detach', 'Close Session', 'Cancel']}
    defaultSelectedIndex={0}
    onSelect={handleExitChoice}
    onCancel={() => setShowExitConfirmation(false)}
  />
)}
```

### 3. Choice dispatcher — `src/tui/components/AgentView.tsx:4391-4426` (TUI-046, REFAC-008, TUI-068, SESS-001)

```ts
const handleExitChoice = useCallback(
  async (index: number, _option: string) => {
    setShowExitConfirmation(false);
    if (index === 2) return;                       // Cancel
    if (index === 0) {                             // Detach
      cleanupCurrentSessionHandler();
      onExit();
    } else if (index === 1) {                      // Close Session
      cleanupCurrentSessionHandler();
      if (currentSessionId) {
        try { await destroySession(currentSessionId); }
        catch (err) { logger.error('Failed to destroy session:', err); }
      }
      onExit();
    }
  },
  [currentSessionId, onExit],
);
```

### 4. Reusable ThreeButtonDialog — `src/components/ThreeButtonDialog.tsx`

| Prop                   | Value                                                    |
|------------------------|----------------------------------------------------------|
| `message`              | `"Exit Session?"` (bold)                                 |
| `description`          | conditional dim text                                     |
| `options`              | `['Detach', 'Close Session', 'Cancel']`                  |
| `defaultSelectedIndex` | `0`                                                      |
| `onSelect`             | `handleExitChoice`                                       |
| `onCancel`             | closes the modal                                         |

Visuals:

- **Border**: `borderColor="yellow"`, round
- **Selected button**: `bg=blue`, `fg=white`, **bold**, label padded ` <label> `
- **Unselected button**: `fg=gray`, no bg
- **Footer (dim, centered)**: `← → Navigate | Enter Select | Esc Cancel`
- Layout: horizontal row, `justifyContent="center"`, `marginX={1}` between buttons
- Keys: **←** prev (wrap), **→** next (wrap), **Enter** commits, **ESC** cancels

---

## Current Rust State

### Files involved

| Concern                        | Path                                                                  |
|--------------------------------|-----------------------------------------------------------------------|
| Esc-cascade fall-through (L7)  | `codelet/fspec-tui/src/app/dispatch_rpc051.rs:55-65`                  |
| AgentView ESC emission         | `codelet/fspec-tui/src/views/agent/dispatch.rs:176-181`               |
| `BackToBoard` dispatch         | `codelet/fspec-tui/src/app/dispatch.rs:102` + `views/navigator.rs:158`|
| Backend destroy_session RPC    | `codelet/fspec-tui/src/transport/{embedded,websocket,mod}.rs`         |
| Dialog primitive               | `codelet/fspec-tui/src/components/dialog_theme.rs::render_dialog`     |
| Three-flat-options precedent   | `codelet/fspec-tui/src/components/create_session_dialog.rs`           |
| Action enum                    | `codelet/fspec-tui/src/components/mod.rs:482` (Action variants)       |
| Compositor push/remove         | RPC-027 / RPC-079 infrastructure                                      |

### Gap

There is no `ThreeButtonDialog` wrapper in the Rust port (RPC-079 ported
ErrorDialog / NotificationDialog / StatusDialog only). The CreateSessionDialog
in `components/create_session_dialog.rs` is the closest precedent for
three-flat-cyclic-options + cyan accent + centred row, and provides the
template we follow.

---

## Rust Port Design

### New component: `ExitConfirmationDialog`

```
codelet/fspec-tui/src/components/exit_confirmation_dialog.rs   (NEW, ~250 LoC)
```

- `pub const EXIT_CONFIRMATION_DIALOG_ID: &str = "exit-confirmation-dialog";`
- `pub enum ExitChoice { Detach, CloseSession, Cancel }`
- Constructor: `ExitConfirmationDialog::new(is_busy: bool)` — chooses description text
- Accent: **`Accent::Yellow`** (matches TS `borderColor='yellow'`)
- Title: `"Exit Session?"` (bold)
- Description (dim, conditional):
  - `is_busy = true` → `"The agent is currently running. Choose how to exit."`
  - `is_busy = false` → `"Choose how to exit the session."`
- Three buttons: `Detach`, `Close Session`, `Cancel` — same selected/unselected styling as `CreateSessionDialog` (`bg=Blue,fg=White,bold` vs `fg=Gray`)
- Footer: `"← → Navigate | Enter Select | Esc Cancel"`
- Default selection: `Detach` (index 0)
- Keys: ←/→ cyclic move, Enter commits via `Action::AgentExitChoice { choice }`, ESC commits `Action::AgentExitChoice { choice: ExitChoice::Cancel }`
- Mouse: ScrollUp/Left → move_left, ScrollDown/Right → move_right (parity with CreateSessionDialog)
- Delegates rendering to `dialog_theme::render_dialog` (RPC-079 RULE-4: no hand-rendering)
- Implements `Component` with `Priority::Critical` (parity with `PauseDialog`, since this dialog must overlay any other layer when the user wants to exit)
- Callback on Enter/ESC: `compositor.remove(EXIT_CONFIRMATION_DIALOG_ID)`

### New Action variant

```rust
// codelet/fspec-tui/src/components/mod.rs
pub enum Action {
    // ...
    /// RPC-098 — user picked an option from the exit-confirmation dialog.
    AgentExitChoice { choice: ExitChoice },
}

pub enum ExitChoice { Detach, CloseSession, Cancel }
```

### Modified Esc cascade (L7) — `dispatch_rpc051.rs`

Replace the current fall-through:

```rust
// BEFORE
let _ = self.action_tx.send(Action::BackToBoard);
```

with:

```rust
// AFTER — RPC-098
let is_busy = matches!(
    self.agent_view_store.session_status_for(&session).copied(),
    Some(SessionStatus::Running) | Some(SessionStatus::Compacting),
);
let dialog = ExitConfirmationDialog::new(is_busy)
    .with_action_tx(self.action_tx.clone());
self.compositor.push(Box::new(dialog));
```

Note: the L4 branch (active session → `backend.interrupt`) is *unchanged* —
ESC during a running stream still interrupts first; only after the stream is
idle does L7 reach the dialog. This matches the TS priority order
(`displayIsLoading` is L5 in TS, before L6 clear-input and L7 confirm).

### New dispatch handler — `app/dispatch_rpc098.rs` (NEW)

```rust
impl App {
    pub(crate) fn handle_agent_exit_choice(&mut self, choice: ExitChoice) {
        // Dialog already self-removes from compositor via Callback.
        match choice {
            ExitChoice::Cancel => { /* stay on AgentView, no-op */ }
            ExitChoice::Detach => {
                let _ = self.action_tx.send(Action::BackToBoard);
            }
            ExitChoice::CloseSession => {
                if let Some(session) = self.agent_view_store.current_session().cloned() {
                    let backend = self.backend.clone();
                    let handle = tokio::spawn(async move {
                        let _ = backend.destroy_session(session).await;
                    });
                    self.pending_tasks.push(handle);
                }
                let _ = self.action_tx.send(Action::BackToBoard);
            }
        }
    }
}
```

Wire it in `app/dispatch_rpc022.rs` next to the other Agent* actions:

```rust
Action::AgentExitChoice { choice } => self.handle_agent_exit_choice(choice),
```

---

## Acceptance Criteria (preview — finalised via Example Mapping)

1. **Dialog opens at L7** when ESC is pressed and L1-L6 do not consume.
2. **Dialog does NOT open** when no session exists (current direct
   `BackToBoard` behaviour preserved — matches TS `if (currentSessionId)`).
3. **Description text** reads `"The agent is currently running. Choose how to
   exit."` when session is Running/Compacting, else
   `"Choose how to exit the session."`.
4. **Defaults**: title bold `"Exit Session?"`, yellow border (Accent::Yellow),
   focus on Detach (index 0), footer
   `"← → Navigate | Enter Select | Esc Cancel"`.
5. **Cyclic navigation**: ← from Detach wraps to Cancel; → from Cancel wraps
   to Detach.
6. **Enter on Detach** → `Action::AgentExitChoice { choice: Detach }` →
   `BackToBoard`; **no** `destroy_session` call on backend.
7. **Enter on Close Session** → `Action::AgentExitChoice { choice:
   CloseSession }` → spawns `backend.destroy_session(id)` task **then**
   `BackToBoard`.
8. **Enter on Cancel** → `Action::AgentExitChoice { choice: Cancel }` → dialog
   removes, AgentView remains active, no view transition.
9. **ESC inside dialog** → equivalent to Cancel (per TS `onCancel`).
10. **Priority**: `Priority::Critical` so the modal sits above any popup /
    mode view.
11. **No double-open**: pressing ESC again while the dialog is on the
    compositor does NOT push a second dialog (compositor `push` is idempotent
    via `EXIT_CONFIRMATION_DIALOG_ID`, OR L7 checks `compositor.contains(id)`
    first — implementation choice during testing phase).
12. **Render delegation**: the new component delegates to
    `dialog_theme::render_dialog`; no hand-rendering with `Block`/`Paragraph`.
13. **Insta snapshot** for the dialog on an 80×24 buffer in both `is_busy=true`
    and `is_busy=false` cases.

---

## Out of Scope

- Adding a *generic* `ThreeButtonDialog` Rust wrapper (TS has one but RPC-079
  decision was to keep one-off dialogs delegating to `dialog_theme`; we follow
  that convention here. A separate refactor card may unify
  CreateSessionDialog + ExitConfirmationDialog later.)
- Changing the L1-L6 cascade behaviour.
- Modifying `destroy_session` backend semantics.

---

## Reference Tags (TS)

- **TUI-045** — ESC priority order in AgentView
- **TUI-046** — Exit confirmation modal + handleExitChoice
- **TUI-068** — `destroySession` from sessionService (work-unit detach included)
- **REFAC-008** — Cleanup of local session handler before detach/destroy
- **SESS-001** — Session lifecycle logging
