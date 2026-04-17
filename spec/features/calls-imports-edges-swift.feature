@KGRAPH-051
Feature: Calls/Imports edges — Swift
  """
  Uses ast-grep patterns and edge_helpers. No Imports edges
  (Swift imports are module-level). Calls only.
  """

  Background: User Story
    As a developer
    I want to get Imports and Calls edges extracted from Swift source files
    So that dead code detection works for Swift projects via ast_dead_code

  Scenario: Extract Calls edges from Swift function calls
    Given a Swift file with function `execute()` that calls `configure()`
    And `configure` is defined in the same file
    When the Swift extractor processes the source file
    Then a Calls edge should be emitted from `execute` to `configure`
