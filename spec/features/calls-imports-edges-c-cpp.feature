@KGRAPH-047
Feature: Calls/Imports edges — C and C++

  """
  C/C++ use #include directives for imports and bare function calls.
  No TypeRef edges — C/C++ type annotations are not function-signature-level types.
  """

  Background: User Story
    As a developer
    I want to get Imports and Calls edges extracted from C and C++ source files
    So that dead code detection works for C/C++ projects via ast_dead_code

  Scenario: Extract Imports edges from C include directives
    Given a C file with `#include "jv.h"`
    And the target file `jv.h` exists in the project
    When the C extractor processes the source file
    Then an Imports edge should be emitted from the source file to `jv.h`
    And system includes like `#include <stdio.h>` should NOT produce edges

  Scenario: Extract Calls edges from C function calls
    Given a C file with function `main()` that calls `jv_parse()`
    And `jv_parse` is defined in the same file
    When the C extractor processes the source file
    Then a Calls edge should be emitted from `main` to `jv_parse`

  Scenario: Extract Imports edges from C++ include directives
    Given a C++ file with `#include "utils.h"`
    And the target file `utils.h` exists in the project
    When the C++ extractor processes the source file
    Then an Imports edge should be emitted from the source file to `utils.h`
