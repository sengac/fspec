@KGRAPH-048
Feature: Calls/Imports/TypeRef edges — C#
  """
  Uses KindMatcher for method extraction. Resolves C# using statements
  via namespace-to-path conversion.
  """

  Background: User Story
    As a developer
    I want to get Imports, Calls, and TypeRef edges extracted from C# source files
    So that dead code detection works for C# projects via ast_dead_code

  Scenario: Extract Imports edges from C# using statements
    Given a C# file with `using MyApp.Services;`
    And the target file `MyApp/Services.cs` exists in the project
    When the C# extractor processes the source file
    Then an Imports edge should be emitted from the source file to the target
    And system `using System.Collections` imports should NOT produce edges

  Scenario: Extract Calls edges from C# method calls
    Given a C# file with method `Process()` that calls `Validate()`
    And `Validate` is defined in the same file
    When the C# extractor processes the source file
    Then a Calls edge should be emitted from `Process` to `Validate`

  Scenario: Extract TypeRef edges from C# type annotations
    Given a C# file with `public Response Handle(Request req)`
    And types `Request` and `Response` are defined in the same file
    When the C# extractor processes the source file
    Then TypeRef edges should be emitted from `Handle` to `Request` and `Response`
