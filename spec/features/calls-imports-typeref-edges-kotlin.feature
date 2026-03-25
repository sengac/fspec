@KGRAPH-050
Feature: Calls/Imports/TypeRef edges — Kotlin

  """
  Uses KindMatcher and edge_helpers shared functions.
  Kotlin colon-based type annotations for TypeRef.
  """

  Background: User Story
    As a developer
    I want to get Imports, Calls, and TypeRef edges extracted from Kotlin source files
    So that dead code detection works for Kotlin projects via ast_dead_code

  Scenario: Extract Imports edges from Kotlin import statements
    Given a Kotlin file with `import com.myapp.UserService`
    And the target file `com/myapp/UserService.kt` exists in the project
    When the Kotlin extractor processes the source file
    Then an Imports edge should be emitted from the source file to the target
    And external `import kotlin.collections.List` imports should NOT produce edges

  Scenario: Extract Calls edges from Kotlin function calls
    Given a Kotlin file with function `process()` that calls `validate()`
    And `validate` is defined in the same file
    When the Kotlin extractor processes the source file
    Then a Calls edge should be emitted from `process` to `validate`
