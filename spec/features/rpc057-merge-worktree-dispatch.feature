@done
@RPC-057
@rpc
@agent-view
@tui
@slash-command
@rust
@dialog
@git-integration
Feature: /merge-worktree flow + worktree RPC surface
  """
  Phase 7.4 of the RPC-030 roadmap. Reaches TS-parity for the
  /merge-worktree slash command by:

  1. Adding FIVE new RPC methods through the trait, FspecService,
     FspecBackend, and both transports:
       * merge_session_worktree(session_id, strategy) -> Result<MergeOutcome>
       * discard_session_worktree(session_id) -> Result<()>
       * prune_orphaned_worktrees() -> Result<Vec<String>>
       * list_session_worktrees() -> Vec<SessionWorktreeInfo>
       * inspect_session_changes(session_id) -> Result<SessionChangesSummary>
  2. Replacing the `SlashCommandAction::MergeWorktree` notice fallback
     in dispatch_slash_commands.rs with a real `handle_slash_merge_worktree`
     routed through a new app/dispatch_merge_worktree.rs file (mirroring the
     dispatch_blocklist pattern).
  3. Adding a compositor-owned `MergeConfirmDialog` that paints the
     SessionChangesSummary + three buttons (Merge, Discard, Cancel).
  4. On a Conflict response, seeding the per-session input draft with a
     conflict-context message via the existing Action::SeedPendingInput
     plumbing (RPC-052) so the LLM agent can see the conflicting files
     and resolve them — TS parity with the buildConflictLlmContext +
     pendingAutoSubmitRef pattern.

  TS reference: `src/tui/handlers/mergeWorktreeHandler.ts` —
  `handleMergeWorktree(ctx)` calls inspectSessionChanges →
  mergeSessionChanges → on Conflict: injectLlmContext(...). The Rust
  port adds an explicit pre-merge confirm dialog so destructive
  operations are gated behind user input (departure from the TS UX
  that merges immediately).

  Out of scope: a conflict-resolution UI (matches TS — user / agent
  edits files externally); the prune/list RPCs exist on the surface
  but no slash command currently wires them (future card).
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionManagerHandle MUST expose default-impl methods for all five operations so existing handles compile unchanged.
  #   2. StubSessionManagerHandle MUST expose per-call counters and seedable state for cross-transport parity tests.
  #   3. codelet-sessions handle_impl delegates each method to codelet-git (merge_session, discard_session, prune_orphaned, list_worktrees + derive_session_status, inspect_session). repo_path comes from std::env::current_dir().
  #   4. MergeStrategy is on the trait surface for future evolution; the codelet-git layer ignores it (single algorithm). The UI passes MergeStrategy::FastForward.
  #   5. SlashCommandAction::MergeWorktree with no current session is a silent no-op.
  #   6. SlashCommandAction::MergeWorktree spawns backend.inspect_session_changes first; zero-change result emits "[merge] nothing to merge" notice; non-zero result opens MergeConfirmDialog.
  #   7. MergeConfirmDialog has three buttons (Merge, Discard, Cancel); Merge is the default focus; Esc cancels.
  #   8. On Action::MergeConfirmed { session_id }: spawn backend.merge_session_worktree; route Success→notice, NoChanges→notice, Conflict→Action::SeedPendingInput, Err→error notice. Dialog is popped from the compositor BEFORE the round-trip completes.
  #   9. On Action::DiscardConfirmed { session_id }: spawn backend.discard_session_worktree; route Ok→notice, Err→error notice. Dialog is popped before the round-trip.
  #   10. The conflict-context payload format starts with "Merge produced conflicts in the following files:" followed by a bullet list and an "Effective worktree:" footer.
  #
  # ========================================

  Background: User Story
    As a fspec TUI user with an open AgentView session and modified worktree changes
    I want to run /merge-worktree to merge those changes back to main with explicit confirmation
    So that I can complete the session's work cleanly, see a per-file change summary before merging, and recover gracefully when the merge produces conflicts — full TS-Ink parity in the Rust ratatui frontend

  Scenario: /merge-worktree with no current session is a silent no-op
    Given an App with NO open AgentView session
    When SlashCommandSelected(SlashCommandAction::MergeWorktree) is dispatched
    Then no backend method is called
    And no scrollback notice is emitted
    And the compositor contains no merge-confirm-dialog

  Scenario: /merge-worktree on a session with no changes emits "nothing to merge"
    Given an App with open session s-1 wired to a MockBackend whose inspect_session_changes returns zero changes
    When SlashCommandSelected(SlashCommandAction::MergeWorktree) is dispatched
    Then within 1 second backend.inspect_session_changes is called exactly once with session_id "s-1"
    And within 1 second Action::EmitSessionNotice carrying "[merge] nothing to merge" for s-1 is observed on the action bus
    And no merge-confirm-dialog is pushed onto the compositor

  Scenario: /merge-worktree on a session with changes opens the MergeConfirmDialog
    Given an App with open session s-1 wired to a MockBackend whose inspect_session_changes returns SessionChangesSummary { files_changed: 1, insertions: 4, deletions: 2, commits: ["abc1234"] }
    When SlashCommandSelected(SlashCommandAction::MergeWorktree) is dispatched
    Then within 1 second backend.inspect_session_changes is called exactly once with session_id "s-1"
    And within 1 second the compositor contains a layer with id "merge-confirm-dialog"

  Scenario: MergeConfirmDialog renders the change summary
    Given a MergeConfirmDialog seeded with SessionChangesSummary { files_changed: 2, insertions: 10, deletions: 3, commits: ["abc1234", "def5678"] }
    When the dialog is rendered into a 80x24 buffer
    Then the rendered text contains "2 files changed"
    And the rendered text contains "+10"
    And the rendered text contains "-3"
    And the rendered text contains "Merge"
    And the rendered text contains "Discard"
    And the rendered text contains "Cancel"

  Scenario: MergeConfirmDialog opens with Merge focused; Tab cycles forward through buttons
    Given a fresh MergeConfirmDialog
    Then the focused button index equals 0
    When the user presses Tab
    Then the focused button index equals 1
    When the user presses Tab
    Then the focused button index equals 2
    When the user presses Tab
    Then the focused button index equals 0

  Scenario: MergeConfirmDialog handle_key Enter on Merge emits the MergeConfirmed outcome
    Given a fresh MergeConfirmDialog for session s-1 with the Merge button focused
    When the user presses Enter
    Then handle_key returns MergeConfirmDialogOutcome::Merge

  Scenario: MergeConfirmDialog handle_key Enter on Discard emits the DiscardConfirmed outcome
    Given a fresh MergeConfirmDialog for session s-1 with the Discard button focused
    When the user presses Enter
    Then handle_key returns MergeConfirmDialogOutcome::Discard

  Scenario: MergeConfirmDialog handle_key Esc emits the Cancel outcome regardless of focus
    Given a fresh MergeConfirmDialog for session s-1 with the Merge button focused
    When the user presses Esc
    Then handle_key returns MergeConfirmDialogOutcome::Cancel

  Scenario: Action::MergeConfirmed routes through backend.merge_session_worktree and emits a success notice
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    And the backend's merge_session_worktree returns Ok(MergeOutcome { status: Success, conflicts: [], merge_commit: Some("abc1234") })
    When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    Then within 1 second backend.merge_session_worktree is called exactly once with session_id "s-1"
    And within 1 second the compositor no longer contains a layer with id "merge-confirm-dialog"
    And within 1 second Action::EmitSessionNotice for s-1 with text starting with "[merge] success" is observed on the action bus

  Scenario: Action::MergeConfirmed with NoChanges status emits the nothing-to-merge notice
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    And the backend's merge_session_worktree returns Ok(MergeOutcome { status: NoChanges, conflicts: [], merge_commit: None })
    When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[merge] nothing to merge" is observed on the action bus

  Scenario: Action::MergeConfirmed with Conflict status seeds the input with a conflict-context message
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    And the backend's merge_session_worktree returns Ok(MergeOutcome { status: Conflict, conflicts: ["src/a.rs", "src/b.rs"], merge_commit: None })
    When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    Then within 1 second Action::SeedPendingInput for s-1 with text containing "Merge produced conflicts" is observed on the action bus
    And the seeded text contains "src/a.rs"
    And the seeded text contains "src/b.rs"
    And the compositor no longer contains a layer with id "merge-confirm-dialog"

  Scenario: Action::MergeConfirmed with Err emits an error notice and pops the dialog
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    And the backend's merge_session_worktree returns Err("worktree not found")
    When Action::MergeConfirmed { session_id: "s-1" } is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /merge-worktree: worktree not found" is observed on the action bus
    And no Action::SeedPendingInput is observed
    And the compositor no longer contains a layer with id "merge-confirm-dialog"

  Scenario: Action::DiscardConfirmed routes through backend.discard_session_worktree
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    And the backend's discard_session_worktree returns Ok(())
    When Action::DiscardConfirmed { session_id: "s-1" } is dispatched
    Then within 1 second backend.discard_session_worktree is called exactly once with session_id "s-1"
    And within 1 second the compositor no longer contains a layer with id "merge-confirm-dialog"
    And within 1 second Action::EmitSessionNotice for s-1 with text "[discard] worktree discarded" is observed on the action bus

  Scenario: Action::DiscardConfirmed with Err emits an error notice
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    And the backend's discard_session_worktree returns Err("worktree not found")
    When Action::DiscardConfirmed { session_id: "s-1" } is dispatched
    Then within 1 second Action::EmitSessionNotice for s-1 with text "[error] /merge-worktree discard: worktree not found" is observed on the action bus

  Scenario: Action::CancelMergeDialog pops the dialog without firing any backend call
    Given an App with open session s-1 and a MergeConfirmDialog on the compositor
    When Action::CancelMergeDialog is dispatched
    Then the compositor no longer contains a layer with id "merge-confirm-dialog"
    And no backend method is called
    And no scrollback notice is emitted
