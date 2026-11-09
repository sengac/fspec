@wip
@BUG-696
@git
@git-ops
@rust
@bug-fix
@operations
Feature: A truncated git index is an error, not a panic
  """
  gix-index 0.40.1 File::at slices the trailing checksum (data[data.len() - hash_len..]) before it checks the file length, so a .git/index shorter than its checksum panics with subtract-with-overflow (debug and ci-test) or an out-of-range slice. codelet-git now checks the index length before every gix call that reads it (index::ensure_index_not_truncated): the listings in status.rs and change_type.rs, StagedFiles::open (index_blob.rs), collect_worktree_files (tree_utils.rs), git_add and git_commit (repo_ops.rs) and the merge-commit index sync (merge_commit.rs). A file shorter than the 12-byte header plus the object hash is a CorruptedIndex error naming the path and its length. Upstream gix-index main (0.58.0) checks the length itself; the guard can go when fspec upgrades gix.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Every place codelet-git opens .git/index checks its length first: a file shorter than the 12-byte header plus the object hash (32 bytes for SHA-1) is a CorruptedIndex error naming the path and its length, and gix is never called on it
  #   2. A missing index keeps its current meaning at each site (empty for the listings since BUG-637, an error for the worktree snapshot); an index of the minimum length or longer goes to gix unchanged
  #   3. The guard exists because gix-index 0.40.1 File::at slices the checksum before checking the length; upstream gix-index main (0.58.0) checks it, so the guard can go when fspec upgrades gix
  #
  # EXAMPLES:
  #   1. A repository whose .git/index holds 5 bytes: the staged, unstaged and changed-files listings and StagedFiles::open fail with an error naming the index and '5 bytes' instead of panicking
  #   2. A worktree whose .git/index holds 5 bytes: the worktree snapshot fails with a CorruptedIndex error naming the index instead of panicking
  #   3. A .git/index of exactly 32 bytes that is not an index still goes to gix and fails with gix's decode error, not the length error
  #   4. Staging a file (git_add) or committing (git_commit) in a repository whose .git/index holds 5 bytes fails with a CorruptedIndex error naming the index instead of panicking; the merge-commit index sync uses the same check
  #
  # ========================================
  Background: User Story
    As a developer running fspec in a repository whose git index is damaged
    I want a truncated .git/index to be reported as an error naming the index
    So that fspec never crashes on a damaged index

  Scenario: Listings report a truncated index
    Given a repository whose .git/index holds 5 bytes
    When the staged, unstaged and changed-files listings are read and StagedFiles is opened
    Then each fails with an error naming the index and its 5 bytes

  Scenario: The worktree snapshot reports a truncated index
    Given a repository whose .git/index holds 5 bytes
    When the worktree snapshot is collected
    Then it fails with a corrupted index error naming the index

  Scenario: Staging and committing report a truncated index
    Given a repository whose .git/index holds 5 bytes
    When a file is staged and a commit is made
    Then each fails with a corrupted index error naming the index

  Scenario: An index of the minimum length still goes to gix
    Given a repository whose .git/index holds 32 bytes that are not an index
    When the staged listing is read
    Then it fails with gix's decode error, not the length error
