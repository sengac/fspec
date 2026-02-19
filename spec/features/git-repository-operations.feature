@done
@rust
@napi
@git
@infrastructure
@GIT-013
Feature: Git Repository Operations
  """
  Implements codelet/git Rust crate using gitoxide (gix) - a pure Rust git implementation. Exposes NAPI-RS bindings to TypeScript for status, diff, and branch operations. Uses gix::Repository for all git operations. No external git binary required.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. All existing git status operations must maintain the same TypeScript API surface
  #   2. Git operations must be exposed via NAPI-RS bindings from Rust
  #   3. isomorphic-git dependency must be completely removed after migration
  #   4. Git status operations must use gix::Repository::status() API
  #   5. Diff operations must use gix::diff blob comparison APIs
  #   6. Binary file detection via gitoxide blob attributes
  #
  # EXAMPLES:
  #   1. Querying staged files returns all files added to the index (git add) with their paths
  #   2. Querying unstaged files returns modified files not yet added to the index
  #   3. Querying untracked files returns new files not yet added to git
  #   4. Requesting a file diff returns unified diff format showing added/removed lines
  #   5. Binary files are detected and excluded from text diff output
  #   6. Querying current branch returns the active branch name or detached HEAD state
  #   7. TypeScript API maintains same function signatures as isomorphic-git version
  #
  # QUESTIONS (ANSWERED):
  #   Q: OpenAI Codex uses Ghost Commits (detached commits) instead of worktrees. Should we adopt this simpler approach for undo/restore, or proceed with the gitoxide migration as planned?
  #   A: Hybrid approach: Use git CLI commands (like Codex) with timeouts for all operations. Ghost commits replace stash-based checkpoints for simpler undo/restore. Worktrees remain for multi-session isolation. This gives us the best of both approaches.
  #
  #   Q: Codex uses git CLI commands with timeouts, not a git library. Should we use CLI fallback for complex operations (like stash creation) while using gitoxide for simpler ops (status, diff)?
  #   A: Yes, use git CLI for all operations. Codex uses CLI with 5-second timeouts and it works well. This avoids library compatibility issues and ensures native git behavior.
  #
  # ========================================
  Background: User Story
    As a fspec consumer
    I want to use git operations via TypeScript API backed by gitoxide
    So that get native performance and enable advanced git features like worktrees and ghost commits

  Scenario: Query staged files from repository
    Given a git repository with files staged using git add
    When I call getStagedFiles()
    Then I receive a list of file paths that are staged for commit

  Scenario: Query unstaged files from repository
    Given a git repository with modified files not yet staged
    When I call getUnstagedFiles()
    Then I receive a list of modified files not yet added to the index

  Scenario: Query untracked files from repository
    Given a git repository with new files not yet added to git
    When I call getUntrackedFiles()
    Then I receive a list of new files not tracked by git

  Scenario: Request unified diff for changed file
    Given a git repository with a modified file
    When I call getFileDiff() for that file
    Then I receive unified diff format output showing added and removed lines

  Scenario: Detect and handle binary files in diff
    Given a git repository with a modified binary file
    When I call getFileDiff() for the binary file
    Then the file is identified as binary and excluded from text diff output

  Scenario: Query current branch name
    Given a git repository checked out to a branch
    When I call getCurrentBranch()
    Then I receive the active branch name or detached HEAD state

  Scenario: Maintain TypeScript API compatibility
    Given the existing isomorphic-git based TypeScript API
    When the gitoxide implementation is substituted
    Then all existing function signatures remain unchanged
    And all existing consumers continue to work without modification
