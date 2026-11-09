@done
@BUG-437
@session
@multi-workspace
@rust
Feature: The start workspace is required and validated
  """
  rust/sessions/src/default_project.rs: default_project() returned the
  configured start workspace or, when none was set, std::env::current_dir()
  and then PathBuf::from("."); set_default_project() kept a non-canonical path
  when canonicalize failed and discarded the OnceLock::set result, so a second
  call was silently ignored. SessionManagerHandle::get_effective_cwd also
  answered an unknown session with the default project (or an empty path on
  the stub and over RPC with no session manager).
  Now default_project() and set_default_project() return Result: no start
  workspace, a canonicalize failure and a conflicting second set are errors.
  get_effective_cwd returns Result<PathBuf, String> through the trait, and
  Result<String, String> over the RPC (MCP and both TUI transports map it).
  Tests: rust/sessions/tests/bug437_start_workspace_is_required.rs.
  """

  Background: User Story
    As a user running sessions in a workspace
    I want to have sessions and workspace operations use the configured start workspace or fail
    So that nothing silently lands in whatever directory the process happens to run in

  Scenario: A manager without a start workspace has no default project
    Given a session manager whose start workspace was never set
    When its default project is requested
    Then the request fails naming the missing start workspace

  Scenario: A start workspace that cannot be canonicalized is refused
    Given a session manager
    When the start workspace is set to a path that does not exist
    Then setting it fails with the canonicalize error

  Scenario: A second, different start workspace is refused
    Given a session manager whose start workspace is set
    When a different start workspace is set
    Then setting it fails naming both workspaces and the first stays in effect

  Scenario: The effective cwd of an unknown session is an error
    Given a session manager with a start workspace
    When the effective cwd of a session it does not know is requested
    Then the request fails instead of returning the start workspace
