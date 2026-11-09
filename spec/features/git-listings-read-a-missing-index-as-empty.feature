@done
@BUG-637
@git
@git-ops
@rust
@bug-fix
@operations
Feature: Git listings read a missing index as empty
  """
  A fresh git init has no .git/index until the first git add. BUG-592 made the untracked walk read a missing index as empty (gix index_or_empty), but codelet-git's staged listing (status.rs staged_files_with), unstaged listing (status.rs unstaged_files_with_repo), the unstaged change-type pass (change_type.rs) and StagedFiles::open (index_blob.rs, used by fspec audit --changed) still called repo.index(), which fails when the file is missing, so the changed-files capture and fspec audit failed in a fresh repository. They now call index_or_empty too. Only a missing index reads as empty: an index file that can't be parsed is still an exact error. The worktree snapshot (tree_utils.rs) keeps its CorruptedIndex error, since fspec worktrees always have an index.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A working tree with no index file reads as an empty index for the staged, unstaged and changed-files listings and for StagedFiles, like the untracked walk (BUG-592)
  #   2. An index file that exists but cannot be read is still an exact error; only a missing index reads as empty
  #   3. The worktree snapshot (tree_utils.rs) keeps its CorruptedIndex error: fspec worktrees always have an index
  #
  # ========================================
  Background: User Story
    As a developer running fspec in a repository with no commits and nothing staged
    I want git listings and fspec audit to work before the first git add
    So that a fresh git init isn't reported as a git failure

  Scenario: Listings in a fresh git init with no index
    Given a fresh git init holding a.txt and no .git/index
    When the staged, unstaged and changed-files listings are read
    Then the staged and unstaged listings are empty
    And the changed-files capture lists only a.txt, untracked, as "A"

  Scenario: StagedFiles opens in a fresh git init with no index
    Given a fresh git init holding a.txt and no .git/index
    When StagedFiles is opened
    Then it succeeds and holds no staged blob for a.txt

  Scenario: An unreadable index is still an error
    Given a repository whose .git/index holds bytes that are not an index
    When the staged listing is read
    Then it fails with an error
