@TUI-091
Feature: Footer CWD Registry — per-session dynamic working directory tracking
  """
  SessionRegistry<String> in codelet_tools stores the last known CWD for each
  session (keyed by UUID). BashTool writes after resolve_cwd(). The footer poller
  in codelet_napi reads from it each tick. Follows the same pattern as
  BASH_ABORT_FLAGS (BUG-129), TOOL_PROGRESS_CALLBACKS (BUG-126), etc.
  """

  Background: User Story
    As a user
    I want each session to independently track where commands are running
    So that the footer always reflects the actual working directory per session

  @component:session-footer
  Scenario: Registry updates CWD when Bash tool uses explicit cwd
    Given a session CWD entry is initialized to "/Users/rquast/projects/fspec"
    When the Bash tool writes "/tmp" as the new CWD for that session
    Then the registry returns "/tmp" for that session

  @component:session-footer
  Scenario: Registry tracks CWD independently per session
    Given Session A CWD is "/Users/rquast/projects/fspec"
    And Session B CWD is "/Users/rquast/projects/fspec"
    When Session A CWD is updated to "/tmp"
    Then Session A registry returns "/tmp"
    And Session B registry still returns "/Users/rquast/projects/fspec"

  @component:session-footer
  Scenario: Registry CWD returns to session default on no explicit cwd
    Given a session CWD entry was set to "/tmp"
    When the Bash tool writes "/Users/rquast/projects/fspec" back as the CWD
    Then the registry returns "/Users/rquast/projects/fspec"

  @component:session-footer
  Scenario: Cleanup removes CWD entry on session destroy
    Given a session has a CWD entry in the registry
    When the session is destroyed and cleanup is called
    Then the registry entry for that session is removed

  @component:session-footer
  Scenario: Reading CWD for unknown session returns None
    Given no CWD is registered for a session
    When the footer poller reads the CWD for that session
    Then it receives None and falls back to the initial CWD

  @component:session-footer
  Scenario: Initial CWD is seeded at session creation
    Given a new session is created with effective_cwd "/Users/rquast/projects/fspec"
    When the session creation code seeds the registry
    Then the registry value is available immediately before any Bash commands run
