@done
@session
@tool-execution
@bug-fix
@rust
@WT-016
Feature: The Fspec tool resolves its project root from the daemon's working directory
  """
  create_handler (rust/agent-loop/src/fspec_handler.rs) resolves the request's project_root before anything else: a relative root joins the session's working directory (BackgroundSession worktree_path for an isolated session, else session.project); in an isolated session the result goes through codelet_tools validate_and_resolve_path_with_isolation with the session's worktree and main checkout, so a root in the main checkout is refused with the file tools' blocked-path error before the command runs. The resolved absolute root is used for the in-process dispatch_command, the forwarded FspecRequest of a callback front end, and the focus-reconciliation comparison. Before this fix the raw string went to dispatch_command, whose relative '.' resolved against the daemon process's cwd. Reaching shared state (board, registries) at the main checkout from a worktree is WT-015.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. A relative project_root, including the default '.', is joined to the session's working directory: the worktree of an isolated session, the session's project otherwise; the daemon's working directory never takes part
  #   2. In an isolated session, a project_root inside the main checkout is refused with the file tools' 'blocked from original project' error and the command does not run
  #   3. The resolved absolute root is the one the command runs against, the one forwarded to a callback front end, and the one focus reconciliation compares with the session's project
  #   4. Shared spec state (the board and other registries) reaching the main checkout from a worktree is WT-015's state-root resolver; until it lands an isolated session's Fspec tool acts on the worktree's checkout, exactly as the fspec CLI does in that worktree
  #
  # EXAMPLES:
  #   1. The daemon runs in directory D whose board holds DAEMON-001; an isolated session's worktree board holds TREE-001; the Fspec tool runs list-work-units with project_root '.'; the result lists TREE-001 and not DAEMON-001
  #   2. A non-isolated session's project board holds PROJ-001 while the daemon runs in D; list-work-units with the default project root lists PROJ-001 and not DAEMON-001
  #   3. An isolated session passes the main checkout's absolute path as project_root to create-story; the tool fails with 'blocked from original project' and the main board gains no card
  #
  # ========================================
  Background: User Story
    As a agent calling the Fspec tool in a session
    I want to have a relative project root resolve against my session's working directory, the worktree when I am isolated
    So that my feature, coverage and board commands act on my session's checkout instead of whatever directory the daemon started in

  Scenario: A relative root in an isolated session resolves to the worktree
    Given the daemon's working directory holds a board with DAEMON-001
    And an isolated session whose worktree holds a board with TREE-001
    When the session's Fspec tool runs list-work-units with project_root '.'
    Then the result lists TREE-001
    And the result does not list DAEMON-001

  Scenario: The default root in a non-isolated session resolves to the session's project
    Given the daemon's working directory holds a board with DAEMON-001
    And a non-isolated session whose project holds a board with PROJ-001
    When the session's Fspec tool runs list-work-units without a project_root
    Then the result lists PROJ-001
    And the result does not list DAEMON-001

  Scenario: An isolated session cannot point the Fspec tool at the main checkout
    Given an isolated session whose project is the main checkout
    When the session's Fspec tool runs create-story with the main checkout's absolute path as project_root
    Then the tool fails with the 'blocked from original project' error
    And the main checkout's board is unchanged
