@KGRAPH-049
Feature: Calls/Imports edges — Ruby
  """
  Uses edge_helpers shared functions. No TypeRef (Ruby is dynamically typed).
  Ruby uses require_relative for local imports.
  """

  Background: User Story
    As a developer
    I want to get Imports and Calls edges extracted from Ruby source files
    So that dead code detection works for Ruby projects via ast_dead_code

  Scenario: Extract Imports edges from Ruby require_relative statements
    Given a Ruby file with `require_relative 'helpers'`
    And the target file `helpers.rb` exists in the project
    When the Ruby extractor processes the source file
    Then an Imports edge should be emitted from the source file to `helpers.rb`
    And external `require 'json'` imports should NOT produce edges

  Scenario: Extract Calls edges from Ruby method calls
    Given a Ruby file with method `process` that calls `validate`
    And `validate` is defined in the same file
    When the Ruby extractor processes the source file
    Then a Calls edge should be emitted from `process` to `validate`
