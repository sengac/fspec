@done
@WT-002
@bug-fix
@session
@tui
Feature: fspec binary runs the footer poller so the session footer reflects git state
  """
  The fspec binary (build_service in rust/fspec/src/common.rs) owns a NON-singleton
  SessionManager; chunks reach the TUI only via THAT manager's chunks_tx. The
  NAPI-free poller (codelet-sessions) must resolve its sender via a
  process-global manager-sender slot that each hook impl registers before
  spawning; defaulting to SessionManager::instance() when unregistered preserves
  today's NAPI behavior.

  Architecture:
  - The shared poller lives in codelet-sessions (footer_poller module) — NAPI-free.
  - It resolves the chunks_tx sender via a process-global manager-sender slot
    (codelet-sessions::footer_poller::register_chunk_sender), registered by
    FspecAgentHooks in build_service (non-singleton manager) and by the NAPI
    install path (SessionManager::instance()); when unregistered it falls back
    to SessionManager::instance() (today's NAPI behavior).
  - FspecAgentHooks::spawn_footer_poller / stop_footer_poller call the shared
  poller; NapiSessionManagerHooks keeps delegating to crate::footer_poller,
  which is now a thin shim over the shared poller.
  - git repo detection: codelet-git get_current_branch returns
  Ok(Some(branch)) for symbolic HEAD, Ok(None) for detached HEAD inside a
  repo, Err(_) outside a repo → is_git_repo = Ok result, branch = value.
  - TUI: FooterStateUpdate with is_git_repo=true + branch=None renders the
  worktree CWD with a " (detached)" suffix in the footer right line.
  """

  Background: User Story
    As a developer running the fspec binary
    I want to create an isolated (worktree) session and see its git state in the session footer
    So that the footer reflects the worktree's branch/detached state instead of staying blank for the session's whole life

  Scenario: A session created under the fspec binary's hooks receives a footer state update on the first poll tick
    Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    And a new session has been created in a git repository that has a branch checked out
    When the footer poller's first tick runs
    Then a FooterStateUpdate chunk is emitted on that manager's chunks broadcast for the session
    And the chunk carries the session's working directory and the checked-out branch name

  Scenario: A session in a git repository shows its branch in the footer without any Bash command first
    Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    And a new session has been created in a git repository that has a branch checked out
    When I observe the footer state for that session
    Then the footer state reports the working directory and the branch name
    And no Bash tool invocation is required for the branch to appear

  Scenario: An isolated worktree session reports a detached git state, not a blank non-repo state
    Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    And a new isolated session whose worktree is in a detached HEAD state
    When the footer poller ticks for that session
    Then the emitted footer state has is_git_repo true with an empty (None) branch
    And the chunk carries the worktree path as the CWD

  Scenario: The footer follows the session's effective CWD when the Bash tool moves the session to another directory
    Given a session manager configured with the fspec binary's hook implementation (FspecAgentHooks)
    And a new session has been created whose footer CWD has been seeded to the project root
    When the session's footer CWD registry is updated to a subdirectory
    And the footer poller ticks again
    Then the emitted footer state carries the subdirectory as the CWD

  Scenario: The NAPI hook path delegates to the same shared footer poller implementation
    Given the NAPI session hooks are installed on a session manager
    When a session is created under the NAPI hook implementation
    Then the footer poller for that session runs the shared NAPI-free poller (codelet-sessions)
    And the NAPI hook still delegates spawn/stop to its crate::footer_poller shim so existing shape tests stay green
