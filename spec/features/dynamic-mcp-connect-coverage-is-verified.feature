@done
@COV-067
@tools
@coverage
@rust
Feature: dynamic-mcp-connect coverage maps the tests that carry each scenario
  """
  spec/features/dynamic-mcp-connect.feature.coverage mapped 'Route tool
  call', 'Connect via HTTP', 'Multi-server' and 'tool list changed' to
  rust/tools/src/mcp/connect_tool.rs (production code) and sampling, list,
  disconnect, HTTP-auth and cleanup to tests_01.rs ranges holding other
  scenarios' tests (stale since the MCP module split). Every scenario is now
  mapped to the test functions in rust/tools/src/mcp/tests_0{1..4}.rs that
  carry its @step comments (implementation mappings kept), and the two
  injection steps name the MCP injection channel the code uses instead of
  watcher_input_tx / supervisor_input_tx.
  The regression test reads the feature and its coverage file and checks
  every step of every scenario appears as an @step comment inside one of
  the scenario's mapped test ranges.
  Tests: rust/tools/tests/cov067_dynamic_mcp_connect_coverage.rs.
  """

  Background: User Story
    As a maintainer relying on coverage to find a scenario's tests
    I want to have every dynamic-mcp-connect scenario mapped to the test functions that carry its steps
    So that coverage points at real tests instead of production code or other scenarios' tests

  Scenario: Every dynamic-mcp-connect scenario maps tests carrying all of its steps
    Given the dynamic-mcp-connect feature and its coverage file
    When each scenario's mapped test ranges are read
    Then every step of the scenario appears as an @step comment in those ranges
