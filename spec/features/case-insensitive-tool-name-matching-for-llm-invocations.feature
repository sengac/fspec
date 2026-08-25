@TOOL-021
Feature: Case-insensitive tool name matching for LLM invocations
  """
  Architecture notes:
  - ToolSet normalizes all tool names to lowercase in the HashMap key. ToolServer normalizes names for CallTool and RemoveTool messages.
  """

  Background: User Story
    As a LLM
    I want to invoke tools with inconsistent casing
    So that tool calls succeed regardless of casing

  Scenario: Lookup tool with different casing than registration
    Given a tool named "fspec" is registered in the ToolSet
    When I look up the tool as "Fspec"
    Then the tool should be found
    And I look up the tool as "FSPEC"
    Then the tool should be found

  Scenario: Call tool with mixed casing
    Given a tool named "add" is registered in the ToolSet
    When I call the tool with name "Add"
    Then the call should succeed

  Scenario: Delete tool with different casing
    Given a tool named "fspec" is registered in the ToolSet
    When I delete the tool with name "FSPEC"
    Then the tool should no longer exist in the ToolSet

  Scenario: Contains check with mixed casing
    Given a tool named "fspec" is registered in the ToolSet
    When I check if "Fspec" exists
    Then the ToolSet should contain the tool

  Scenario: Error message preserves original casing
    Given a ToolSet with no tools registered
    When I try to call a tool named "NonExistent"
    Then the error should mention "NonExistent" in the error message
