@done
@persistence
@config-management
@safety
@BUG-640
Feature: fspec-config.json writes are atomic and tests never touch the real user config
  """
  Architecture notes:
  - codelet-common gets the one user-directory resolver (fspec_user_dir) and one read-modify-write helper for fspec-config.json (fspec_config::update_config_file): SpecLock::acquire on the file's directory from read to write, then spec_lock::replace_file_atomically. sessions' and tools' write_config_value and the user arm of write_config_with_dirs go through it.
  - The guard is cfg(feature = "test-support") in codelet_common::user_dir::fspec_user_dir; codelet-test-helpers depends on codelet-common with that feature, so it is on in every test build that uses the shared helpers and off in release builds. codelet_test_helpers::isolated_user_directory() sets FSPEC_USER_DIR once per test binary.
  - fspec-hooks.json keeps its own BUG-611 writer, which already holds the spec lock and writes atomically.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. codelet-common has the one resolver of the fspec user directory: FSPEC_USER_DIR when set and not empty, else <home>/.fspec; the copies in sessions (profile_sections::config_io), tools (rlcd::config::user_dir), fspec-core (validate_config, research, init templates, the specifying reminder) and fspec-tui (profiles_config) are deleted and call it
  #   2. A test build never resolves the real user directory: when codelet-common is built with its test-support feature (codelet-test-helpers enables it, so every crate's test build has it; cargo build -p codelet-fspec and the installed binary never do), the resolver refuses to run unless FSPEC_USER_DIR is set and not empty, failing with 'FSPEC_USER_DIR is not set in a test-support build; isolate the user directory with codelet_test_helpers::isolated_user_directory() first'. The executable's path is never inspected (product owner, 2026-11-08, replaces assumption 1)
  #   3. Every test that reaches user config (rpc410_hitl_wire_protocol_parity and the other sessions, fspec-tui and agent-loop tests that set a default model, save a profile or provider settings) isolates its user directory through codelet_test_helpers::isolated_user_directory()
  #   4. Every write of fspec-config.json, user scope and project scope, holds the config directory's file lock (codelet_common::spec_lock::SpecLock) from its read to its write and replaces the file atomically (temp file in the same directory, then rename: codelet_common::spec_lock::replace_file_atomically), through one shared read-modify-write helper in codelet-common::fspec_config
  #   5. Concurrent writers to one fspec-config.json never leave partial or concatenated JSON and never lose another writer's change: every change made by N concurrent read-modify-write updates is present afterwards
  #   6. The system blocklist (blocklist.json) and system stage permissions (stage-permissions.json) live in the user directory and resolve through the same resolver, so a test build never reads the user's real ~/.fspec/blocklist.json (BUG-656 folded in: same root fix)
  #
  # EXAMPLES:
  #   1. A test-support build with FSPEC_USER_DIR unset asks for the user config path: it fails with 'FSPEC_USER_DIR is not set in a test-support build; isolate the user directory with codelet_test_helpers::isolated_user_directory() first', and nothing under <home>/.fspec is read or written
  #   2. rpc410_hitl_wire_protocol_parity sets a default model: the model is saved in the isolated user directory's fspec-config.json and the real ~/.fspec/fspec-config.json is untouched
  #   3. 8 threads each save a different custom model to one user fspec-config.json at once: afterwards the file is valid JSON holding all 8 models
  #   4. Two processes write the same project spec/fspec-config.json at once: the file is always valid JSON and holds both changes
  #   5. Source scan: no non-test code writes fspec-config.json except through the shared helper; a std::fs::write of fspec-config.json anywhere else fails the check naming file:line
  #
  # ASSUMPTIONS:
  #   1. (Superseded by the product owner, 2026-11-08: the guard is the test-support feature, never the executable's path; see rule 2.)
  #
  # ========================================
  Background: User Story
    As a developer running fspec's test suites alongside my own fspec sessions
    I want to have every test write only an isolated config home and every fspec-config.json write be atomic and locked
    So that neither tests nor concurrent fspec processes can corrupt my real ~/.fspec/fspec-config.json

  Scenario: A test-support build that has not isolated its user directory cannot resolve the real one
    Given a build with codelet-common's test-support feature and FSPEC_USER_DIR unset
    When it resolves the fspec user directory
    Then it fails with "FSPEC_USER_DIR is not set in a test-support build; isolate the user directory with codelet_test_helpers::isolated_user_directory() first"
    And nothing under <home>/.fspec is read or written

  Scenario: Setting a default model in a session test writes only the isolated user config
    Given the rpc410_hitl_wire_protocol_parity test support with its isolated user directory
    When a fresh session sets the default model "anthropic/claude-opus-4-5"
    Then the isolated user directory's fspec-config.json holds that model
    And the resolved user directory is not <home>/.fspec

  Scenario: Concurrent writers to the user config keep every change
    Given a user fspec-config.json
    When 8 threads each save a different custom model to it at once
    Then the file is valid JSON
    And it holds all 8 custom models

  Scenario: Concurrent processes writing the project config keep both changes
    Given a project spec/fspec-config.json
    When two processes each write a different key to it at once
    Then the file is valid JSON
    And it holds both keys

  Scenario: Every fspec-config.json writer goes through the shared atomic locked helper
    Given the workspace's non-test Rust sources
    When every write of a path ending in fspec-config.json is listed
    Then each one is a call of the shared fspec-config.json update helper
    And no other code writes fspec-config.json with std::fs::write or File::create

  Scenario: A test build's system blocklist is the isolated user directory's
    Given a tools test build with FSPEC_USER_DIR set to a temp dir
    When it resolves the system blocklist path
    Then the path is <temp>/blocklist.json
    And with FSPEC_USER_DIR unset it fails with the test-support error instead of naming <home>/.fspec/blocklist.json
