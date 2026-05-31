# RPC-060 — Isolated session creation + AgentView `/new isolated` flow

**Parent:** RPC-030 · **Phase:** 7.7 · **Estimate:** 5 pts · **Depends on:** RPC-059

## Goal

Wire `backend.create_isolated_session(role)` → returns `IsolatedSessionInfo` (wire type from RPC-036). AgentView surfaces "create isolated" as a variant of the new-session flow.

## Backend

Already implemented in RPC-042: `SessionManagerHandle::create_isolated_session(role)` delegates to `SessionManager::create_isolated_session_with_id` (originally `session_manager.rs` line 3542). That method:

1. Generates a fresh UUID.
2. Creates a git worktree via `codelet_git::create_worktree(base_repo, worktree_path)`.
3. Resolves an isolation context via `codelet_cli::session::context_gathering::IsolationContext`.
4. Constructs the session manifest pointing at the worktree.
5. Initialises MCP via `codelet_tools::init_mcp_session`.
6. Broadcasts a metadata-update chunk.

## Frontend — CreateSessionDialog

`AgentViewStore.show_create_session_dialog` (already exists in RPC-012). Extend the dialog UI with a checkbox: **"Isolated worktree"**.

```
┌── New session ──────────────────────────────┐
│ Role: _________________________________     │
│                                             │
│ [ ] Isolated worktree                       │
│                                             │
│         [Create]   [Cancel]                 │
└─────────────────────────────────────────────┘
```

On submit:

```rust
Action::CreateSessionSubmitted { role, isolated } => {
    self.compositor.pop_topmost();
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        let result = if isolated {
            backend.create_isolated_session(role).await
                .map(|info| info.session_id)
        } else {
            backend.create_session(role).await
        };
        match result {
            Ok(session_id) => {
                let _ = sender.send(Action::SessionCreated { session_id });
            }
            Err(e) => {
                let _ = sender.send(Action::EmitGlobalNotice {
                    text: format!("[error] create session: {e}"),
                });
            }
        }
    });
}
```

## Alternative: slash command

If the TS frontend uses a dedicated `/new-isolated` slash command instead of the dialog toggle, mirror that. Audit `src/tui/utils/slashCommands.ts` for any `new-isolated` entry; if not present, the dialog-toggle approach is fine.

## Isolation indicator

When a session is isolated, `SessionFooter` already renders an isolation badge driven by `StreamChunk::IsolationStateChange` (RPC-036 + RPC-045). Make sure the badge shows immediately on creation by broadcasting the chunk inside `create_isolated_session_with_id` (it should already — verify by reading line 3791 of the original `session_manager.rs`).

## Acceptance criteria

1. `create_isolated_session` works via both transports.
2. CreateSessionDialog has an "Isolated worktree" checkbox.
3. Creating an isolated session creates a git worktree on disk.
4. SessionFooter shows isolation badge.
5. The worktree path is queryable via `IsolatedSessionInfo.worktree_path` and `StreamChunk::IsolationStateChange`.
6. Integration test in `codelet/fspec-tui/tests/isolated_session.rs` creates an isolated session in a temp git repo + asserts worktree exists.

## Risks

- Worktree creation can fail in non-git directories. The dialog should disable the checkbox or show a tooltip when `WorkspaceInfo.is_git_repo == false`.
- Concurrent isolated sessions: each gets its own worktree directory under `.fspec/worktrees/<uuid>`. Confirm `create_worktree` is concurrency-safe.

## Out of scope

- Worktree merge (covered by RPC-057).
- Auto-creating isolated sessions per work-unit (future feature).
