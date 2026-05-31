# Dialog Reuse Audit (Why RPC-078 needs ZERO new dialogs)

RPC-078 is **inline-conversation-text** work, not modal-dialog work.
This file documents the audit that confirmed no dialog changes are
required, and pins the conventions that any future related work MUST
follow.

## TypeScript Ink reference: dialogs vs inline status

`src/tui/components/AgentView.tsx` uses **two distinct channels**:

### Channel 1 — Inline conversation entries (USED by RPC-078)

Transient feedback (UserNotification, API errors, ⚠ Interrupted,
reconnection status, role/debug/compaction messages, command failures)
is pushed into the `conversation` state array as:

```ts
{ type: 'status', content: '...' }
```

`conversationUtils.ts` line 31 maps `'status'` → `'tool'` role for
rendering (white tool-output style). The Rust port must do the same:
these chunks are *scrollback lines*, not popups.

### Channel 2 — Modal dialogs (NOT used by RPC-078)

`AgentView.tsx` mounts exactly ONE modal dialog: `<ErrorDialog ... />`
for **fatal stream-level API errors** and **data-directory init
failures**. Even then, the same `API Error: ...` message also lands in
Channel 1 so the scrollback retains the error after the modal closes.

RPC-078 does not change Channel 2. The modal stays out of scope.

## Rust dialog system inventory (for future reference)

`codelet/fspec-tui/src/components/dialog_theme.rs` is the canonical
overlay renderer. Every dialog under `codelet/fspec-tui/src/components/`
already delegates to `render_dialog(area, buf, &FspecDialog { ... })`:

| Rust dialog                  | Accent  | Purpose                          |
|------------------------------|---------|----------------------------------|
| `disconnect_dialog.rs`       | Red     | WebSocket disconnect modal       |
| `pause_dialog.rs`            | Yellow  | HITL pause prompt                |
| `help_dialog.rs`             | Cyan    | `?` help overlay                 |
| `hitl_dialog.rs`             | Cyan    | `request_user_input` prompt      |
| `role_dialog.rs`             | Cyan    | `/role` picker                   |
| `create_session_dialog.rs`   | Cyan    | `/new` session form              |
| `thinking_level_dialog.rs`   | Yellow  | `/thinking off|low|med|high`     |
| `model_selector_dialog.rs`   | Cyan    | `/model` picker                  |

### Conventions every Rust dialog MUST follow

1. Build a `FspecDialog { accent, title, rows: Vec<DialogRow>, footer, min_width }` and call `render_dialog(area, buf, &dialog)`. **Never** hand-render with `Block`/`Paragraph` directly.
2. Use `Accent::Red` for errors/destructive, `Accent::Yellow` for warnings/medium-risk, `Accent::Cyan` for info/safe/neutral.
3. Reuse the constants from `dialog_theme.rs`: `MARKER_SELECTED` (`▸ `), `MARKER_UNSELECTED` (`  `), `FOOTER_SEPARATOR` (` │ `).
4. Footer is a single dim, centered string built from `key Action` chunks joined by `FOOTER_SEPARATOR`.
5. Set `Priority::Critical` on `Component::priority()` so the modal captures input before any background view.

## Gap (NOT in RPC-078 scope, follow-up work unit only)

The Rust side is missing three reusable wrappers that the TS side has:

| TS component         | Rust equivalent      | Status   |
|----------------------|----------------------|----------|
| `ErrorDialog.tsx`    | (none — generic)     | MISSING  |
| `NotificationDialog` | (none — generic)     | MISSING  |
| `StatusDialog.tsx`   | (none — generic)     | MISSING  |

If any future work needs to mount an error/notification/progress modal,
add the missing wrappers as a **separate work unit** first
(e.g. `error_dialog.rs`, `notification_dialog.rs`,
`status_dialog.rs` — each delegating to `render_dialog`). Then have
every consumer use the wrapper. Never inline raw `FspecDialog` builds
for these three semantics.

## Conclusion for RPC-078

- **No dialogs created.** ✅
- **No dialogs modified.** ✅
- **Inline-status rendering only.** ✅
- All UI changes live in `chunk_to_lines.rs` + `scrollback.rs` +
  `dispatch_rpc020.rs` (delete-only).
