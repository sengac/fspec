@done
@BUG-447
@tools
@security
@rust
Feature: Tool isolation fails closed without an isolation callback
  """
  codelet_tools::facade::get_isolation_context returned Ok(None) (no path
  restriction) when GET_EFFECTIVE_CWD_CALLBACK was never registered, so any
  process hosting tools without that wiring granted unrestricted file access
  silently. Decision (user rule: no fallbacks; card option a): without a
  callback, a lookup for a session is an error unless the process explicitly
  called declare_unrestricted_tool_access(reason). The fspec binary's
  build_service registers the callback; test binaries that drive tools
  without a session manager declare unrestricted access at the start of the
  tests that need it. Uuid::nil() stays the explicit no-session id.
  Tests: rust/tools/tests/bug447_tool_isolation_fails_closed.rs (a binary
  that never declares or registers).
  """

  Background: User Story
    As a user running sessions in an isolated worktree
    I want to have tool file access refused in any process that has not wired session isolation
    So that a missing registration can never silently grant access outside the worktree

  Scenario: A session's isolation lookup fails without a callback or declaration
    Given a process with no isolation callback and no unrestricted-access declaration
    When the isolation context of a session is looked up
    Then the lookup fails saying no isolation callback is registered

  Scenario: The explicit no-session id stays unrestricted
    Given a process with no isolation callback and no unrestricted-access declaration
    When the isolation context of the nil session id is looked up
    Then no isolation applies
