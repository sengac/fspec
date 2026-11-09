@done
@BUG-449
@tools
@security
@rust
Feature: Stage write permissions are enforced by every file-writing tool
  """
  BLOCK-006 stage gating was not enforced: nothing in production called
  init_stage_permissions (the NAPI front door that did is gone), so the
  global matcher was empty and allowed every write; the base Write, Edit,
  ApplyPatch and AstGrepRefactor tools never consulted the stage (only the
  file facade wrappers did); a failed stage lookup read as "no stage"; and
  config load errors were logged and ignored. tool026's NONE scenario was
  only blocked by the live RLCD engine scoring near its threshold.
  Now codelet_tools::stage_gate::check_session_write(session, tool, path):
  the session callback returns Result<Option<SessionStage{stage,
  project_root}>, String>; the project's stage-permissions config is loaded
  (load errors refuse) and the path is matched relative to the project root;
  a failed lookup, or no callback in a process that did not declare
  unrestricted tool access (BUG-447), refuses. Every file-writing tool and
  the facade wrappers call it; the global matcher fallback is gone.
  Tests: rust/sessions/tests/bug449_stage_write_permissions_are_enforced.rs
  (RLCD disabled via the isolated user directory).
  """

  Background: User Story
    As a user relying on ACDD stage permissions
    I want to have every file-writing tool refuse writes the work unit's stage does not allow
    So that an implementation file cannot be written while the card is still being specified, regardless of chance

  Scenario: Writing and editing an implementation file is refused while the card is specifying
    Given a session whose work unit is in the specifying stage
    When the Write and Edit tools target src/production.rs in the session's project
    Then both are refused naming the specifying stage and the file is unchanged

  Scenario: A file the stage allows is written
    Given a session whose work unit is in the specifying stage
    When the Write tool targets spec/features/login.feature
    Then the file is written

  Scenario: A failed stage lookup refuses the write
    Given a session the registered manager does not know
    When the Write tool targets src/production.rs
    Then the write is refused because the session's stage cannot be looked up
