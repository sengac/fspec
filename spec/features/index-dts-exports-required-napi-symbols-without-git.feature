@BUG-153
Feature: index.d.ts exports required NAPI symbols without consulting git
  """
  Architecture notes:
  - session_bindings_shape.rs: scenario_index_dts_is_byte_identical_to_pre_rpc043_baseline
  Remove git diff invocation. Keep direct symbol assertions.
  - Test must pass regardless of git branch or working-tree state
  """

  Background: User Story
    As a developer
    I want to run shape tests deterministically
    So that tests pass regardless of git branch or working-tree state

  Scenario: index.d.ts exports required NAPI symbols without consulting git
    Given the file rust/napi/index.d.ts exists on disk
    When I read the file content directly
    Then the file contains the export sessionManagerCreate
    And the file contains the export sessionSetGlobalChunkCallback
    And the file contains the interface GlobalChunkCallbackArgs
    And the file contains the interface IsolatedSessionResult
