@done
@COV-066
@tools
@coverage
@rust
Feature: mcp-same-turn-tool-availability coverage maps the tests that carry each scenario
  """
  The MCP coverage mappings in dynamic-mcp-connect (fixed and guarded by
  COV-067) and mcp-same-turn-tool-availability pointed at test code without
  the scenarios' @step comments (helper_tests_02.rs / helper_tests_03.rs
  ranges covering other tests, shifted by MCP-005). Every
  mcp-same-turn-tool-availability scenario now maps the test function in
  rust/tools/src/mcp/helper_tests_03.rs / helper_tests_04.rs that carries
  its steps (implementation mappings kept).
  The regression test uses codelet_test_helpers::coverage_steps to check
  every step of every scenario appears in the scenario's mapped ranges.
  Tests: rust/tools/tests/cov066_mcp_same_turn_coverage.rs.
  """

  Background: User Story
    As a maintainer relying on coverage to find a scenario's tests
    I want to have the MCP coverage mappings verified against the tests' @step comments
    So that link-coverage can re-point them and their coverage is checked, not assumed

  Scenario: Every mcp-same-turn-tool-availability scenario maps tests carrying all of its steps
    Given the mcp-same-turn-tool-availability feature and its coverage file
    When each scenario's mapped test ranges are read
    Then every step of the scenario appears as an @step comment in those ranges
