@done
@config-management
@BUG-466
@session
@session-management
@rust
Feature: Session lookups and listings report failures
  """
  Found while doing BUG-465. SessionManagerHandle::get_provider_credential
  returned Option (sessions logged a failed listing and answered None; the
  default answered None) and get_active_session defaulted to None (the RPC
  answered None without a session manager); list_session_worktrees turned a
  failed session diff into dirty=false and base_commit=head; the TUI dropped
  a failed list_sessions (and a failed session delete) and listed every
  project when the current directory could not be resolved; and ten
  FspecBackend trait defaults (profile, continue, goal, default-model and
  thinking-default writes and reads, send_input_with_thinking dropping the
  thinking config) answered for backends that do not implement them.
  Now get_provider_credential returns Result<Option<_>, String> and
  get_active_session Result<Option<SessionId>, String> (trait, sessions,
  stub, tarpc in its RPC-449 position, MCP, both transports); a worktree
  diff failure fails the listing; the TUI shows those failures as an error
  dialog; and the FspecBackend defaults refuse.
  Tests: rust/fspec-tui/tests/bug466_session_lookups_and_listings_report_failures.rs.
  """

  Background: User Story
    As a user browsing sessions, worktrees and provider credentials
    I want to have failed lookups and listings shown instead of answered with nothing
    So that an empty or missing answer always means there is nothing there

  Scenario: A failed provider credential lookup is reported
    Given a providers directory containing an invalid "broken.json"
    When the session manager looks up the "anthropic" provider credential
    Then the lookup fails naming the invalid provider config

  Scenario: The active session lookup is refused without a session manager
    Given an RPC service with no session manager attached
    When the active session is read over the embedded transport
    Then the read fails saying no session manager is attached

  Scenario: A worktree whose changes cannot be read fails the worktree listing
    Given an isolated session worktree containing a file that cannot be read
    When the session worktrees are listed
    Then the listing fails instead of reporting the worktree as clean

  Scenario: A failed session listing is shown when the resume view opens
    Given a backend whose session listing fails with "sessions directory is unreadable"
    When the resume view opens
    Then an error dialog shows "sessions directory is unreadable"

  Scenario: Backend defaults refuse instead of answering
    Given the FspecBackend trait defaults for profile, continue, goal, default-model and thinking-default calls and send_input_with_thinking
    When their default bodies are read
    Then every default returns an error instead of a value
