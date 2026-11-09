# WT-019 AST research: where session focus changes

(2026-11-07, HEAD bf18c221b)

- fspec astgrep --pattern 'fn apply_resolved_focus($$$) -> $R { $$$ }' --lang rust --path rust/sessions/src
    No matches found
- fspec astgrep --pattern '$S.apply_resolved_focus($$$)' --lang rust --path rust/sessions/src
    rust/sessions/src/focus_reconciliation.rs:31
    rust/sessions/src/focus_reconciliation.rs:64
    rust/sessions/src/focus_reconciliation.rs:107
    rust/sessions/src/activity_focus.rs:56
    
- fspec astgrep --pattern 'fn clear_isolation(&self) { $$$ }' --lang rust --path rust/sessions/src
    No matches found
- fspec astgrep --pattern 'pub fn worktree_path(&self) -> $R { $$$ }' --lang rust --path rust/sessions/src
    rust/sessions/src/background_session/workspace.rs:31
    

Findings: every focus change (set_session_focus, admit_tool_invocation, reconcile_status_success, reconcile_work_units) calls SessionManager::apply_resolved_focus, so the claim check belongs there. A session is isolated while worktree_path() is Some; detach_session_worktree (merge, discard) calls clear_isolation; destroy_session removes the session from the manager.
