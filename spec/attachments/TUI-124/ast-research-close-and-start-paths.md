# TUI-124 AST research: close and start dialog paths

## `ExitConfirmationDialog::new($$$A)` in rust/fspec-tui/src
```
rust/fspec-tui/src/app/dispatch_menu_zone_c.rs:93:25:ExitConfirmationDialog::new(is_busy)
rust/fspec-tui/src/app/dispatch_esc_cascade.rs:83:22:ExitConfirmationDialog::new(is_busy)

```
## `CreateSessionDialog::new($$$A)` in rust/fspec-tui/src
```
rust/fspec-tui/src/app/dispatch_create_session_dialog.rs:56:13:CreateSessionDialog::new(preselect, context)

```
## `CreateSessionDialog::new_bot($$$A)` in rust/fspec-tui/src
```
rust/fspec-tui/src/app/dispatch_create_session_dialog.rs:85:22:CreateSessionDialog::new_bot(None)

```
## `ConfirmDialog::new($$$A)` in rust/fspec-tui/src
```
rust/fspec-tui/src/views/provider_settings/list_actions.rs:205:32:ConfirmDialog::new(
rust/fspec-tui/src/views/provider_settings/list_actions.rs:234:32:ConfirmDialog::new(
rust/fspec-tui/src/views/provider_settings/list.rs:140:40:ConfirmDialog::new(
rust/fspec-tui/src/views/agent/resume_session_view.rs:243:48:ConfirmDialog::new(
rust/fspec-tui/src/views/agent/confirm_dialog.rs:271:22:ConfirmDialog::new(
rust/fspec-tui/src/views/full_screen_shell.rs:249:22:ConfirmDialog::new(

```
## `backend.destroy_session($$$A)` in rust/fspec-tui/src
```
rust/fspec-tui/src/app/merge_worktree_success.rs:54:25:backend.destroy_session(session_id)
rust/fspec-tui/src/app/dispatch_agent_exit.rs:133:33:backend.destroy_session(session_for_destroy)

```
## `self.chain_of_command.cleanup_subordinate($$$A)` in rust/sessions/src
```
rust/sessions/src/session_manager/manager_impl_6.rs:28:9:self.chain_of_command.cleanup_subordinate(uuid)

```
## `session_manager.add_supervisor($$$A)` in rust/agent-loop/src
```
rust/agent-loop/src/agent_manager_handler.rs:198:21:session_manager.add_supervisor(subordinate_id, spawner_id)

```

## Findings
- Close paths that mount the close dialog: the agent bar's Zone C button (dispatch_menu_zone_c.rs) and Esc cascade level 7 (dispatch_esc_cascade.rs). Both now gather close facts first.
- Backend close: SessionManager::destroy_session (sessions/src/session_manager/manager_impl_6.rs) interrupts the turn, keeps the saved manifest (resumable with /resume), marks an isolated session's git-session manifest terminated (worktree prunable by /worktrees prune) and only drops ChainOfCommand links: it never closes subordinates.
- Subordinates are registered after their session is created (agent_manager_handler.rs add_supervisor), so the TUI's cached supervisors can lag; the close facts are fetched at close time (get_subordinates / get_supervisors).
- Start paths: CreateSessionDialog::new (New Agent, '.', Shift+Right, Enter on a card, mux, /isolation) and ::new_bot (New Bot).
- Delete path: ResumeSessionView D -> ConfirmDialog -> persistence_delete_session (deletes the saved manifest).
- merge_worktree_success.rs also destroys a session after /merge-worktree (its own MergeConfirmDialog).
