@KGRAPH-044
Feature: Calls/Imports/TypeRef edges — Go
  """
  Uses edge_helpers for call extraction. Go imports are string paths; local imports
  start with . or match known project files. External packages (github.com/*, stdlib)
  are filtered out.
  """

  Background: User Story
    As a developer
    I want to get Imports and Calls edges extracted from Go source files
    So that dead code detection works for Go projects via ast_dead_code

  Scenario: Extract Imports edges from Go import statements with external filtering
    Given a Go file with `import "github.com/spf13/cobra"` and `import "./internal/util"`
    When the Go extractor processes the source file
    Then an Imports edge should be emitted for the local `./internal/util` import
    And the external `github.com/spf13/cobra` import should NOT produce an edge

  Scenario: Extract Calls edges from Go function calls
    Given a Go file with function `Execute()` that calls `initConfig()`
    And `initConfig` is defined in the same file
    When the Go extractor processes the source file
    Then a Calls edge should be emitted from `Execute` to `initConfig`
