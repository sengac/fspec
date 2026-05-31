# RPC-079 AST Research — Existing Dialog Baseline & Inline FspecDialog Audit

Date: 2026-05-28
Scope: `codelet/fspec-tui/src/`
Tooling: `AstGrep` / `Grep` (read-only AST queries).

## Goal

Establish the baseline patterns the three new wrappers
(`ErrorDialog` / `NotificationDialog` / `StatusDialog`) must follow,
and audit every existing `FspecDialog { … }` literal so rule [8] (no
inline error/notification/progress modal struct literals after this
work unit) is verifiable.

## 1. All existing `FspecDialog { … }` literals in `components/`

AstGrep pattern: `let dialog = FspecDialog { $$$FIELDS };` —
language=rust, scope=`codelet/fspec-tui/src/components/`.

| File                                  | Line | Accent  | Purpose                                  |
|---------------------------------------|------|---------|------------------------------------------|
| `components/help_dialog.rs`           | 85   | Cyan    | `?` help overlay                         |
| `components/pause_dialog.rs`          | 266  | Yellow  | HITL pause approval                      |
| `components/hitl_dialog.rs`           | 314  | Cyan    | `request_user_input` prompt              |
| `components/model_selector_dialog.rs` | 251  | Cyan    | `/model` picker                          |
| `components/role_dialog.rs`           | 180  | Cyan    | `/role` text input                       |
| `components/thinking_level_dialog.rs` | 184  | Yellow  | `/thinking` level picker                 |
| `components/disconnect_dialog.rs`     | 121  | Red     | WebSocket disconnect modal               |
| `components/create_session_dialog.rs` | 249  | Cyan    | `/new` session form                      |

**Finding**: Every existing literal lives inside the `render()` method
of a `Component` that delegates to `dialog_theme::render_dialog`. None
implement generic error / notification / progress semantics. Rule [8]
is therefore satisfied trivially at the START of RPC-079 — we only
have to avoid REGRESSING when adding the new wrappers, and verify zero
inline literals for these three semantics remain afterwards.

## 2. Inline `FspecDialog { … }` literals OUTSIDE `components/`

AstGrep / Grep on `codelet/fspec-tui/src/views/`:

| File                                              | Accent  | Purpose                                  |
|---------------------------------------------------|---------|------------------------------------------|
| `views/agent/slash_command_popup.rs:238`          | Cyan    | Slash-command list picker                |
| `views/agent/file_search_popup.rs:228`            | Cyan    | `@file` mention picker                   |
| `views/agent/confirm_dialog.rs:200`               | Yellow  | RPC-026 generic two-button confirm       |
| `views/agent/merge_confirm_dialog.rs:205`         | Yellow  | RPC-057 merge worktree three-button      |

**Finding**: All four are list-pickers or button-confirmations — none
are error / notification / progress semantics. No refactor needed for
rule [8].

## 3. Dialog-id constant naming pattern

AstGrep pattern: `pub const $NAME: &str = $VAL;` — restricted to
`*_DIALOG_ID` constants.

| File                                  | Constant                        | Value                       |
|---------------------------------------|---------------------------------|-----------------------------|
| `disconnect_dialog.rs`                | `DISCONNECT_DIALOG_ID`          | `"disconnect-dialog"`       |
| `help_dialog.rs` (inline)             | (none — `"help-dialog"` inline) | `"help-dialog"`             |
| `pause_dialog.rs`                     | `PAUSE_DIALOG_ID`               | `"pause-dialog"`            |
| `hitl_dialog.rs`                      | `HITL_DIALOG_ID`                | `"hitl-dialog"`             |
| `model_selector_dialog.rs`            | `MODEL_SELECTOR_DIALOG_ID`      | `"model-selector-dialog"`   |
| `role_dialog.rs`                      | `ROLE_DIALOG_ID`                | `"role-dialog"`             |
| `thinking_level_dialog.rs`            | `THINKING_LEVEL_DIALOG_ID`      | `"thinking-level-dialog"`   |
| `create_session_dialog.rs`            | `CREATE_SESSION_DIALOG_ID`      | `"create-session-dialog"`   |
| `views/agent/merge_confirm_dialog.rs` | `MERGE_CONFIRM_DIALOG_ID`       | `"merge-confirm-dialog"`    |

**Decision for RPC-079**:
- `ERROR_DIALOG_ID = "error-dialog"`
- `NOTIFICATION_DIALOG_ID = "notification-dialog"`
- `STATUS_DIALOG_ID = "status-dialog"`

## 4. Critical-priority + Callback dismissal pattern

Reference: `components/disconnect_dialog.rs` (shortest example).

```rust
impl Component for DisconnectDialog {
    fn priority(&self) -> Priority { Priority::Critical }
    fn id(&self) -> &str { &self.id }
    fn handle_event(&mut self, event: &Event) -> EventResult {
        // ... match KeyCode::Esc ...
        let id = self.id.clone();
        let callback: Callback = Box::new(move |compositor| {
            let _ = compositor.remove(&id);
        });
        EventResult::Consumed(Some(callback))
    }
    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let dialog = FspecDialog { accent: Accent::Red, title: "Disconnected",
                                    rows: self.body_rows(), footer: "",
                                    min_width: 50 };
        render_dialog(area, buf, &dialog);
    }
}
```

All three new wrappers follow this pattern exactly.

## 5. Auto-dismiss timer pattern (NEW for RPC-079)

No existing dialog auto-dismisses on a timer. We introduce the pattern
here using:

1. A new `Action::DismissDialog(String)` variant — payload is the
   dialog id (matches the existing `id`-string addressing used by
   `Compositor::remove`).
2. `App::dispatch` handles `Action::DismissDialog(id)` by calling
   `self.compositor.remove(&id)` and setting `self.should_render =
   true`. Routed via `try_dispatch_rpc079` to keep `dispatch.rs`
   under the 300-LoC ceiling enforced by the source-shape tests.
3. The dialog stores `created_at: tokio::time::Instant`,
   `auto_dismiss_ms: u64`, and `action_tx:
   Option<UnboundedSender<Action>>`. On `arm()`, it spawns a
   `tokio::spawn` task that `sleep`s for `auto_dismiss_ms` and then
   sends `Action::DismissDialog(self.id())`. ESC aborts the task.
4. Test pattern: `#[tokio::test(start_paused = true)]` +
   `tokio::time::advance(Duration::from_secs(N)).await` exercises the
   countdown deterministically. Requires
   `tokio = { workspace = true, features = ["test-util"] }` as a
   dev-dependency (added to `codelet/fspec-tui/Cargo.toml`).

## 6. Test surface to mirror

- Inline `#[cfg(test)] mod tests` in each `*_dialog.rs` file with:
  - `*_priority_is_critical()` — pin `Priority::Critical` + canonical id
  - `*_renders_required_literal_strings()` — assert title/body/footer
    landed in the 80x24 buffer
  - `*_rendering_is_byte_equal_across_runs_insta_snapshot()` — insta
    YAML snapshot of all 24 rows
- Integration test `tests/dialogs_rpc079.rs`:
  - ESC dismissal emits the canonical `Action::DismissDialog(id)` /
    `compositor.remove(id)` callback
  - NotificationDialog auto-dismiss timing (1s countdown, 2s fire)
    under `tokio::time::pause()`
  - StatusDialog state transitions (Restoring→Complete→auto-close,
    Restoring→Error→ESC-dismiss)
  - Repo-wide grep assertion: no inline `FspecDialog {` literal
    appears in `src/` outside the legal allow-list (the 8 existing
    component dialogs + the 4 existing view dialogs + the 3 new
    wrappers introduced by this work unit).
