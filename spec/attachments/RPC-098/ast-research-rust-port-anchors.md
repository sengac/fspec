# RPC-098 — AST Research: Anchors for the Rust Port

This research surveys the existing Rust ratatui codebase to identify the
exact functions, types, and integration points the RPC-098 implementation
will touch. All paths are relative to repository root.

## 1. Existing three-flat-options dialog precedent

**Pattern**: `impl Component for CreateSessionDialog`

| File | Line | Match |
|------|------|-------|
| `codelet/fspec-tui/src/components/create_session_dialog.rs` | 158 | `impl Component for CreateSessionDialog { ... }` |

CreateSessionDialog (read in turn 6705) is the closest precedent for the new
ExitConfirmationDialog. It already shows:

- The `Component` impl with `priority()`, `id()`, `handle_event()`,
  `render()`
- A three-variant enum `CreateSessionOption { Yes, Isolated, Cancel }` with a
  fixed `OPTIONS: [_; 3]` table
- Cyclic Left/Right (`move_left`, `move_right`) via index arithmetic on
  `OPTIONS`
- `with_action_tx(self, UnboundedSender<Action>)` builder method
- `Callback`-based removal via `compositor.remove(&id)` on Enter/ESC
- Delegation to `dialog_theme::render_dialog(area, buf, &FspecDialog{..})`
- ASCII pipe footer, dim styled spans, centered button row with
  `selected_style = bg(Blue).fg(White).bold` and `unselected_style =
  fg(Gray)`

**Decision**: clone the structural template of `create_session_dialog.rs`
verbatim. Replace:

- `CreateSessionOption` → `ExitChoice`
- `Yes/Isolated/Cancel` → `Detach/CloseSession/Cancel`
- Accent::Cyan → **Accent::Yellow**
- `Action::CreateSessionSubmitted{..}` / `Action::CreateSessionCancelled` →
  `Action::AgentExitChoice { choice }`
- `Priority::Foreground` → **`Priority::Critical`** (parity with PauseDialog
  — the exit modal must overlay everything when the user wants out)
- `CREATE_SESSION_DIALOG_ID` → `EXIT_CONFIRMATION_DIALOG_ID`
- title `"Work on <id>?"`/`"Start New Agent?"` → static `"Exit Session?"`
- description text → conditional on `is_busy: bool`

## 2. Esc-cascade entry point (will be modified)

**Pattern**: `pub fn handle_agent_esc_pressed(&mut self)`

| File | Line | Match |
|------|------|-------|
| `codelet/fspec-tui/src/app/dispatch_rpc051.rs` | 33 | `pub(crate) fn handle_agent_esc_pressed(&mut self) { ... }` |

The current L7 fall-through (lines 55-65) sends `Action::BackToBoard` and
carries a TODO comment:

```rust
// L7 (deferred to follow-up card) — exit confirmation. For
// RPC-095 we navigate straight back to the Board.
let _ = self.action_tx.send(Action::BackToBoard);
```

**RPC-098 replaces this branch** with:

```rust
if !self.compositor.contains(EXIT_CONFIRMATION_DIALOG_ID) {
    let is_busy = matches!(
        self.agent_view_store.session_status_for(&session).copied(),
        Some(SessionStatus::Running) | Some(SessionStatus::Compacting),
    );
    let dialog = ExitConfirmationDialog::new(is_busy)
        .with_action_tx(self.action_tx.clone());
    self.compositor.push(Box::new(dialog));
}
```

Note: the L4 active-session branch (Running/Compacting) already returns early
with a `backend.interrupt(session)` spawn, so `is_busy` should always be
`false` by the time the new code runs — but we compute it defensively
because future changes to the cascade may relax the L4 gate. This also
provides a single integration point for the (now-impossible-but-spec'd) busy
description text to remain accurate.

## 3. Backend destroy_session contract

**Pattern**: `async fn destroy_session(&self, SessionId) -> Result<()>`

| File | Line | Match |
|------|------|-------|
| `codelet/fspec-tui/src/transport/embedded.rs`  | 568 | `async fn destroy_session(&self, session_id: SessionId) -> Result<()> { ... }` |
| `codelet/fspec-tui/src/transport/mod.rs`       | 517 | trait default impl |
| `codelet/fspec-tui/src/transport/websocket.rs` | 924 | `async fn destroy_session(&self, session_id: SessionId) -> Result<()> { ... }` |

The Backend trait exposes `destroy_session(SessionId) -> Result<()>` across
all three transports (embedded, websocket, and mock in `mod.rs`). The
**CloseSession** branch in the new `dispatch_rpc098.rs` will call this in
the same way `dispatch_rpc051.rs` line 50 already invokes
`backend.interrupt(session).await` — via `tokio::spawn` plus
`self.pending_tasks.push(handle)`.

## 4. Compositor lifecycle API

**Pattern**: `impl Compositor` + `push/remove/contains`

| File | Line | Match |
|------|------|-------|
| `codelet/fspec-tui/src/compositor.rs` | 36 | `impl Compositor { ... }` |
| `codelet/fspec-tui/src/compositor.rs` | 45 | `pub fn push(&mut self, component: Box<dyn Component>)` |
| `codelet/fspec-tui/src/compositor.rs` | 70 | `pub fn remove(&mut self, id: &str) -> Option<Box<dyn Component>>` |
| `codelet/fspec-tui/src/compositor.rs` | 92 | `pub fn contains(&self, id: &str) -> bool` |

Confirms the three API methods the RPC-098 design uses already exist. The
no-double-push idempotence rule (Rule 10) is straightforward to implement
because `Compositor::contains` is already public.

## 5. Action enum location

**Pattern**: `Action` variants in `components/mod.rs`

| File | Line | Match |
|------|------|-------|
| `codelet/fspec-tui/src/components/mod.rs` | 482 | `AgentEscPressed,` (existing sibling variant) |

The new `Action::AgentExitChoice { choice: ExitChoice }` variant will be
added next to `AgentEscPressed`, alongside its companion `pub enum ExitChoice`.

## 6. Dispatch wiring location

The dispatch table that routes `Action::AgentEscPressed` to
`handle_agent_esc_pressed()` lives in `codelet/fspec-tui/src/app/dispatch_rpc022.rs:241`:

```rust
Action::AgentEscPressed => self.handle_agent_esc_pressed(),
```

RPC-098 adds a sibling line:

```rust
Action::AgentExitChoice { choice } => self.handle_agent_exit_choice(choice),
```

## Summary of Anchors

| Concern | File | Existing? | Action |
|---------|------|-----------|--------|
| Dialog skeleton | `components/create_session_dialog.rs` | ✓ | clone structure into `exit_confirmation_dialog.rs` |
| Dialog primitive | `components/dialog_theme.rs::render_dialog` | ✓ | delegate (per RPC-079 Rule 4) |
| Esc cascade L7 | `app/dispatch_rpc051.rs:55-65` | ✓ | replace `BackToBoard` with `compositor.push(dialog)` |
| Action::AgentExitChoice | `components/mod.rs:482` | ✗ | add new variant + ExitChoice enum |
| Dispatcher | `app/dispatch_rpc098.rs` | ✗ | new file |
| Dispatch wiring | `app/dispatch_rpc022.rs:241` | ✓ | add new arm |
| Backend destroy_session | `transport/{embedded,websocket,mod}.rs` | ✓ | call via tokio::spawn |
| Compositor::contains | `compositor.rs:92` | ✓ | use for idempotence |
| Component trait | derived from CreateSessionDialog | ✓ | impl Critical priority |
