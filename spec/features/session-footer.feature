@TUI-091
Feature: SessionFooter component with CWD and git branch name display
  """
  Rust footer poller: ONLY call get_current_branch (reads .git/HEAD). Remove
  get_staged_files, get_unstaged_files, get_untracked_files calls entirely.
  The FooterStateUpdate chunk drops dirty/untracked fields. TypeScript
  SessionFooter simplifies to just [⎇ branch-name] with no indicators.

  CWD is DYNAMIC — it tracks the actual directory of the last Bash command
  executed in each session. Uses SessionRegistry<String> in codelet_tools,
  written by BashTool after resolve_cwd(), read by the footer poller each tick.
  Each session has its own registry entry — fully isolated.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. SessionFooter is 1 line high with dark grey background (#333333), matching SessionHeader
  #   2. CWD display shortens HOME to ~ (e.g. /Users/rquast/projects/fspec → ~/projects/fspec)
  #   3. Git info displays ONLY the branch name — format: [⎇ branch-name]. NO dirty (*) or untracked (%) indicators.
  #   4. Branch name must be obtained by reading .git/HEAD (a single file read) — NOT by running full git status
  #   5. The footer poller must NOT call get_staged_files, get_unstaged_files, or get_untracked_files
  #   6. When not in a git repository, only show CWD without branch info
  #   7. Content displayed right-aligned in the footer bar
  #   8. For isolated sessions, use sessionGetEffectiveCwd to show worktree path instead of project root
  #   9. Footer CWD must dynamically update to show the actual working directory of the last Bash command
  #  10. Each background session tracks its own CWD independently
  #  11. When a Bash tool call uses the cwd parameter, the footer updates to show that directory
  #  12. The git branch in the footer must also update to reflect the branch of the new CWD
  #  13. For isolated sessions, initial CWD is the worktree path but still updates with Bash cwd changes
  #
  # EXAMPLES:
  #   1. Git repo on main branch shows: ~/projects/fspec [⎇ main]
  #   2. Git repo on feature branch shows: ~/projects/fspec [⎇ feature-branch] — no indicators regardless of file state
  #   3. Non-git directory shows only CWD: ~/my-project (no branch info)
  #   4. Detached HEAD state shows: ~/projects/fspec [⎇ (detached)]
  #   5. CPU usage from footer polling is near zero — only a single .git/HEAD file read per poll
  #   6. Session A runs Bash(cwd=/tmp) — Session A footer shows /tmp, Session B unchanged
  #   7. Agent runs Bash(cwd=/Users/rquast/other-repo on develop) — footer shows ~/other-repo [⎇ develop]
  #   8. Bash(cwd=/tmp) then Bash(no cwd) — footer returns to project root
  #
  # ========================================
  Background: User Story
    As a user
    I want to see the current working directory and git branch in the session footer
    So that I know where commands are running and which branch I'm on without running git commands

  @component:session-footer
  Scenario: Display CWD and branch name in git repository
    Given I am in a git repository at "~/projects/fspec"
    And the current branch is "main"
    When the SessionFooter renders
    Then I should see "~/projects/fspec" in the footer
    And I should see "[⎇ main]" in the footer
    And the footer should have a dark grey background

  @component:session-footer
  Scenario: Display branch name without dirty or untracked indicators
    Given I am in a git repository at "~/projects/fspec"
    And the current branch is "feature-branch"
    And there are unstaged changes
    And there are untracked files
    When the SessionFooter renders
    Then I should see "[⎇ feature-branch]" in the footer
    And the branch indicator should not contain "*" or "%"

  @component:session-footer
  Scenario: Display only CWD for non-git directory
    Given I am in a non-git directory at "~/my-project"
    When the SessionFooter renders
    Then I should see "~/my-project" in the footer
    And I should not see any branch indicator

  @component:session-footer
  Scenario: Footer has dark grey background spanning full width
    Given I am in a git repository
    When the SessionFooter renders
    Then the footer should be 1 line high
    And the footer should have background color "#333333"
    And the footer should span the full terminal width

  @component:session-footer
  Scenario: Display detached HEAD state
    Given I am in a git repository at "~/projects/fspec"
    And the HEAD is detached
    When the SessionFooter renders
    Then I should see "[⎇ (detached)]" in the footer

  @component:session-footer
  Scenario: Shorten HOME directory to tilde in CWD display
    Given the HOME directory is "/Users/rquast"
    And I am in a directory at "/Users/rquast/projects/fspec"
    When the SessionFooter renders
    Then I should see "~/projects/fspec" in the footer
    And I should not see "/Users/rquast" in the footer

  @component:session-footer
  @performance
  Scenario: Footer poller uses near-zero CPU
    Given the footer poller is running
    When it polls for git information
    Then it should only read the branch name via get_current_branch
    And it should not call get_staged_files
    And it should not call get_unstaged_files
    And it should not call get_untracked_files

  @component:session-footer
  Scenario: Footer CWD updates when Bash tool uses explicit cwd parameter
    Given a session is started at "~/projects/fspec" on branch "main"
    And the footer shows "~/projects/fspec [⎇ main]"
    When the Bash tool executes a command with cwd "/tmp"
    Then the footer should update to show "/tmp"
    And the git branch indicator should disappear since "/tmp" is not a git repository

  @component:session-footer
  Scenario: Footer git branch updates when CWD changes to a different repository
    Given a session is started at "~/projects/fspec" on branch "main"
    When the Bash tool executes a command with cwd pointing to another git repository on branch "develop"
    Then the footer should show the new repository path with "[⎇ develop]"
    And the branch was resolved by reading .git/HEAD in the new cwd not the original session path

  @component:session-footer
  Scenario: Each session tracks CWD and git branch independently
    Given Session A is started at "~/projects/fspec" on branch "main"
    And Session B is started at "~/projects/fspec" on branch "main"
    When Session A runs a Bash command with cwd "/tmp"
    Then Session A footer shows "/tmp" with no git branch
    And Session B footer still shows "~/projects/fspec [⎇ main]" unchanged

  @component:session-footer
  Scenario: Footer CWD returns to session default when Bash runs without explicit cwd
    Given a session is started at "~/projects/fspec" on branch "main"
    And the Bash tool previously ran with cwd "/tmp"
    And the footer currently shows "/tmp"
    When the Bash tool executes a command with no cwd parameter
    Then the footer should show "~/projects/fspec [⎇ main]"
