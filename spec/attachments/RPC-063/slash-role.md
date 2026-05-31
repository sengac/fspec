# RPC-063 — `/role` slash command end-to-end (UI dialog)

**Parent:** RPC-030 · **Phase:** 6.4 · **Estimate:** 2 pts · **Depends on:** RPC-062

## Goal

Wire `/role` to a proper RoleDialog (text input). Currently the bare `/role` slash command in `dispatch_rpc020.rs::handle_slash_command` clears the role (calls `set_session_role(sid, None)`). That matches the submit-line parser's "clear" behaviour, but the dialog should appear for editing.

## TS reference

`AgentView.tsx` line 2743: `setShowRoleDialog(true)` when session exists, status notice otherwise.

## Trait wiring (already present)

- `FspecBackend::get_session_role(SessionId) -> Result<Option<String>>`
- `FspecBackend::set_session_role(SessionId, Option<String>) -> Result<()>`

## Work

### Step 1 — RoleDialog component

`codelet/fspec-tui/src/components/dialogs/role_dialog.rs`:

```
┌── Role ─────────────────────────────────────┐
│                                             │
│ Current role: <existing or "(none)">        │
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │ <textarea for editing>                  │ │
│ │                                         │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│  [Enter] save  [Ctrl+D] clear  [Esc] cancel │
└─────────────────────────────────────────────┘
```

State: `current: String`, `draft: String`, `original: Option<String>`. On mount, loads `original` from `backend.get_session_role(sid)`.

### Step 2 — Dispatcher change

Replace the existing `Role` handler in `dispatch_rpc020.rs::handle_slash_command`:

```rust
SlashCommandAction::Role => {
    let Some(session_id) = self.agent_view_store.current_session().cloned() else {
        self.emit_notice("/role: no active session");
        return;
    };
    // Fetch existing role, then open dialog.
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        let existing = backend.get_session_role(session_id.clone()).await.unwrap_or(None);
        let _ = sender.send(Action::OpenRoleDialog { session_id, existing });
    });
}

Action::OpenRoleDialog { session_id, existing } => {
    let dialog = RoleDialog::new(session_id, existing);
    self.compositor.push(Box::new(dialog));
}

Action::RoleSaved { session_id, role } => {
    self.compositor.pop_topmost();
    let backend = self.backend.clone();
    tokio::spawn(async move {
        let _ = backend.set_session_role(session_id, role).await;
    });
}

Action::RoleCleared { session_id } => {
    self.compositor.pop_topmost();
    let backend = self.backend.clone();
    tokio::spawn(async move {
        let _ = backend.set_session_role(session_id, None).await;
    });
}
```

### Step 3 — Submit-line `/role` direct path

The submit-line parser (`slash_parser.rs::parse_slash_command`) currently has a `/role <text>` direct-set path. Keep it for power users:

```rust
if let Some(rest) = trimmed.strip_prefix("/role ") {
    let role = rest.trim().to_string();
    if role.is_empty() {
        return ParsedSlash::ClearRole;
    }
    return ParsedSlash::SetRole(role);
}
if trimmed == "/role" {
    // Palette-equivalent: open dialog (via SlashCommandSelected(Role))
    return ParsedSlash::OpenRoleDialog;
}
```

## Acceptance criteria

1. `/role` (from palette OR submit-line) opens `RoleDialog` seeded with current role.
2. `/role <text>` from submit-line sets the role directly.
3. RoleDialog Enter saves, Ctrl+D clears, Esc cancels.
4. SessionHeader (or appropriate widget) shows the role badge when set.
5. Integration test in `codelet/fspec-tui/tests/role_dialog.rs`.

## Out of scope

- Multi-line role editing (single line is fine for now; TS uses textarea but content rarely exceeds one line).
