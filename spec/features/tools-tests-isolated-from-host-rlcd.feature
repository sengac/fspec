@BUG-582
@done
@BUG-481
@BUG-347
@tools
@test-maintenance
@rust
Feature: Tools tests are isolated from the host's RLCD service and user config
  """
  codelet-tools file-tool tests (apply_patch_tool_test, core_file_tools_test,
  read_image_dimension_validation_test, read_multimodal_test and the other
  file-tool suites) read the host's ~/.fspec: the RLCD defaults are enabled,
  checkMode "all" and url http://127.0.0.1:8000, so every file operation was
  scored by whatever RLCD service the developer had running, and blocked
  nondeterministically. Two rlcd001_service_supervisor scenarios left
  spawn.port at the default 8000, where a live service is found and reused.
  codelet_test_helpers::isolated_user_directory() now points FSPEC_USER_DIR at
  a per-process temp user directory whose fspec-config.json disables RLCD;
  every file-tool test calls it first. The supervisor scenarios bind a free
  spawn port with a scan limit of 1.
  BUG-481: the rule has no exemptions. bug456_providers_pass_session_id_to_tools
  names ReadTool::new and the other file tools in its source checks, so it
  isolates its user directory too (in provider_source, before any read).
  Tests: rust/tools/tests/bug347_tools_tests_isolated_from_host_rlcd.rs.
  """

  Background: User Story
    As a developer running the codelet-tools tests
    I want to have the tests use an isolated user directory and their own ports
    So that a live RLCD service or user config on the machine never changes a test's outcome

  Scenario: The isolated user directory disables RLCD
    Given a test binary that isolates its user directory
    When the RLCD configuration is loaded
    Then it comes from the isolated directory and RLCD is disabled

  Scenario: Every file-tool test isolates its user directory
    Given the codelet-tools test sources
    When the tests that run file tools are scanned
    Then each of them isolates its user directory
