@done
@high
@tui
@checkpoint
@TUI-005
Feature: Checkpoint Viewer loads actual checkpoint data
  """
  Uses isomorphic-git to load checkpoints from .git/fspec-checkpoints-index/, git.listFiles() to enumerate checkpoint files, and getCheckpointFileDiff() to compare versions. Worker threads prevent UI blocking during diff operations.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Checkpoint list must load from .git/fspec-checkpoints-index/ directory
  #   2. File list must be loaded using git.listFiles() with checkpoint OID
  #   3. Diff pane must compare checkpoint file version with HEAD version
  #   4. Top-left pane shows only checkpoints, bottom-left pane shows only files, right pane shows only diff
  #
  # EXAMPLES:
  #   1. User opens checkpoint viewer (C key), sees list of 3 checkpoints sorted by timestamp
  #   2. User selects checkpoint 'baseline', file list shows ['src/auth.ts', 'src/login.ts'] from that checkpoint
  #   3. User selects 'src/auth.ts' from checkpoint, diff pane shows changes between checkpoint version and HEAD
  #
  # ========================================
  Background: User Story
    As a developer viewing checkpoint data
    I want to see actual checkpoints with their files and diffs
    So that I can inspect real checkpoint state instead of stub data

  Scenario: Display checkpoints from git checkpoint system
    Given I have 3 checkpoints in .git/fspec-checkpoints-index/
    When I open the checkpoint viewer with C key
    Then I should see all 3 checkpoints sorted by timestamp

  Scenario: Display files from selected checkpoint
    Given I have selected checkpoint 'baseline'
    When The checkpoint viewer loads files using git.listFiles()
    Then I should see all files from that checkpoint in the bottom-left pane

  Scenario: Display diff between checkpoint and HEAD
    Given I have selected file 'src/auth.ts' from checkpoint 'baseline'
    When The diff pane loads using getCheckpointFileDiff()
    Then I should see the diff comparing checkpoint version to HEAD version
