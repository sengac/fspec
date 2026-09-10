@done
@wt-006
@tui
@dialog
@slash-command
@bug-fix
@rust
@agent-view
Feature: /merge-worktree with only ignored changes emits nothing to merge
  """
  WT-006 (TUI half). The wire summary gains a `files_ignored` count
  (audit: spec/attachments/WT-006/wt-006.md, defect a). A session
  worktree whose only delta is gitignored artifacts (build output,
  logs) must still report zero changed files, so the /merge-worktree
  pre-flight stays honest: the zero-change rule fires on the tracked
  change set, and ignored-only sessions emit the
  "[merge] nothing to merge" notice without opening the merge
  confirm dialog.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. get_session_diff / inspect_session_changes report gitignored untracked files
  #      in a separate files_ignored list: they never count toward
  #      files_changed/added/deleted, never appear in the unified diff,
  #      and never affect merge conflict detection (merge stays tracked-only),
  #      so "nothing to merge" is honest about ignored artifacts
  #
  # EXAMPLES:
  #   1. A worktree with only build artifacts (a .gitignored target/ dir with
  #      files) → get_session_diff reports files_changed/added/deleted empty AND
  #      files_ignored listing the artifacts; the /merge-worktree inspect reports
  #      0 files changed so the TUI says 'nothing to merge' honestly
  #
  # ========================================
  Background: User Story
    As a fspec TUI user with an open isolated session
    I want /merge-worktree to report "nothing to merge" when the only changes are gitignored build artifacts
    So that I never get a merge dialog (or a fake change count) for artifacts git does not track

  @tui
  @dialog
  Scenario: /merge-worktree with only ignored changes emits "nothing to merge"
    Given an App with open session s-1 wired to a MockBackend whose inspect_session_changes returns SessionChangesSummary { files_changed: 0, insertions: 0, deletions: 0, commits: [], files_ignored: 3 }
    When SlashCommandSelected(SlashCommandAction::MergeWorktree) is dispatched
    Then within 1 second Action::EmitSessionNotice carrying "[merge] nothing to merge" for s-1 is observed on the action bus
    And no merge-confirm-dialog is pushed onto the compositor
