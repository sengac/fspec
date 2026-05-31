# RPC-057 — `/merge-worktree` flow + worktree RPC surface

**Parent:** RPC-030 · **Phase:** 7.4 · **Estimate:** 5 pts · **Depends on:** RPC-056

## Goal

Port the TS `/merge-worktree` flow (`handleMergeWorktree` from `src/tui/services/merge-worktree.ts`, called at `AgentView.tsx` line 2798). New RPC methods backed by `codelet-git` (already NAPI-free). Confirm dialog UI. On merge conflicts, seed input with conflict context (TS equivalent: `pendingAutoSubmitRef.current = true` pattern).

## Backend trait additions

```rust
fn merge_session_worktree(&self, session_id: &SessionId, strategy: MergeStrategy) -> Result<MergeOutcome, String>;
fn discard_session_worktree(&self, session_id: &SessionId) -> Result<(), String>;
fn prune_orphaned_worktrees(&self) -> Result<Vec<String>, String>;
fn list_session_worktrees(&self) -> Vec<SessionWorktreeInfo>;
fn inspect_session_changes(&self, session_id: &SessionId) -> Result<SessionChangesSummary, String>;
```

New wire types:

```rust
pub enum MergeStrategy { FastForward, Squash, ThreeWay }

pub struct MergeOutcome {
    pub status: MergeStatus, // Success | Conflict | NoChanges
    pub conflicts: Vec<String>, // file paths with conflicts
    pub merge_commit: Option<String>,
}

pub enum MergeStatus { Success, Conflict, NoChanges }

pub struct SessionWorktreeInfo {
    pub session_id: SessionId,
    pub worktree_path: String,
    pub base_commit: String,
    pub head_commit: String,
    pub dirty: bool,
}

pub struct SessionChangesSummary {
    pub files_changed: u32,
    pub insertions: u32,
    pub deletions: u32,
    pub commits: Vec<String>, // short SHAs
}
```

## Implementation

Delegate to `codelet-git`:
- `merge_session_worktree` → `codelet_git::merge_worktree(path, strategy)`
- `discard_session_worktree` → `codelet_git::remove_worktree(path)`
- `prune_orphaned_worktrees` → scan + remove
- `inspect_session_changes` → `git diff --stat` over the worktree

## Frontend — confirm dialog + conflict path

```rust
SlashCommandAction::MergeWorktree => {
    let Some(session_id) = self.agent_view_store.current_session().cloned() else {
        self.emit_notice("/merge-worktree: no active session");
        return;
    };
    // First inspect changes
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        match backend.inspect_session_changes(session_id.clone()).await {
            Ok(summary) => {
                let _ = sender.send(Action::OpenMergeConfirmDialog { session_id, summary });
            }
            Err(e) => {
                let _ = sender.send(Action::EmitNotice {
                    session_id,
                    text: format!("[error] /merge-worktree: {e}"),
                });
            }
        }
    });
}

Action::OpenMergeConfirmDialog { session_id, summary } => {
    let dialog = MergeConfirmDialog::new(session_id, summary);
    self.compositor.push(Box::new(dialog));
}

Action::MergeConfirmed { session_id, strategy } => {
    self.compositor.pop_topmost();
    let backend = self.backend.clone();
    let sender = self.dispatch_sender.clone();
    tokio::spawn(async move {
        match backend.merge_session_worktree(session_id.clone(), strategy).await {
            Ok(outcome) if outcome.status == MergeStatus::Conflict => {
                // Seed input with conflict context (TS pendingAutoSubmitRef pattern)
                let conflict_msg = format!(
                    "Merge produced conflicts in these files:\n{}\n\nResolve them and re-attempt.",
                    outcome.conflicts.join("\n")
                );
                let _ = sender.send(Action::SeedPendingInput {
                    session_id,
                    text: conflict_msg,
                });
            }
            Ok(outcome) => {
                let _ = sender.send(Action::EmitNotice {
                    session_id,
                    text: format!("[merge] {:?}: {} files changed", outcome.status, outcome.merge_commit.unwrap_or_default()),
                });
            }
            Err(e) => {
                let _ = sender.send(Action::EmitNotice {
                    session_id,
                    text: format!("[error] /merge-worktree: {e}"),
                });
            }
        }
    });
}
```

`MergeConfirmDialog`: Shows summary, asks "Merge with FastForward / Squash / ThreeWay or Discard?".

## Acceptance criteria

1. All five RPC methods exist on `SessionManagerHandle`, `FspecService`, `FspecBackend`.
2. `codelet/sessions` delegates to `codelet-git`.
3. `/merge-worktree` shows confirm dialog with change summary.
4. Successful merge emits notice with commit SHA.
5. Conflict outcome seeds input with conflict context (parity with TS auto-submit).
6. Discard option works.
7. Integration test in `codelet/fspec-tui/tests/merge_worktree.rs` covers happy + conflict paths against a real git fixture.

## Risks

- `git merge` is destructive on the base branch. Confirm dialog must be explicit.
- Conflict files: parse `git status --porcelain` output reliably.
- The seed-input-on-conflict UX deviates from the typical "modal asks then resolves" pattern. Match TS exactly so users aren't surprised.

## Out of scope

- Conflict resolution UI (TS doesn't have one either — user edits files externally).
