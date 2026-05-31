# Bug 2 — `?` and `q` Trapped by App-Level Shortcuts in Rust fspec-tui

**Scope**: Compare keyboard event dispatch between the TS Ink reference frontend (`src/tui/`) and the Rust ratatui port (`codelet/fspec-tui/`), focused on why `?` and `q` keystrokes cannot be typed into the AgentView `MultiLineInput`.

**TL;DR**: The Rust app calls `handle_app_shortcut` (which consumes `?`/`q`/`Ctrl+D`) **before** the navigator forwards the key to the focused `MultiLineInput`. The TS frontend has the inverse precedence — focused text input runs at `InputPriority.MEDIUM` (500) and the global view shortcuts run at `InputPriority.LOW` (200). The fix is to invert the dispatch order in `App::handle_event` so that `Compositor → Navigator → app-shortcut fallback` becomes the new order, while preserving the existing Stage-1 `DisconnectDialog` short-circuit.

---

## 1. TS Ink Frontend — Dispatch Order

### 1.1 Architecture — single `useInput`, priority-based dispatch

The TS TUI does **not** scatter `useInput` hooks across components. There is exactly one `useInput` hook in the entire app, owned by `InputManager` (`src/tui/input/InputManager.tsx:69-89`):

```tsx
// src/tui/input/InputManager.tsx:68-89
useInput(
  (input, key) => {
    const handlers = registry.getOrderedHandlers();

    // Dispatch to handlers in priority order
    for (const handler of handlers) {
      // Skip inactive handlers
      if (!handler.isActive()) {
        continue;
      }

      const handled = handler.handler(input, key);

      if (handled === true) {
        // Handler consumed the input, stop propagation
        return;
      }
    }
  },
  { isActive }
);
```

Handlers are sorted by priority descending, then stable by registration order. Priority constants (`src/tui/input/types.ts:27-38`):

| Level         | Value | Used by                                                                 |
|---------------|------:|-------------------------------------------------------------------------|
| `CRITICAL`    | 1000  | Modal dialogs that block all input                                      |
| `HIGH`        |  800  | Overlays, pause handler (`AgentView.tsx:4463`)                          |
| `MEDIUM`      |  500  | **Text input** (`MultiLineInput.tsx:130`, `InputTransition.tsx:270,297`)|
| `LOW`         |  200  | **View-level shortcuts** (`AgentView.tsx:4551`, `BoardView.tsx:287`)    |
| `BACKGROUND`  |  100  | Passive scroll/navigation (`VirtualList`)                               |

### 1.2 The text input registers at MEDIUM (500) and consumes printables

`MultiLineInput.tsx:128-323` registers via `useInputCompat`:

```tsx
// src/tui/components/MultiLineInput.tsx:127-323 (excerpt)
useInputCompat({
  id: 'multi-line-input',
  priority: InputPriority.MEDIUM,          // 500
  description: 'Multi-line text input keyboard handler',
  isActive,
  handler: (input, key) => {
    // ... cursor movement, history, backspace ...

    // Filter to only printable characters
    const clean = input
      .split('')
      .filter((ch) => {
        const code = ch.charCodeAt(0);
        return code >= 32 && code !== 127;   // includes '?' (63) and 'q' (113)
      })
      .join('');

    if (clean) {
      insertString(clean);
      return true;                            // CONSUMES → halts propagation
    }
    return false;
  },
});
```

Both `?` (codepoint 63) and `q` (codepoint 113) satisfy the `code >= 32 && code !== 127` filter, so they are inserted into the buffer and `return true` halts the dispatch loop at `InputManager.tsx:82-85`.

### 1.3 There is no `?`-opens-Help and no `q`-quits binding in the TS frontend

Confirmed by exhaustive grep across `src/tui/`:
- No `HelpDialog` component exists.
- No handler tests `input === '?'` or `input === 'q'`.
- The visible shortcut hint bar (`KeybindingShortcuts.tsx:14`) lists `C`, `F`, `D`, `/`, `Tab`, `Esc` — not `?` or `q`.
- Quit is performed via `Esc` → `ConfirmationDialog` (e.g. `BoardView.tsx:291-300`, `AgentView.tsx:4733-4773`).

**Implication for the Rust port**: TS has no need to suppress `?`/`q` because they are not globally bound. But the architecture it uses (text input at MEDIUM, view shortcuts at LOW, with `isActive` gating) is the right shape to replicate: focused input handlers run **before** view-level globals, and a `return true` consumes the key.

### 1.4 The `isActive` gating pattern

Each handler self-declares whether it is "focused" via `isActive: boolean | (() => boolean)`. Examples:

- `MultiLineInput.tsx:132` — `isActive` prop driven by parent (false when a dialog blocks input)
- `BoardView.tsx:289` — `isActive: viewMode === 'board' && !showAttachmentDialog && !showCreateSessionDialog && !showExitConfirmation`
- `AgentView.tsx:4553` — `isActive: !showCreateSessionDialog`

There is no separate focus manager — priority + `isActive` IS the focus model.

---

## 2. Rust ratatui Frontend — Dispatch Order (current, broken)

### 2.1 `App::handle_event` — the bug site

**File**: `codelet/fspec-tui/src/app/events.rs:34-71`

```rust
pub fn handle_event(&mut self, event: &Event) -> EventResult {
    let topmost_is_critical = matches!(
        self.compositor.topmost_priority(),
        Some(Priority::Critical)
    );
    let topmost_is_disconnect =
        self.compositor.topmost_id().as_deref() == Some(DISCONNECT_DIALOG_ID);

    // Stage 1: DisconnectDialog short-circuits everything (CORRECT)
    if topmost_is_disconnect {
        return self.handle_disconnect_dialog_event(event);
    }

    // Stage 2: App-level shortcuts (?, q, Ctrl+D) — BUG: runs BEFORE input
    if !topmost_is_critical {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Release {
                if let Some(result) = self.handle_app_shortcut(key) {
                    return result;
                }
            }
        }
    }

    // Stage 3: Compositor (dialog stack)
    let result = self.compositor.handle_event(event);
    if let EventResult::Consumed(Some(callback)) = result {
        callback(&mut self.compositor);
        self.should_render = true;
        return EventResult::consumed();
    }
    if result.is_consumed() {
        self.should_render = true;
        return result;
    }

    // Stage 4: Navigator (Board / Agent / ProviderSettings / Blocklist)
    let nav_result = self.navigator.handle_event(event, &self.board_store);
    if nav_result.is_consumed() {
        self.should_render = true;
    }
    nav_result
}
```

**Order today**: `DisconnectDialog → app-shortcut (?, q, Ctrl+D) → Compositor → Navigator → MultiLineInput textarea`.

The textarea sits at the very end of the chain and never sees `?` or `q` because Stage 2 short-circuits them upstream.

### 2.2 `handle_app_shortcut` — what it traps

**File**: `codelet/fspec-tui/src/app/events.rs:96-113`

```rust
fn handle_app_shortcut(&mut self, key: &KeyEvent) -> Option<EventResult> {
    if key.code == KeyCode::Char('?') && key.modifiers == KeyModifiers::NONE {
        self.compositor.push(Box::new(HelpDialog::new()));
        self.should_render = true;
        return Some(EventResult::consumed());
    }
    if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
        self.should_quit = true;
        return Some(EventResult::consumed());
    }
    if key.code == KeyCode::Char('d')
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        self.should_quit = true;
        return Some(EventResult::consumed());
    }
    None
}
```

Three shortcuts: `?` → HelpDialog, `q` → quit, `Ctrl+D` → quit. Returns `None` for unmatched keys.

### 2.3 Navigator forwards to AgentView's textarea

**File**: `codelet/fspec-tui/src/views/navigator.rs:84-95`

```rust
pub fn handle_event(
    &mut self,
    event: &Event,
    board_store: &BoardStore,
) -> EventResult {
    match self.active_view {
        ViewMode::Board => self.board.handle_event(event, board_store),
        ViewMode::Agent => self.agent.handle_event(event),
        ViewMode::ProviderSettings => self.handle_provider_settings_event(event),
        ViewMode::Blocklist => self.handle_blocklist_event(event),
    }
}
```

For `Agent` mode, `AgentView::handle_event` (`codelet/fspec-tui/src/views/agent/dispatch.rs:194-281`) checks Ctrl+R, mode views, popups, Esc, Ctrl+C, PageUp/PageDown/End, Shift-arrow chords — **none match plain `?` or `q`** — then falls through to:

```rust
// codelet/fspec-tui/src/views/agent/dispatch.rs:259-281
let before = self.input.value();
let outcome = self.input.handle_event(event);
self.sync_popups();
match outcome {
    InputEventOutcome::Submitted(value) => { ... }
    InputEventOutcome::Continued => {
        let after = self.input.value();
        if after != before {
            self.emit(Action::PendingInputChanged(after));
        }
        EventResult::consumed()
    }
    InputEventOutcome::Ignored => EventResult::ignored(),
}
```

And the textarea's `handle_key` catch-all (`codelet/fspec-tui/src/views/agent/multiline_input.rs:168-171`):

```rust
// Everything else → forward to the textarea.
let input = Input::from(crossterm::event::KeyEvent::new(code, mods));
let _ = self.textarea.input(input);
InputEventOutcome::Continued
```

So if `?`/`q` reached the navigator they would be inserted into the buffer — but Stage 2 intercepts them first.

---

## 3. Special-Case Audit — DisconnectDialog and HelpDialog

### 3.1 DisconnectDialog (Critical priority) — `q` interception MUST survive any fix

**File**: `codelet/fspec-tui/src/app/events.rs:39-44, 73-94`

The DisconnectDialog short-circuit at Stage 1 runs **before** `handle_app_shortcut`. The dispatcher uses an **id-based check** (not priority) so it preempts every other layer including the app-shortcut path:

```rust
let topmost_is_disconnect =
    self.compositor.topmost_id().as_deref() == Some(DISCONNECT_DIALOG_ID);
if topmost_is_disconnect {
    return self.handle_disconnect_dialog_event(event);
}
```

And:

```rust
// codelet/fspec-tui/src/app/events.rs:73-94
fn handle_disconnect_dialog_event(&mut self, event: &Event) -> EventResult {
    if let Event::Key(key) = event {
        if key.kind != KeyEventKind::Release {
            if key.code == KeyCode::Char('q') && key.modifiers == KeyModifiers::NONE {
                self.should_quit = true;
                let _ = self.compositor.remove(DISCONNECT_DIALOG_ID);
                return EventResult::consumed();
            }
            if key.code == KeyCode::Char('r') && key.modifiers == KeyModifiers::NONE {
                let _ = self.action_tx.send(Action::ManualReconnect);
                return EventResult::consumed();
            }
        }
    }
    EventResult::consumed()  // Everything else: swallow (no-op)
}
```

**Verdict**: ✅ The DisconnectDialog correctly intercepts `q` only when topmost, via a path that bypasses both the app-shortcut layer and the compositor. **This special case must survive the fix.** Because it runs as Stage 1 (above any inversion), it is unaffected by the proposed reordering of Stages 2–4.

### 3.2 HelpDialog — does it consume `q` to dismiss?

**File**: `codelet/fspec-tui/src/components/help_dialog.rs:63-74`

```rust
fn handle_event(&mut self, event: &Event) -> EventResult {
    if let Event::Key(key) = event {
        if key.code == KeyCode::Esc {
            let id = self.id.clone();
            let callback: Callback = Box::new(move |compositor| {
                let _ = compositor.remove(&id);
            });
            return EventResult::Consumed(Some(callback));
        }
    }
    EventResult::ignored()
}
```

**No** — HelpDialog only consumes `Esc`. It explicitly returns `Ignored` for all other keys.

However, HelpDialog is `Priority::Critical`. With the **current** dispatch order, while HelpDialog is topmost:
- Stage 2 (`!topmost_is_critical`) evaluates **false** → app-shortcut is skipped → `q`/`?` no longer fire at app level.
- Stage 3 (compositor) sees the key, HelpDialog returns `Ignored`.
- Stage 4 (navigator) sees the key; the AgentView's textarea swallows it.

After inverting Stages 2 and 4, the picture stays the same for HelpDialog:
- Stage 3 (compositor) gets the key first → HelpDialog returns `Ignored` for `?`/`q`.
- New Stage 4 (navigator/textarea) consumes the key.
- New Stage 5 (app-shortcut) is not reached because navigator consumed.

**Verdict**: ✅ HelpDialog behaves equivalently before and after the fix. Only `Esc` dismisses it in both orderings. If the user pops HelpDialog and then types `?` into the focused AgentView textarea while HelpDialog is still topmost, the `?` goes into the buffer — which is the desired behaviour and matches what users expect ("dialog up, but typing into input still works"). If we want `?` to also dismiss HelpDialog like Esc, that's a separate enhancement.

---

## 4. Concrete Differences Table

| Concern                                     | TS Ink Frontend (`src/tui/`)                                                   | Rust ratatui (`codelet/fspec-tui/`) — current                                                                  | Required Change                                                                                                |
|---------------------------------------------|--------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| Number of OS-level key sources              | One (`useInput` in `InputManager.tsx:69`)                                      | One (`EventStream` in `App::run`, `events.rs:166`)                                                              | None                                                                                                            |
| Dispatch model                              | Priority-based, single dispatcher iterates registered handlers                 | Hard-coded staged dispatch in `App::handle_event` (`events.rs:34-71`)                                           | Reorder stages so input-bearing view runs before app-shortcut fallback                                          |
| `?` global binding                          | **None**                                                                       | Opens HelpDialog (`events.rs:97-101`)                                                                           | Keep binding, but demote to fallback (after navigator)                                                          |
| `q` global binding                          | **None** (Esc-confirmation pattern instead)                                    | Sets `should_quit` (`events.rs:102-105`)                                                                        | Keep binding, but demote to fallback                                                                            |
| `Ctrl+D` global binding                     | n/a                                                                            | Sets `should_quit` (`events.rs:106-110`)                                                                        | Keep — typically not produced by tui-textarea, so safe either order                                             |
| Focused text input priority                 | `InputPriority.MEDIUM` (500) — `MultiLineInput.tsx:130`                        | Implicit "last in chain" — only reached via Navigator → AgentView → input.handle_event                          | Inversion alone is enough — no need to introduce explicit priorities                                            |
| View-level shortcut priority                | `InputPriority.LOW` (200) — `BoardView.tsx:287`, `AgentView.tsx:4551`          | Inside `AgentView::handle_event` / `BoardView::handle_event` (already after compositor)                         | OK as-is                                                                                                        |
| Modal interception                          | `InputPriority.CRITICAL` (1000) handlers register first                        | Compositor priority bands + Stage-1 DisconnectDialog id-check                                                   | Keep Stage-1 DisconnectDialog; keep `!topmost_is_critical` guard around app-shortcut                            |
| HelpDialog dismiss key                      | n/a (no HelpDialog exists)                                                     | Only `Esc` (`help_dialog.rs:63-74`)                                                                             | OK — works the same before/after the fix                                                                        |
| Consume semantics                           | Handler `return true` → halt loop                                              | `EventResult::Consumed` → return early; `Ignored` → continue                                                    | Already aligned                                                                                                 |
| Navigator return for unmatched keys         | n/a                                                                            | `EventResult::ignored()` (board.rs:114, navigator.rs:102/127, plus AgentView textarea returns Consumed for printables) | Preserve — required for the fallback to fire when AgentView decides not to consume                              |
| `isActive` gating                           | Boolean / closure per handler                                                  | `Component::is_active()` per compositor layer + view-mode discriminator                                         | None — fallback semantics already cover this                                                                    |

---

## 5. Exact Patch Site — `App::handle_event`

**File**: `codelet/fspec-tui/src/app/events.rs`
**Function**: `App::handle_event` (lines 34-71)
**Doc comment** (lines 5-7): also needs updating since it documents the current (broken) order.

### 5.1 Before (current, broken — events.rs:34-71)

```rust
pub fn handle_event(&mut self, event: &Event) -> EventResult {
    let topmost_is_critical = matches!(
        self.compositor.topmost_priority(),
        Some(Priority::Critical)
    );
    let topmost_is_disconnect =
        self.compositor.topmost_id().as_deref() == Some(DISCONNECT_DIALOG_ID);

    if topmost_is_disconnect {
        return self.handle_disconnect_dialog_event(event);
    }

    // BUG: app-shortcut runs BEFORE compositor + navigator → traps ?/q
    if !topmost_is_critical {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Release {
                if let Some(result) = self.handle_app_shortcut(key) {
                    return result;
                }
            }
        }
    }

    let result = self.compositor.handle_event(event);
    if let EventResult::Consumed(Some(callback)) = result {
        callback(&mut self.compositor);
        self.should_render = true;
        return EventResult::consumed();
    }
    if result.is_consumed() {
        self.should_render = true;
        return result;
    }
    let nav_result = self.navigator.handle_event(event, &self.board_store);
    if nav_result.is_consumed() {
        self.should_render = true;
    }
    nav_result
}
```

### 5.2 After (proposed fix)

```rust
pub fn handle_event(&mut self, event: &Event) -> EventResult {
    let topmost_is_critical = matches!(
        self.compositor.topmost_priority(),
        Some(Priority::Critical)
    );
    let topmost_is_disconnect =
        self.compositor.topmost_id().as_deref() == Some(DISCONNECT_DIALOG_ID);

    // Stage 1: DisconnectDialog short-circuit (unchanged) — Critical-id
    // override that bypasses every other layer.
    if topmost_is_disconnect {
        return self.handle_disconnect_dialog_event(event);
    }

    // Stage 2: Compositor (modal dialog stack, priority-ordered).
    let result = self.compositor.handle_event(event);
    if let EventResult::Consumed(Some(callback)) = result {
        callback(&mut self.compositor);
        self.should_render = true;
        return EventResult::consumed();
    }
    if result.is_consumed() {
        self.should_render = true;
        return result;
    }

    // Stage 3: Navigator (BoardView / AgentView / ProviderSettings /
    // Blocklist). The AgentView's MultiLineInput is the focused text
    // sink — it MUST see '?' and 'q' before any app-level fallback.
    let nav_result = self.navigator.handle_event(event, &self.board_store);
    if nav_result.is_consumed() {
        self.should_render = true;
        return nav_result;
    }

    // Stage 4: App-level shortcut FALLBACK (?, q, Ctrl+D) — fires only
    // when nothing upstream consumed the key. Skipped while a Critical
    // dialog is topmost so HelpDialog's Esc-only dismissal is not
    // shadowed by an accidental quit.
    if !topmost_is_critical {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Release {
                if let Some(result) = self.handle_app_shortcut(key) {
                    self.should_render = true;
                    return result;
                }
            }
        }
    }

    nav_result
}
```

### 5.3 Why this works

1. **`?`/`q` typed into AgentView**:
   - Stage 1: no DisconnectDialog → skip.
   - Stage 2: compositor is empty (or HelpDialog returns `Ignored`) → continue.
   - Stage 3: navigator → AgentView → MultiLineInput catch-all inserts char and returns `Consumed`. **Return early.** Stage 4 never fires. ✅
2. **`?` pressed while on BoardView (no input focus)**:
   - Stage 1: skip.
   - Stage 2: compositor empty → continue.
   - Stage 3: BoardView's `handle_event` returns `EventResult::ignored()` for `?` (verified at `board.rs:113-180` — `?` matches no branch and falls through to the `Ignored` arm).
   - Stage 4: fallback fires → `HelpDialog::new()` pushed. ✅
3. **`q` on BoardView**: same as #2 → `should_quit = true`. ✅
4. **DisconnectDialog topmost + `q`**: Stage 1 short-circuit unchanged. ✅
5. **HelpDialog topmost + `?`**: Stage 2 → HelpDialog returns `Ignored`. Stage 3 → BoardView/AgentView either consumes (textarea) or ignores. Stage 4 guard `!topmost_is_critical` is **false** (HelpDialog is Critical) → fallback skipped. Behaviour matches today: only `Esc` dismisses HelpDialog, `?`/`q` are inert at app level while it is up. ✅
6. **`Ctrl+D` typed into AgentView**: `tui-textarea` does NOT insert printable for Ctrl-chord modifiers (only Char with NONE/SHIFT modifiers become text). The Ctrl+D key reaches `multiline_input.rs:168` and is forwarded to `textarea.input()` which for `Ctrl+D` is a no-op delete-forward edit on `tui-textarea`. **Caveat**: this means Ctrl+D MIGHT be consumed by the textarea silently (returning `Continued` → `EventResult::consumed()`) and never reach Stage 4. This is a behaviour change worth calling out — see §6.3 below.

### 5.4 Required doc-comment update

`codelet/fspec-tui/src/app/events.rs:5-7` says:

```
//! The crossterm event flow is:
//!   DisconnectDialog (Critical) → app-shortcuts (`?` / `q` / Ctrl+D)
//!     → Compositor → Navigator → store mutation via [`super::dispatch`].
```

After the fix:

```
//! The crossterm event flow is:
//!   DisconnectDialog (Critical) → Compositor → Navigator →
//!     app-shortcut fallback (`?` / `q` / Ctrl+D) → store mutation via
//!     [`super::dispatch`].
//!
//! The fallback runs LAST so the AgentView's MultiLineInput can type
//! '?' and 'q' as literal characters. App-level shortcuts only fire
//! when no upstream layer consumed the key.
```

Also update `events.rs:31-33` (the `handle_event` doc comment) similarly.

---

## 6. Tests — Existing + Required New

### 6.1 Existing tests touching `?` / `q` at App level

Located via `Grep "KeyCode::Char\('(\?|q)'\)"`:

| File:Line | Test | Affected by inversion? | Action |
|----------|------|------------------------|--------|
| `tests/app_with_mock_backend.rs:76` | `question_at_app_level_pushes_the_help_dialog_onto_the_compositor` | ⚠️ Conditionally | Verify `active_view` after `fresh_app_with_mock_backend()` |
| `tests/app_with_mock_backend.rs:96` | `esc_while_help_dialog_on_top_removes_the_dialog_via_deferred_callback` | ⚠️ Conditionally | Same as above (depends on first `?` press still opening dialog) |
| `tests/app_with_mock_backend.rs:119` | `q_at_app_level_sets_should_quit_and_run_loop_exits` | ⚠️ Conditionally | Same |
| `tests/app_with_mock_backend.rs:144` | `app_with_mock_backend_snapshot_captures_help_dialog_visible_then_dismissed` | ⚠️ Conditionally | Same |
| `tests/disconnect_dialog_slice1_rpc011.rs:228` | DisconnectDialog swallows `?` while topmost | ✅ Unaffected | Stage 1 short-circuit unchanged |
| `tests/disconnect_dialog_slice1_rpc011.rs:282` | `pressing_q_in_disconnect_dialog_exits_the_client_cleanly` | ✅ Unaffected | Stage 1 short-circuit unchanged |
| `tests/rpc028_popup_scroll.rs:357` | `legacy_slash_ordinary_char_is_ignored` (component-level) | ✅ Unaffected | Calls `SlashCommandPopup::handle_key` directly |

The four `app_with_mock_backend.rs` tests are the **canaries**. Their behaviour after the fix depends on what `active_view` the `fresh_app_with_mock_backend()` constructs:

- **If `active_view == Board` at construction** (which is the default per `Navigator::new` at `navigator.rs:67`: `active_view: ViewMode::Board`): BoardView returns `Ignored` for plain `?`/`q` (verified at `board.rs:113-180` — neither key matches any branch). Stage 4 fallback fires → tests continue to pass exactly as written.
- **If `active_view == Agent`**: AgentView's textarea consumes the char → tests would fail. The current `MockBackend` setup leaves the app on Board, so these tests are expected to keep passing.

**Action required**: Run these tests as-is after the fix. If they pass → no test changes needed. If they fail, the `fresh_app_with_mock_backend()` helper must be inspected (it might leave active_view as Board, in which case all four tests stay green).

**Spot-check expectation**: Given `Navigator::default → active_view: Board` and `BoardView::handle_event` ignoring `?`/`q`, all four tests should continue to pass under the fix.

### 6.2 New tests required

Add to a new file `codelet/fspec-tui/tests/rpc073_input_consumes_question_and_q.rs` (or extend `app_with_mock_backend.rs`):

```rust
//! RPC-073: '?' and 'q' typed into the AgentView MultiLineInput must
//! be inserted as literal characters, NOT trapped by the app-level
//! shortcut layer.
//!
//! Feature: spec/features/rpc073-multiline-input-typeable-question-q.feature

use std::sync::Arc;
use codelet_fspec_tui::{synth_key, App, FspecBackend, Priority};
use crossterm::event::KeyCode;
mod common;
use common::{test_app, MockBackend};

#[test]
fn typing_question_mark_into_agent_view_does_not_open_help_dialog() {
    // @step Given an App focused on the AgentView with an attached session
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _term) = test_app(backend);
    // Switch to AgentView (helper depends on existing test scaffolding —
    // may need a new helper like `app.set_active_view(ViewMode::Agent)`
    // or send Shift+Right from Board).
    app.navigator_mut().active_view = codelet_fspec_tui::ViewMode::Agent;

    // @step When the user presses '?'
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));

    // @step Then the HelpDialog is NOT pushed onto the compositor
    assert_ne!(
        app.compositor().topmost_id(),
        Some("help-dialog".to_string()),
        "'?' typed in AgentView must NOT open the HelpDialog"
    );

    // @step And the MultiLineInput buffer now contains "?"
    assert_eq!(app.navigator().agent.input_value(), "?");
}

#[test]
fn typing_q_into_agent_view_does_not_quit_the_app() {
    // @step Given an App focused on the AgentView
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _term) = test_app(backend);
    app.navigator_mut().active_view = codelet_fspec_tui::ViewMode::Agent;

    // @step When the user presses 'q'
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));

    // @step Then should_quit is false
    assert!(!app.should_quit(), "'q' typed in AgentView must NOT quit");

    // @step And the MultiLineInput buffer contains "q"
    assert_eq!(app.navigator().agent.input_value(), "q");
}

#[test]
fn typing_the_phrase_is_this_card_done_question_into_agent_view_works() {
    // @step Given an App focused on the AgentView
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _term) = test_app(backend);
    app.navigator_mut().active_view = codelet_fspec_tui::ViewMode::Agent;

    // @step When the user types "is this card done?"
    for ch in "is this card done?".chars() {
        let _ = app.handle_event(&synth_key(KeyCode::Char(ch)));
    }

    // @step Then the buffer contains the full phrase
    assert_eq!(app.navigator().agent.input_value(), "is this card done?");

    // @step And the HelpDialog was NOT opened
    assert_ne!(
        app.compositor().topmost_id(),
        Some("help-dialog".to_string())
    );

    // @step And the app did NOT quit
    assert!(!app.should_quit());
}

#[test]
fn question_at_app_level_in_board_view_still_opens_help_dialog() {
    // @step Given a fresh App on the BoardView (no input focus)
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _term) = test_app(backend);
    assert_eq!(app.active_view(), codelet_fspec_tui::ViewMode::Board);

    // @step When the user presses '?'
    let _ = app.handle_event(&synth_key(KeyCode::Char('?')));

    // @step Then the HelpDialog IS pushed (fallback fires for BoardView)
    assert_eq!(
        app.compositor().topmost_id(),
        Some("help-dialog".to_string())
    );
    assert_eq!(app.compositor().topmost_priority(), Some(Priority::Critical));
}

#[test]
fn q_at_app_level_in_board_view_still_quits() {
    // @step Given a fresh App on the BoardView
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _term) = test_app(backend);
    assert_eq!(app.active_view(), codelet_fspec_tui::ViewMode::Board);

    // @step When the user presses 'q'
    let _ = app.handle_event(&synth_key(KeyCode::Char('q')));

    // @step Then should_quit is true (fallback fires for BoardView)
    assert!(app.should_quit());
}
```

**Helper accessors that may need adding** (read-only, behind `#[cfg(test)]` or `pub(crate)` exposure):
- `App::navigator()` and `App::navigator_mut()` — likely already exists (used by `app.navigator().id()` in existing tests).
- `AgentView::input_value(&self) -> String` — wrapper around `self.input.value()`. The `MultiLineInput::value()` method exists at `multiline_input.rs` (referenced at `dispatch.rs:259` as `self.input.value()`).

### 6.3 Caveat — Ctrl+D behaviour worth a snapshot test

With the inversion, `Ctrl+D` typed in AgentView could be consumed silently by `tui-textarea` (line 169 of `multiline_input.rs` forwards every unmatched key including Ctrl-chords). Add a confirmation test:

```rust
#[test]
fn ctrl_d_in_board_view_still_quits() {
    // BoardView ignores Ctrl+D → fallback should fire.
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _term) = test_app(backend);
    let key = crossterm::event::KeyEvent::new(
        KeyCode::Char('d'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    let _ = app.handle_event(&crossterm::event::Event::Key(key));
    assert!(app.should_quit());
}

#[test]
fn ctrl_d_in_agent_view_behaviour_is_documented() {
    // Document what Ctrl+D does in AgentView after the fix. If
    // tui-textarea silently swallows it, should_quit stays false and
    // we may need to lift Ctrl+D back into a higher-priority handler
    // OR teach AgentView::handle_event to forward Ctrl+D upward.
    let backend: Arc<dyn FspecBackend> = Arc::new(MockBackend::new());
    let (mut app, _term) = test_app(backend);
    app.navigator_mut().active_view = codelet_fspec_tui::ViewMode::Agent;
    let key = crossterm::event::KeyEvent::new(
        KeyCode::Char('d'),
        crossterm::event::KeyModifiers::CONTROL,
    );
    let _ = app.handle_event(&crossterm::event::Event::Key(key));
    // Pick ONE of these assertions based on observed behaviour:
    // (a) If quit still works: assert!(app.should_quit());
    // (b) If textarea swallows: assert!(!app.should_quit());
    //     → then add a Ctrl+D early-out in AgentView::handle_event.
}
```

If Ctrl+D regresses, the minimal fix is to add a branch in `codelet/fspec-tui/src/views/agent/dispatch.rs:240` next to the existing Ctrl+C branch:

```rust
if key.code == KeyCode::Char('d') && key.modifiers.contains(KeyModifiers::CONTROL) {
    return EventResult::ignored();  // Let Stage 4 fallback handle quit.
}
```

### 6.4 Snapshot tests (insta)

The existing snapshot test `app_with_mock_backend_snapshot_captures_help_dialog_visible_then_dismissed` (`app_with_mock_backend.rs:139-158`) should be re-run. The snapshot itself should not change because:
1. App starts on BoardView.
2. `?` keypress → BoardView ignores → Stage 4 fallback pushes HelpDialog.
3. Render is identical to current.

If `cargo insta` flags a snapshot drift, the test setup changed in some other way; the dispatch fix should not affect rendering.

---

## 7. Summary of Required Changes

| File | Change | Lines |
|------|--------|-------|
| `codelet/fspec-tui/src/app/events.rs` | Move `handle_app_shortcut` block from Stage 2 to Stage 4 (after navigator). Update doc comments at lines 5-7 and 31-33. | 34-71 |
| `codelet/fspec-tui/src/views/agent/dispatch.rs` | (Optional, contingent) Add early `Ignored` return for Ctrl+D so the new Stage 4 fallback can still quit. | ~240 |
| `codelet/fspec-tui/tests/rpc073_input_consumes_question_and_q.rs` | New file. 5–6 tests covering AgentView typing + BoardView fallback. | — |
| `codelet/fspec-tui/src/views/agent/mod.rs` (or similar) | Expose `AgentView::input_value(&self) -> String` test accessor. | — |
| `spec/features/rpc073-multiline-input-typeable-question-q.feature` | New Gherkin feature file referenced by the new tests. | — |

No changes required to:
- `compositor.rs` (priority dispatch already correct)
- `help_dialog.rs` (Esc-only dismiss is fine)
- `disconnect_dialog.rs` (Stage 1 short-circuit unaffected)
- `navigator.rs` (already returns `Ignored` for unmatched keys via BoardView; AgentView consumes textarea input as required)
- `multiline_input.rs` (catch-all already inserts `?`/`q` correctly when reached)

---

## 8. Bottom Line

The Rust port replicates the TS frontend's *components* (compositor = modal stack, navigator = view router, multi-line input = focused text sink) but inverted their *priority semantics*. The TS frontend runs text input at MEDIUM (500) **above** view shortcuts at LOW (200); the Rust app's hard-coded staging runs the global `?`/`q` trap **above** the navigator, which is the opposite ordering. Moving the app-shortcut block to a fallback position after `navigator.handle_event` aligns the Rust behaviour with the TS architecture and restores the ability to type `?` and `q` into the AgentView while preserving Help/Quit/DisconnectDialog semantics in every other context.

