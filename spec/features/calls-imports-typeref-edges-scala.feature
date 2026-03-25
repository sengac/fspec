@KGRAPH-052
Feature: Calls/Imports/TypeRef edges — Scala

  """
  Uses ast-grep patterns and edge_helpers shared functions.
  Scala colon-based type annotations for TypeRef.
  """

  Background: User Story
    As a developer
    I want to get Imports, Calls, and TypeRef edges extracted from Scala source files
    So that dead code detection works for Scala projects via ast_dead_code

  Scenario: Extract Imports edges from Scala import statements
    Given a Scala file with `import com.myapp.UserService`
    And the target file `com/myapp/UserService.scala` exists in the project
    When the Scala extractor processes the source file
    Then an Imports edge should be emitted from the source file to the target
    And external `import scala.collection.mutable` imports should NOT produce edges

  Scenario: Extract Calls edges from Scala function calls
    Given a Scala file with function `process()` that calls `validate()`
    And `validate` is defined in the same file
    When the Scala extractor processes the source file
    Then a Calls edge should be emitted from `process` to `validate`
