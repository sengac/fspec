@done
@session
@system-reminders
@bug-fix
@rust
@BUG-581
Feature: The agent's environment reminder names the daemon's working directory
  """
  The environment reminder takes its working directory from the session, never std::env::current_dir. codelet-cli: gather_environment_info and gather_environment_info_with_isolation take the working directory; Session::inject_context_reminders and inject_context_reminders_with_isolation take it and pass it on. codelet-sessions: BackgroundSession::working_directory() is the worktree of an isolated session, else the project; creation (session_creation_helper), clear_history, the isolation-off path (handle_impl_9) and the focus reminder all use it. The focus reminder (activity_focus.rs apply_reminder) used base_environment_content, gathered once at construction from the daemon cwd with no isolation lines; it now gathers the session's current environment (working directory and isolation context) each time it rewrites the reminder, and the cached field is gone.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. The environment reminder's 'Working directory' is the session's working directory: the worktree of an isolated session, the session's project otherwise; the daemon process's working directory is never read
  #   2. Every environment reminder a session gets says the same thing: at creation, after clearing history, after isolation is turned off, and the reminder rewritten when the session's work unit focus changes
  #   3. The focus reminder keeps the isolation lines (Isolation, Worktree, Base commit) of an isolated session
  #
  # EXAMPLES:
  #   1. The daemon runs in D; a session is created over project P; its environment reminder says 'Working directory: P' and never D
  #   2. An isolated session over P with worktree W clears its history; the environment reminder says 'Working directory: W' and 'Isolation: ACTIVE'
  #   3. An isolated session over P with worktree W gets a work unit focus; the rewritten environment reminder says 'Working directory: W', 'Isolation: ACTIVE' and 'Current work unit: <id>'
  #
  # ========================================
  Background: User Story
    As a agent working in a session
    I want to be told my session's real working directory in the environment reminder
    So that paths I build from it point into my session's checkout, the worktree when I am isolated, not the directory the daemon started in

  Scenario: A new session's reminder names its project, not the daemon's directory
    Given the daemon's working directory is D
    When a session is created over project P
    Then its environment reminder says 'Working directory: P'
    And the reminder does not name D

  Scenario: An isolated session's reminder names its worktree after clearing history
    Given the daemon's working directory is D
    And an isolated session over project P with worktree W
    When the session clears its history
    Then its environment reminder says 'Working directory: W' and 'Isolation: ACTIVE'
    And the reminder does not name D

  Scenario: A focus change keeps the isolated session's working directory and isolation lines
    Given the daemon's working directory is D
    And an isolated session over project P with worktree W
    When the session's work unit focus is set to AUTH-001
    Then its environment reminder says 'Working directory: W', 'Isolation: ACTIVE' and 'Current work unit: AUTH-001'
    And the reminder does not name D
