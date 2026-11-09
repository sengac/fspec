@done
@session
@system-reminders
@bug-fix
@rust
@BUG-598
Feature: AGENTS.md is read from the daemon's directory, not the session's
  """
  codelet-cli context_gathering::discover_agents_md takes the start directory (no Option, no std::env::current_dir default). Session::inject_context_reminders_with_isolation passes the working directory it is already given (BUG-581: BackgroundSession::effective_cwd, the worktree of an isolated session, else the project) to it, so creation (session_creation_helper) and clear_history read the session's AGENTS.md. Test: rust/sessions/tests/bug598_agents_md_from_session_directory.rs.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. AGENTS.md is discovered by walking up from the session's working directory (BackgroundSession::effective_cwd: the worktree of an isolated session, the session's project otherwise); the daemon process's working directory is never read
  #   2. Every place a session gets its context reminders uses that directory: creation and clearing history
  #   3. Discovery takes the start directory explicitly; there is no current-directory default
  #
  # EXAMPLES:
  #   1. The daemon runs in D holding AGENTS.md 'daemon rules'; a session over project P holding AGENTS.md 'project rules' gets 'project rules' and never 'daemon rules'
  #   2. An isolated session over P with worktree W holding AGENTS.md 'worktree rules' clears its history and gets 'worktree rules'
  #   3. A session over a project with no AGENTS.md anywhere above it, while D holds one, gets no AGENTS.md reminder
  #
  # ========================================
  Background: User Story
    As a agent working in a session
    I want to get the AGENTS.md of my session's own working directory
    So that I follow the instructions of the project or worktree I work in, not of the directory the daemon started in

  Scenario: A new session reads its project's AGENTS.md, not the daemon's
    Given the daemon's working directory D holds an AGENTS.md saying 'daemon rules'
    And project P holds an AGENTS.md saying 'project rules'
    When a session is created over project P
    Then its context reminders say 'project rules'
    And they never say 'daemon rules'

  Scenario: An isolated session reads its worktree's AGENTS.md after clearing history
    Given the daemon's working directory D holds an AGENTS.md saying 'daemon rules'
    And an isolated session over project P with worktree W holding an AGENTS.md saying 'worktree rules'
    When the session clears its history
    Then its context reminders say 'worktree rules'
    And they never say 'daemon rules'

  Scenario: A project without AGENTS.md never gets the daemon's
    Given the daemon's working directory D holds an AGENTS.md saying 'daemon rules'
    And project P has no AGENTS.md in it or above it
    When a session is created over project P
    Then its context reminders never say 'daemon rules'
